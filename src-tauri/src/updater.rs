//! Automatic updater backed by `tauri-plugin-updater`.
//!
//! The update source and signing public key are configured at runtime from the
//! app config (`update_repo` / `update_pubkey`), so the app does not assume any
//! specific repository and reports clearly when the update source is missing.

use tauri::AppHandle;
use tauri_plugin_updater::{UpdaterBuilder, UpdaterExt};

/// Serialisable update info returned to the frontend.
#[derive(Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub date: String,
    pub body: String,
    pub current_version: String,
}

/// Build an updater from a GitHub `owner/repo` string. The updater queries the
/// release asset `update-<target>.json` at `releases/latest/download`.
fn build_updater(
    app: &AppHandle,
    repo: &str,
    pubkey: &str,
) -> Result<tauri_plugin_updater::Updater, String> {
    let repo = repo.trim();
    if repo.is_empty() {
        return Err("未配置更新仓库".into());
    }
    // Reject anything that is not "owner/repo".
    if !repo.contains('/') || repo.len() > 200 {
        return Err("更新仓库格式无效（应为 owner/repo）".into());
    }

    let endpoints =
        format!("https://github.com/{repo}/releases/latest/download/update-{{{{target}}}}.json");

    let mut builder: UpdaterBuilder = app
        .updater_builder()
        .endpoints(vec![url::Url::parse(&endpoints).map_err(|e| e.to_string())?])
        .map_err(|e| e.to_string())?;

    // Only set a custom pubkey when provided; otherwise fall back to the
    // (possibly empty) pubkey from tauri.conf.json.
    let pk = pubkey.trim();
    if !pk.is_empty() {
        builder = builder.pubkey(pk);
    }

    builder.build().map_err(|e| e.to_string())
}

/// Check for an update against the configured repository.
#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    repo: String,
    pubkey: String,
) -> Result<UpdateInfo, String> {
    let updater = build_updater(&app, &repo, &pubkey)?;
    match updater.check().await {
        Ok(Some(update)) => {
            let current = update.current_version.clone();
            Ok(UpdateInfo {
                available: true,
                version: update.version.clone(),
                date: update.date.map(|d| d.to_string()).unwrap_or_default(),
                body: update.body.unwrap_or_default(),
                current_version: current,
            })
        }
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

/// Download and install the latest update (shows a progress event on failure).
#[tauri::command]
pub async fn download_and_install(
    app: AppHandle,
    repo: String,
    pubkey: String,
) -> Result<(), String> {
    let updater = build_updater(&app, &repo, &pubkey)?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let update = update.ok_or_else(|| "没有可用更新".to_string())?;

    // Download and install. On failure we surface the error; the plugin will
    // fall back to its own installer when the download completes.
    update
        .download_and_install(|_chunk, _total| {}, move || {})
        .await
        .map_err(|e| format!("下载/安装更新失败: {e}"))
}
