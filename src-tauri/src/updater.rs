//! Automatic updater backed by `tauri-plugin-updater`.
//!
//! The update source is hardcoded to the official GitHub repository
//! (`JackPolaris/memreduct-rs`); the UI only exposes a single "Check for
//! updates" button and the current version — no repo/key configuration.

use tauri::{AppHandle, Manager};
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

    // Download and run the installer. The plugin handles the download and the
    // Windows installer (passive). Once the installer finishes, this process
    // exits so the new version can take over.
    update
        .download_and_install(
            |_chunk, _total| {},
            || {
                // Installer finished: quit the app to let the new version launch.
                app.exit(0);
            },
        )
        .await
        .map_err(|e| format!("下载/安装更新失败: {e}"))
}
