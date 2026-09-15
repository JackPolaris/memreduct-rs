//! Automatic updater backed by `tauri-plugin-updater`.
//!
//! The update source is hardcoded to the official GitHub repository
//! (`JackPolaris/memreduct-rs`); the UI only exposes a single "Check for
//! updates" button and the current version — no repo/key configuration.

use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// Official release repository (hardcoded, owner/repo).
const UPDATE_REPO: &str = "JackPolaris/memreduct-rs";

/// Release target triple of the running binary.
///
/// `tauri.conf.json` only substitutes `{{target}}` for endpoints declared at
/// build time; these endpoints are assembled at runtime, so the architecture has
/// to be mapped explicitly. A hardcoded `x86_64` made the updater a 404 for
/// anyone running the `aarch64` (Windows on ARM) or 32-bit build.
fn target_triple() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64-pc-windows-msvc",
        "x86" => "i686-pc-windows-msvc",
        _ => "x86_64-pc-windows-msvc",
    }
}

/// Full URL of the update manifest for this architecture.
fn manifest_url() -> String {
    format!(
        "https://github.com/{UPDATE_REPO}/releases/latest/download/update-{}.json",
        target_triple()
    )
}

/// Serialisable update info returned to the frontend.
#[derive(Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub date: String,
    pub body: String,
    pub current_version: String,
}

/// Build the updater against the hardcoded official repository.
///
/// The signing public key comes from `tauri.conf.json`
/// (`plugins.updater.pubkey`), which `updater_builder()` picks up — it is not
/// duplicated in the user config any more.
fn build_updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    let endpoint = manifest_url();

    app.updater_builder()
        .endpoints(vec![url::Url::parse(&endpoint).map_err(|e| e.to_string())?])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())
}

/// Check for an update against the official repository.
#[tauri::command]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, String> {
    let updater = build_updater(&app)?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateInfo {
            available: true,
            version: update.version.clone(),
            date: update.date.map(|d| d.to_string()).unwrap_or_default(),
            body: update.body.unwrap_or_default(),
            current_version: update.current_version.clone(),
        }),
        Ok(None) => {
            let current = app.package_info().version.to_string();
            Ok(UpdateInfo {
                available: false,
                version: String::new(),
                date: String::new(),
                body: String::new(),
                current_version: current,
            })
        }
        Err(e) => Err(format!("检查更新失败: {e}")),
    }
}

/// Download the latest update and install it in the background, then exit the
/// app so the installer can replace the binary / relaunch it.
#[tauri::command]
pub async fn download_and_install(app: AppHandle) -> Result<(), String> {
    let updater = build_updater(&app)?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let update = update.ok_or_else(|| "没有可用更新".to_string())?;

    // Download with live progress emitted to the frontend. The `on_download_finish`
    // callback must stay empty: exiting here would kill the process BEFORE the
    // installer is launched. `Update::install` itself ShellExecutes the
    // installer and then calls `std::process::exit(0)` on its own.
    let progress_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                let _ = progress_app.emit(
                    "update-progress",
                    serde_json::json!({ "chunk": chunk, "total": total }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| format!("下载/安装更新失败: {e}"))
}

#[cfg(test)]
mod tests {
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
