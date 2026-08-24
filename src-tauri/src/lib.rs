//! Mem Reduct - Tauri backend.
//!
//! Exposes memory info + cleanup commands to the React frontend and manages
//! the tray icon, automatic cleanup and global hotkeys.

pub mod cmdline;
pub mod config;
pub mod elevation;
pub mod hotkey;
pub mod memory;
pub mod ntapi;
pub mod tray;

use config::Config;
use memory::{CleanResult, MemoryInfo};
use std::sync::Mutex;
use std::time::Duration;
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter, Manager, State};

/// Serializable OS info.
#[derive(Debug, serde::Serialize)]
pub struct OsInfo {
    pub major: u32,
    pub minor: u32,
    pub is_win8_1: bool,
    pub is_win10: bool,
}

/// App state kept for the lifetime of the process.
struct AppState {
    config: Mutex<Config>,
    tray: Mutex<Option<TrayIcon>>,
}

#[tauri::command]
fn get_memory_info() -> MemoryInfo {
    memory::get_memory_info()
}

/// Whether the app is running elevated (affects cleanup effectiveness).
#[tauri::command]
fn is_elevated() -> bool {
    elevation::is_elevated()
}

#[tauri::command]
fn clean_memory(
    state: State<'_, AppState>,
    mask: Option<u32>,
    source: Option<String>,
) -> CleanResult {
    let cfg = state.config.lock().unwrap().clone();
    let mask = mask.unwrap_or(cfg.reduct_mask);
    let is_autoclean = matches!(
        source.as_deref(),
        Some("auto") | Some("hotkey") | Some("cmdline")
    );
    let allow_standby = cfg.allow_standby_list_cleanup;

    let result = memory::clean_memory(mask, allow_standby, is_autoclean);

    // Update statistic timestamp.
    let mut guard = state.config.lock().unwrap();
    guard.statistic_last_reduct = unix_now();
    if config::save(&guard).is_err() {
        // Non-fatal: config save failure shouldn't crash cleaning.
    }
    drop(guard);

    result
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn save_config(state: State<'_, AppState>, config: Config) -> Result<(), String> {
    let _ = config::save(&config).map_err(|e| e.to_string());
    *state.config.lock().unwrap() = config;
    Ok(())
}

#[tauri::command]
fn get_config_location() -> String {
    match config::config_location() {
        config::ConfigLocation::Portable => "portable".into(),
        config::ConfigLocation::AppData => "appdata".into(),
    }
}

#[tauri::command]
fn get_os_info() -> OsInfo {
    let (major, minor) = memory::os_version();
    OsInfo {
        major,
        minor,
        is_win8_1: memory::is_win8_1_plus(),
        is_win10: memory::is_win10_plus(),
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Helper: read `allow_standby_list_cleanup` from the current config state.
fn cfg_allow_standby(app: &AppHandle) -> bool {
    app.state::<AppState>().config.lock().unwrap().allow_standby_list_cleanup
}

/// Decide whether auto-clean should run based on the current usage percent,
/// the on/off flags, the threshold, interval and the last-clean timestamp.
///
/// Mirrors the original: threshold-based cleanup is gated by a 30s cooldown,
/// interval-based cleanup likewise, and both are skipped when disabled.
fn should_autoclean(
    percent: u32,
    enable_by_threshold: bool,
    threshold: u32,
    enable_by_interval: bool,
    interval_minutes: u32,
    last_reduct: i64,
) -> bool {
    let now = unix_now();
    let elapsed = now.saturating_sub(last_reduct);
    // 30s cooldown shared by both modes (AUTOREDUCT_COOLDOWN).
    if elapsed < 30 {
        return false;
    }
    if enable_by_threshold && percent >= threshold {
        return true;
    }
    if enable_by_interval && elapsed >= interval_minutes as i64 * 60 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoclean_threshold_logic() {
        let now = unix_now();
        // Threshold hit and cooldown passed -> clean.
        assert!(should_autoclean(95, true, 90, false, 30, now - 60));
        // Below threshold -> no clean.
        assert!(!should_autoclean(50, true, 90, false, 30, now - 60));
        // Disabled -> no clean.
        assert!(!should_autoclean(95, false, 90, false, 30, now - 60));
        // Cooldown not passed -> no clean.
        assert!(!should_autoclean(95, true, 90, false, 30, now));
        // Interval mode.
        assert!(should_autoclean(10, false, 90, true, 30, now - 30 * 60));
        assert!(!should_autoclean(10, false, 90, true, 30, now - 60));
    }
}

/// Periodic background loop: auto-clean + refresh tray + emit info to UI.
fn spawn_background(app: AppHandle) {
    std::thread::spawn(move || {
        // The original uses a 1000ms timer; we use it for tray + data.
        loop {
            std::thread::sleep(Duration::from_millis(1000));

            let state = app.state::<AppState>();
            let cfg = state.config.lock().unwrap().clone();

            // Refresh tray every 1s.
            {
                let state = app.state::<AppState>();
                let guard = state.tray.lock().unwrap();
                if let Some(tray) = guard.as_ref() {
                    let info = memory::get_memory_info();
                    let _ = tray.set_title(Some(format!("{}%", info.physical_memory.percent)));
                }
            }

            // Emit fresh memory info to the frontend.
            let info = memory::get_memory_info();
            let _ = app.emit("memory-update", info);

            // Auto-clean by threshold or interval (with a shared 30s cooldown).
            let now = unix_now();
            if should_autoclean(
                info.physical_memory.percent,
                cfg.autoreduct_enable,
                cfg.autoreduct_value,
                cfg.autoreduct_interval_enable,
                cfg.autoreduct_interval_value,
                cfg.statistic_last_reduct,
            ) {
                let guard = app.state::<AppState>();
                let mut c = guard.config.lock().unwrap();
                c.statistic_last_reduct = now;
                drop(c);
                let _ = memory::clean_memory(
                    cfg.reduct_mask,
                    cfg.allow_standby_list_cleanup,
                    true,
                );
                let _ = app.emit("autoclean-done", ());
            }

        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(config::load()),
            tray: Mutex::new(None),
        })
        .setup(|app| {
            // Create the tray icon and store it in state for background updates.
            if let Ok(tray) = tray::create_tray(&app.handle()) {
                *app.state::<AppState>().tray.lock().unwrap() = Some(tray);
            }

            let handle = app.handle().clone();

            // Handle `-clean` / `-clean:full` command-line action.
            match cmdline::parse() {
                cmdline::CommandLineAction::CleanDefault => {
                    let state = handle.state::<AppState>();
                    let mask = state.config.lock().unwrap().reduct_mask;
                    let _ = memory::clean_memory(mask, cfg_allow_standby(&handle), false);
                }
                cmdline::CommandLineAction::CleanFull => {
                    let _ = memory::clean_memory(memory::mask::ALL, cfg_allow_standby(&handle), false);
                }
                cmdline::CommandLineAction::None => {}
            }

            // Start the global hotkey (Ctrl+F1 default) if enabled.
            {
                let state = handle.state::<AppState>();
                let cfg = state.config.lock().unwrap().clone();
                if cfg.hotkey_clean_enable {
                    // MOD_CONTROL (2) | DEFAULT F1 (0x71)
                    let hotkey_handle = handle.clone();
                    hotkey::run_hotkey_loop(1, 0x0002, cfg.hotkey_clean, move || {
                        let h = hotkey_handle.clone();
                        let mask = h.state::<AppState>().config.lock().unwrap().reduct_mask;
                        let _ = memory::clean_memory(mask, cfg_allow_standby(&h), false);
                    });
                }
            }

            // Spawn the periodic background loop.
            spawn_background(handle.clone());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_memory_info,
            is_elevated,
            clean_memory,
            get_config,
            save_config,
            get_config_location,
            get_os_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
