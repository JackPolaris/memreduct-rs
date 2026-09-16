//! Automatic updater backed by `tauri-plugin-updater`.
//!
//! The update source is hardcoded to the official GitHub repository
//! (`JackPolaris/memreduct-rs`); the UI exposes a single "check for updates"
//! button and the current version — no repo/key configuration.

use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// Official release repository (hardcoded, owner/repo).
const UPDATE_REPO: &str = "JackPolaris/memreduct-rs";

/// Deadline for a manifest request.
///
/// The plugin's builder leaves this unset and its `Config` has no timeout field,
/// so without an explicit value a check against an unreachable endpoint (blocked
/// proxy, captive portal, dead network, `github.com` filtered out) hangs until
/// the OS gives up — minutes — while the UI shows nothing at all.
const CHECK_TIMEOUT: Duration = Duration::from_secs(30);

/// Deadline for the installer download.
///
/// `Updater::check` hard-codes `timeout: None` on the `Update` it returns, so the
/// download request is unlimited by default as well; we patch it before
/// downloading. Generous, because it covers the whole body on a slow link.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);

/// Marker file written just before the installer runs, consumed by the next
/// start of the app.
///
/// After a silent install the app is relaunched by the *installer*, a process
/// the user never interacted with. Windows' foreground rules only let a process
/// take the foreground when it — or the process that started it — already owns
/// it, so that relaunch comes up behind whatever the user is looking at; with
/// "start minimized to tray" enabled it does not even show. The start therefore
/// has to be treated exactly like the elevation hand-over (`-takeover`), and a
/// file is the only channel that survives the process boundary:
/// `AllowSetForegroundWindow` delegates the right to a *single* process for a
/// *single* call, and the process that ends up calling `SetForegroundWindow` is
/// a grandchild of ours, which that delegation never reaches.
const RELAUNCH_MARKER: &str = "relaunched-after-update.flag";

fn marker_path() -> std::path::PathBuf {
    crate::config::data_dir().join(RELAUNCH_MARKER)
}

/// Record that the next start of this app is the installer's relaunch.
fn mark_at(path: &std::path::Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // Best effort: failing here costs the foreground once, never the install.
    let _ = std::fs::write(path, b"1");
}

/// Read and clear the marker. It must be consumed *once* — leaving it behind
/// would make every later cold start hijack the foreground.
fn take_at(path: &std::path::Path) -> bool {
    let pending = path.exists();
    if pending {
        let _ = std::fs::remove_file(path);
    }
    pending
}

fn mark_relaunch_pending() {
    mark_at(&marker_path());
}

fn clear_relaunch_pending() {
    let _ = std::fs::remove_file(marker_path());
}

/// Consume the marker: `true` when this start is the installer's relaunch.
pub fn take_relaunch_pending() -> bool {
    take_at(&marker_path())
}

/// Map a Rust architecture name to its release target triple.
fn triple_for(arch: &str) -> &'static str {
    match arch {
        "aarch64" => "aarch64-pc-windows-msvc",
        "x86" => "i686-pc-windows-msvc",
        _ => "x86_64-pc-windows-msvc",
    }
}

/// Release target triple of the running binary.
///
/// `tauri.conf.json` only substitutes `{{target}}` for endpoints declared at
/// build time; this endpoint is assembled at runtime, so the architecture has to
/// be mapped explicitly. A hardcoded `x86_64` made the updater a 404 for anyone
/// running the `aarch64` (Windows on ARM) or 32-bit build.
fn target_triple() -> &'static str {
    triple_for(std::env::consts::ARCH)
}

/// Full URL of the update manifest for this architecture.
pub fn manifest_url() -> String {
    format!(
        "https://github.com/{UPDATE_REPO}/releases/latest/download/update-{}.json",
        target_triple()
    )
}

/// Human-facing "latest release" page.
///
/// Separate from [`manifest_url`] on purpose: the manifest is machine JSON, so
/// linking the UI at it is useless to a user. A *failed* check still reports the
/// endpoint it tried, which is where a filtered proxy shows up.
pub fn release_page_url() -> String {
    format!("https://github.com/{UPDATE_REPO}/releases/latest")
}

/// Result of a check, and the facts needed to diagnose a failed one.
#[derive(Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub date: String,
    pub body: String,
    pub current_version: String,
}

/// Static updater facts, without any network I/O.
#[derive(Debug, serde::Serialize)]
pub struct UpdaterInfo {
    pub current_version: String,
    /// HTML release page — the only thing the UI links to.
    pub release_page: String,
}

/// Report the current version and the release page, without touching the network.
#[tauri::command]
pub fn get_updater_info(app: AppHandle) -> UpdaterInfo {
    UpdaterInfo {
        current_version: app.package_info().version.to_string(),
        release_page: release_page_url(),
    }
}

