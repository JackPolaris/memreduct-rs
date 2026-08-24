//! Automatic updater backed by `tauri-plugin-updater`.
//!
//! The update source is hardcoded to the official GitHub repository
//! (`JackPolaris/memreduct-rs`); the UI only exposes a single "Check for
//! updates" button and the current version — no repo/key configuration.

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{UpdaterBuilder, UpdaterExt};

use crate::AppState;

/// Official release repository (hardcoded, owner/repo).
const UPDATE_REPO: &str = "JackPolaris/memreduct-rs";

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
fn build_updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    // Note: `{{target}}` is substituted at build time ONLY for endpoints in
    // tauri.conf.json; since these endpoints are set at runtime we use the
    // concrete Windows target directly.
    let endpoints = format!(
        "https://github.com/{UPDATE_REPO}/releases/latest/download/update-x86_64-pc-windows-msvc.json"
    );

    let pubkey = app
        .state::<AppState>()
        .config
        .lock()
        .map(|c| c.update_pubkey.clone())
        .unwrap_or_default();

    let mut builder: UpdaterBuilder = app
        .updater_builder()
        .endpoints(vec![url::Url::parse(&endpoints).map_err(|e| e.to_string())?])
        .map_err(|e| e.to_string())?;

    if !pubkey.trim().is_empty() {
        builder = builder.pubkey(pubkey.trim().to_string());
    }

    builder.build().map_err(|e| e.to_string())
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
