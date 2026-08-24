//! System tray icon showing the current memory percentage.
//!
//! On Windows the tray shows a percent value as its "title" (via
//! `NIF_TITLE`), which is exactly how Mem Reduct shows the number next to the
//! icon. The live percent is refreshed by the background loop in `lib.rs`.

use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::Runtime;

/// Create the tray icon attached to the app.
pub fn create_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    TrayIconBuilder::new()
        .tooltip("Mem Reduct")
        .icon(app.default_window_icon().unwrap().clone())
        .build(app)
}