/// Build the updater against the hardcoded official repository.
///
/// The signing public key comes from `tauri.conf.json`
/// (`plugins.updater.pubkey`), which `updater_builder()` picks up — it is not
/// duplicated in the user config any more.
///
/// `no_proxy` bypasses the system/environment proxy; see [`check_update`].
fn build_updater(app: &AppHandle, no_proxy: bool) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint = manifest_url();
    let url = url::Url::parse(&endpoint).map_err(|e| format!("更新地址无效({endpoint}):{e}"))?;

    let builder = app.updater_builder().timeout(CHECK_TIMEOUT);
    // `.no_proxy()` is only meaningful on the fallback attempt; the normal path
    // must keep honouring the machine's proxy configuration.
    let builder = if no_proxy {
        builder.no_proxy()
    } else {
        builder
    };

    builder
        .endpoints(vec![url])
        .map_err(|e| format!("更新地址被拒绝:{e}"))?
        .build()
        .map_err(|e| format!("初始化更新器失败:{e}"))
}

/// What a check produced, plus how it got there.
struct Checked {
    update: Option<tauri_plugin_updater::Update>,
    /// True when the request only succeeded after bypassing the proxy, so the
    /// download can stay on the route that is known to work.
    via_no_proxy: bool,
}

/// Check for an update, retrying once with the proxy bypassed.
///
/// Why the fallback exists: a proxy that filters `github.com` (or cannot handle
/// the redirect to `objects.githubusercontent.com`) is answered with a non-2xx
/// status, and the plugin turns a non-2xx response into `ReleaseNotFound` — *not*
/// into a network error (it only records `last_error` for real transport
/// failures). So the user sees "no release found" while the actual cause is the
/// proxy. Since the machine usually *can* reach GitHub directly, one retry with
/// the proxy disabled turns "updates never work" into "updates work, a little
/// slower".
async fn check_update(app: &AppHandle) -> Result<Checked, String> {
    let endpoint = manifest_url();
    let current = app.package_info().version.to_string();

    let via_proxy = build_updater(app, false)?;
    match via_proxy.check().await {
        Ok(update) => Ok(Checked {
            update,
            via_no_proxy: false,
        }),
        Err(first) => {
            let first = first.to_string();
            let direct = build_updater(app, true)?;
            match direct.check().await {
                Ok(update) => {
                    eprintln!("[updater] 经代理检查失败,已忽略代理重试成功:{first}");
                    Ok(Checked {
                        update,
                        via_no_proxy: true,
                    })
                }
                Err(second) => Err(check_failed(
                    &endpoint,
                    &current,
                    &format!("{first}(忽略代理重试亦失败:{second})"),
                )),
            }
        }
    }
}

/// Human-readable failure message, with the context needed to act on it.
fn check_failed(endpoint: &str, current: &str, reason: &str) -> String {
    format!(
        "检查更新失败(当前 v{current}):{reason}\n\
         更新地址:{endpoint}\n\
         若持续失败,通常是网络或代理无法访问 github.com ——\n\
         被代理拦截时,插件会把它报告成「未找到发布」而不是网络错误。"
    )
}

/// Check for an update against the official repository.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let current = app.package_info().version.to_string();

    match check_update(&app).await {
        Ok(Checked {
            update: Some(update),
            ..
        }) => Ok(UpdateInfo {
            available: true,
            version: update.version.clone(),
            date: update.date.map(|d| d.to_string()).unwrap_or_default(),
            body: update.body.clone().unwrap_or_default(),
            current_version: update.current_version.clone(),
        }),
        Ok(Checked { update: None, .. }) => Ok(UpdateInfo {
            available: false,
            version: String::new(),
            date: String::new(),
            body: String::new(),
            current_version: current,
        }),
        Err(message) => {
            // Release builds have no console; keep it for debug/CI runs.
            eprintln!("[updater] {message}");
            Err(message)
        }
    }
}

