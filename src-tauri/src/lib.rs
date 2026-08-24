//! Mem Reduct - Tauri backend.
//!
//! Exposes memory info + cleanup commands to the React frontend and manages
//! the tray icon, automatic cleanup and global hotkeys.

pub mod autostart;
pub mod cmdline;
pub mod config;
pub mod elevation;
pub mod hotkey;
pub mod memory;
pub mod ntapi;
pub mod tray;
pub mod trayicon;
pub mod updater;

use config::Config;
use memory::{CleanResult, MemoryInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
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
pub struct AppState {
    pub config: Mutex<Config>,
    tray: Mutex<Option<TrayIcon>>,
    hotkey_stop: Mutex<Option<Arc<AtomicBool>>>,
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
    let is_manual = source.as_deref() == Some("manual");
    let is_autoclean = matches!(
        source.as_deref(),
        Some("auto") | Some("hotkey") | Some("cmdline")
    );
    let allow_standby = cfg.allow_standby_list_cleanup;

    // Original Mem Reduct behaviour: a manual cleanup while un-elevated
    // relaunches the WHOLE app through the UAC `runas` verb and exits this
    // instance. The elevated instance takes over, so every later cleanup
    // (manual or automatic) runs elevated with no further UAC prompts.
    if is_manual && !elevation::is_elevated() && elevation::relaunch_self_as_admin() {
        // Successful relaunch: the elevated instance takes over.
        std::process::exit(0);
    }
    // User cancelled the UAC prompt → fall through to a limited attempt
    // below (mirrors the original's "no privileges" path).

    let result = memory::clean_memory(mask, allow_standby, is_autoclean);

    // Log cleanup result if enabled.
    if cfg.log_clean_results {
        config::log_cleanup(&format!(
            "freed {} bytes, regions {:?}",
            result.freed_bytes, result.regions
        ));
    }

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
fn save_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<(), String> {
    let _ = config::save(&config).map_err(|e| e.to_string());
    *state.config.lock().unwrap() = config;
    // Live-applying config: re-register the global hotkey.
    register_hotkey(&app);
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

/// Current app version (from package info).
#[tauri::command]
fn get_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Open an external URL in the default browser.
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = url.encode_utf16().chain(core::iter::once(0)).collect();
    static OPEN: [u16; 5] = [0x6f, 0x70, 0x65, 0x6e, 0x00]; // "open\0"

    unsafe {
        let r = ShellExecuteW(
            None,
            PCWSTR(OPEN.as_ptr()),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        if r.0 as isize > 32 {
            Ok(())
        } else {
            Err("打开链接失败".into())
        }
    }
}

/// Show a native system notification via tauri-plugin-notification.
///
/// The frontend independently renders its own in-app toast, so this command
/// must NOT emit "app-toast" again — otherwise every message shows twice.
#[tauri::command]
fn notify(app: AppHandle, title: String, body: String, system: Option<bool>) -> Result<(), String> {
    // Native system notification (Windows toast / macOS / Linux).
    if system.unwrap_or(true) {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title(&title)
            .body(&body)
            .show();
    }

    Ok(())
}

/// Query whether the silent elevated autostart task is installed.
#[tauri::command]
fn get_autostart() -> bool {
    autostart::is_enabled()
}

/// Enable or disable the silent elevated autostart task.
///
/// Enabling requires elevation (to create a highest-privilege logon task). If
/// the app is not currently elevated, a single UAC prompt is shown once to
/// install the task; after that, every logon starts the app elevated & silent.
#[tauri::command]
fn set_autostart(enabled: bool) -> Result<String, String> {
    if enabled {
        if elevation::is_elevated() {
            autostart::install()?;
            Ok("installed".into())
        } else {
            // One-shot elevation to create the task (the only UAC prompt).
            if elevation::relaunch_with_args("-ensure-autostart") {
                Ok("elevation_requested".into())
            } else {
                Err("无法请求管理员权限(UAC 被取消)".into())
            }
        }
    } else {
        if elevation::is_elevated() {
            autostart::uninstall()?;
            Ok("removed".into())
        } else {
            if elevation::relaunch_with_args("-disable-autostart") {
                Ok("elevation_requested".into())
            } else {
                Err("无法请求管理员权限(UAC 被取消)".into())
            }
        }
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
    app.state::<AppState>()
        .config
        .lock()
        .unwrap()
        .allow_standby_list_cleanup
}

/// Clean memory triggered from the tray (double-click / menu).
pub fn clean_from_tray(app: &AppHandle) {
    let mask = app.state::<AppState>().config.lock().unwrap().reduct_mask;
    let allow_standby = cfg_allow_standby(app);
    let _ = memory::clean_memory(mask, allow_standby, false);
    let _ = app.emit("memory-update", memory::get_memory_info());
}

/// Execute a tray action: 0 = show window, 1 = clean memory.
pub fn run_tray_action(app: &AppHandle, action: u32) {
    match action {
        1 => clean_from_tray(app),
        _ => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
    }
}

/// (Re)register the global clean hotkey from the current config.
fn register_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    let cfg = state.config.lock().unwrap().clone();

    // Stop any existing hotkey thread.
    {
        let mut guard = state.hotkey_stop.lock().unwrap();
        if let Some(stop) = guard.take() {
            stop.store(true, Ordering::Relaxed);
        }
    }

    if cfg.hotkey_clean_enable {
        let (mods, vk) = hotkey::decode(cfg.hotkey_clean);
        let stop = Arc::new(AtomicBool::new(false));
        *state.hotkey_stop.lock().unwrap() = Some(stop.clone());
        let app = app.clone();
        hotkey::run_hotkey_loop(1, mods, vk, stop, move || {
            let h = app.clone();
            let mask = h.state::<AppState>().config.lock().unwrap().reduct_mask;
            let _ = memory::clean_memory(mask, cfg_allow_standby(&h), false);
        });
    }
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
                    let pct = info.physical_memory.percent;
                    let _ = tray.set_title(Some(format!("{pct}%")));
                    let _ = tray.set_tooltip(Some(format!(
                        "Mem Reduct\n内存占用: {pct}%\n已用: {:.1} GB / 共: {:.1} GB",
                        info.physical_memory.used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                        info.physical_memory.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                    )));

                    // Render the percent into the tray icon with the configured
                    // colours (background switches to warning/danger on threshold).
                    let danger = cfg.tray_level_danger;
                    let warning = cfg.tray_level_warning;
                    let bg = if pct >= danger {
                        trayicon::unpack_color(cfg.tray_color_danger)
                    } else if pct >= warning {
                        if cfg.tray_change_bg {
                            trayicon::unpack_color(cfg.tray_color_warning)
                        } else {
                            trayicon::unpack_color(cfg.tray_color_bg)
                        }
                    } else {
                        trayicon::unpack_color(cfg.tray_color_bg)
                    };
                    let fg = if cfg.tray_change_bg {
                        trayicon::unpack_color(cfg.tray_color_text)
                    } else if pct >= danger {
                        trayicon::unpack_color(cfg.tray_color_danger)
                    } else if pct >= warning {
                        trayicon::unpack_color(cfg.tray_color_warning)
                    } else {
                        trayicon::unpack_color(cfg.tray_color_text)
                    };

                    let style = trayicon::TrayIconStyle {
                        bg,
                        fg,
                        transparent: cfg.tray_use_transparency,
                        border: cfg.tray_show_border,
                        round: cfg.tray_round_corners,
                    };
                    let rgba = trayicon::render(pct, &style);
                    let icon = tauri::image::Image::new_owned(rgba, 32, 32);
                    let _ = tray.set_icon(Some(icon));
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
                let _ = memory::clean_memory(cfg.reduct_mask, cfg.allow_standby_list_cleanup, true);
                let _ = app.emit("autoclean-done", ());
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            config: Mutex::new(config::load()),
            tray: Mutex::new(None),
            hotkey_stop: Mutex::new(None),
        })
        .setup(|app| {
            // Create the tray icon and store it in state for background updates.
            if let Ok(tray) = tray::create_tray(app.handle()) {
                *app.state::<AppState>().tray.lock().unwrap() = Some(tray);
            }

            let handle = app.handle().clone();

            // Silent autostart: when launched by the logon task (`-startup`),
            // start minimized to the tray instead of opening the window.
            if autostart::is_startup_launch() {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.hide();
                }
            }

            // Handle `-clean` / `-clean:full` command-line action.
            match cmdline::parse() {
                cmdline::CommandLineAction::CleanDefault => {
                    let state = handle.state::<AppState>();
                    let mask = state.config.lock().unwrap().reduct_mask;
                    if !elevation::is_elevated() && elevation::relaunch_self_as_admin() {
                        std::process::exit(0);
                    }
                    let _ = memory::clean_memory(mask, cfg_allow_standby(&handle), false);
                }
                cmdline::CommandLineAction::CleanFull => {
                    if !elevation::is_elevated() && elevation::relaunch_self_as_admin() {
                        std::process::exit(0);
                    }
                    let _ =
                        memory::clean_memory(memory::mask::ALL, cfg_allow_standby(&handle), false);
                }
                // `-clean-once` is handled in main() before the UI starts; this
                // arm is unreachable here.
                cmdline::CommandLineAction::CleanOnce(_) => {}
                cmdline::CommandLineAction::None => {}
            }

            // Start the global hotkey (default Ctrl+F1) if enabled.
            register_hotkey(&handle);

            // Spawn the periodic background loop.
            spawn_background(handle.clone());

            Ok(())
        })
        // Keep the app running in the tray: closing the window hides it instead
        // of quitting, so the tray icon stays (restorable by clicking it).
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_memory_info,
            is_elevated,
            clean_memory,
            get_config,
            save_config,
            get_config_location,
            get_os_info,
            notify,
            get_autostart,
            set_autostart,
            get_version,
            open_external,
            updater::check_for_update,
            updater::download_and_install
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
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