/// Download the latest update and install it, then let the plugin exit the app
/// so the installer can replace the binary.
#[tauri::command]
pub async fn download_and_install(app: AppHandle) -> Result<(), String> {
    // Re-check so the download always uses the URL currently announced. The
    // failure text says so explicitly, otherwise it looks like the install
    // itself failed.
    let checked = check_update(&app)
        .await
        .map_err(|e| format!("下载前重新检查更新失败\n{e}"))?;

    let mut update = checked.update.ok_or_else(|| "没有可用更新".to_string())?;

    // Stay on whichever route just worked, and give the download a deadline:
    // the plugin builds this `Update` with `timeout: None`, so it would
    // otherwise never time out.
    update.no_proxy = checked.via_no_proxy;
    update.timeout = Some(DOWNLOAD_TIMEOUT);

    // Live progress to the frontend. `on_download_finish` must stay empty:
    // exiting there would kill the process BEFORE the installer is launched.
    // `Update::install` launches the installer and then calls
    // `std::process::exit(0)` on its own.
    let progress_app = app.clone();

    // Set before the hand-off: once the installer takes over we no longer get
    // to run any code. Cleared again if the install fails and we keep running,
    // so a later cold start is not mistaken for a relaunch.
    mark_relaunch_pending();
    let installed = update
        .download_and_install(
            move |chunk, total| {
                let _ = progress_app.emit(
                    "update-progress",
                    serde_json::json!({ "chunk": chunk, "total": total }),
                );
            },
            || {},
        )
        .await;
    if installed.is_err() {
        clear_relaunch_pending();
    }
    installed.map_err(|e| format!("下载或安装更新失败:{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaunch_marker_is_consumed_exactly_once() {
        // Temp dir, never the real data dir: the test must not touch the user's
        // config folder.
        let dir = std::env::temp_dir().join("memreduct-marker-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(RELAUNCH_MARKER);
        let _ = std::fs::remove_file(&path);

        assert!(!take_at(&path), "没有标记时不该报告为更新后重启");
        mark_at(&path);
        assert!(take_at(&path), "标记应被读到");
        assert!(
            !take_at(&path),
            "标记必须只消费一次,否则之后每次冷启动都会抢焦点"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn target_triple_maps_each_architecture() {
        assert_eq!(triple_for("x86_64"), "x86_64-pc-windows-msvc");
        assert_eq!(triple_for("aarch64"), "aarch64-pc-windows-msvc");
        // 32-bit builds must not fall back to the x86_64 manifest.
        assert_eq!(triple_for("x86"), "i686-pc-windows-msvc");
        assert_eq!(target_triple(), triple_for(std::env::consts::ARCH));
    }

    #[test]
    fn endpoint_matches_the_running_architecture() {
        let url = manifest_url();
        assert!(
            url.starts_with(
                "https://github.com/JackPolaris/memreduct-rs/releases/latest/download/"
            ),
            "unexpected endpoint: {url}"
        );
        // The manifest is fetched by the app itself, so the filename must carry
        // this build's target triple.
        assert!(
            url.ends_with(&format!("update-{}.json", target_triple())),
            "endpoint must carry the target triple: {url}"
        );
    }

    #[test]
    fn failure_message_carries_context() {
        let message = check_failed("https://example.test/update.json", "3.5.13", "timed out");
        assert!(message.contains("3.5.13"));
        assert!(message.contains("https://example.test/update.json"));
        assert!(message.contains("timed out"));
        assert!(message.contains("github.com"));
    }

    #[test]
    fn timeouts_are_bounded() {
        // A missing timeout is the bug this guards: the request would hang until
        // the OS gave up.
        assert!(CHECK_TIMEOUT.as_secs() > 0 && CHECK_TIMEOUT.as_secs() <= 60);
        assert!(DOWNLOAD_TIMEOUT >= CHECK_TIMEOUT);
    }

    #[test]
    fn remote_release_parses_manifest() {
        // Use the REAL manifest content downloaded from GitHub (verbatim).
        let raw = r#"{
  "version": "3.5.4",
  "notes": "Mem Reduct 3.5.4\n\n- 测试更新功能",
  "pub_date": "2026-08-24T17:50:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVUVmFDZWQvTXo4TXZHVklvR2pyS216ZFRqUi85QVlkWkZhbEVsMjJ4Wm40TVFzTzRKVElOVERQTFNkTWVpc2QwWjJYR2diRnJSZmVuSHhuR04wTFZwYm93aUVQT3ZIbkFZPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzg3NTkzODI5CWZpbGU6TWVtIFJlZHVjdF8zLjUuNF94NjQtc2V0dXAuZXhlCmd4c1NtQXhSTVFwTnFaWkJneTBER2l1ZlZjVVcvcjlmMVRkL0VLR1lHOE5ZNnI2LzJEcVZUVDA3ZGNOYWc3ZnJBZ0lPVFlHcDBNc1FVNW12SGRyZkN3PT0K",
      "url": "https://github.com/JackPolaris/memreduct-rs/releases/download/v3.5.4/Mem.Reduct_3.5.4_x64-setup.exe"
    }
  }
}"#;
        let v: serde_json::Value = serde_json::from_str(raw).expect("json should be valid");
        match serde_json::from_value::<tauri_plugin_updater::RemoteRelease>(v) {
            Ok(release) => {
                println!("parsed OK, version={}", release.version);
            }
            Err(e) => {
                panic!("RemoteRelease parse FAILED: {e:#}");
            }
        }
    }
}
