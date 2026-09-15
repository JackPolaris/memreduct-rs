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
pub mod single_instance;
pub mod tray;
pub mod trayicon;
pub mod updater;

use config::Config;
use memory::{CleanResult, MemoryInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
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
    hotkey: Mutex<Option<HotkeyHandle>>,
}

/// A running global-hotkey listener: its stop flag plus the thread handle, so
/// re-registration can *wait* for the previous `RegisterHotKey` to be released.
struct HotkeyHandle {
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

/// Lock a mutex, recovering from poisoning instead of panicking.
///
/// Every bare `.lock().unwrap()` is a landmine in this app: a panic anywhere
/// while the lock is held poisons the mutex, and the release profile uses
/// `panic = "abort"`, so the *next* lock attempt would silently kill the whole
/// process — including its tray icon — instead of degrading.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the app config.
pub(crate) fn lock_config(state: &AppState) -> MutexGuard<'_, Config> {
    lock_or_recover(&state.config)
}

/// Run a cleanup and keep the shared bookkeeping in sync.
///
/// The UI button, the tray menu, the global hotkey and the automatic loop all
/// funnel through here so the "last reduct" timestamp, the persisted config and
/// the `memory-update` event cannot drift apart between entry points — the tray
/// and hotkey paths used to skip the bookkeeping entirely, so an
/// interval-based auto-clean could fire seconds after a manual cleanup.
///
/// Always emits `clean-done` carrying the originating `source`; the frontend
/// ignores `"manual"` because that caller already gets the result back from the
/// `clean_memory` command.
fn perform_clean(app: &AppHandle, mask: u32, source: &str, is_autoclean: bool) -> CleanResult {
    let allow_standby = lock_config(&app.state::<AppState>()).allow_standby_list_cleanup;
    let result = memory::clean_memory(mask, allow_standby, is_autoclean);

    {
        let state = app.state::<AppState>();
        let mut cfg = lock_config(&state);
        cfg.statistic_last_reduct = unix_now();
        // A failed write must not take the cleanup down with it.
        let _ = config::save(&cfg);
    }

    let _ = app.emit("memory-update", memory::get_memory_info());
    let _ = app.emit(
        "clean-done",
        // Borrow the result so it can still be returned to a command caller.
        serde_json::json!({ "source": source, "result": &result }),
    );

    result
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
    app: AppHandle,
    state: State<'_, AppState>,
    mask: Option<u32>,
    source: Option<String>,
) -> CleanResult {
    let cfg = lock_config(&state).clone();
    let mask = mask.unwrap_or(cfg.reduct_mask);
    let is_manual = source.as_deref() == Some("manual");
    let is_autoclean = matches!(
        source.as_deref(),
        Some("auto") | Some("hotkey") | Some("cmdline")
    );

    // Original Mem Reduct behaviour: a manual cleanup while un-elevated
    // relaunches the WHOLE app through the UAC `runas` verb and exits this
    // instance. The elevated instance takes over, so every later cleanup
    // (manual or automatic) runs elevated with no further UAC prompts.
    //
    // The replacement is started with `-takeover`, which lets it wait for *this*
    // process to disappear instead of concluding "another instance is already
    // running" and quitting (see `single_instance`).
    if is_manual && !elevation::is_elevated() && elevation::relaunch_self_as_admin() {
        // Successful relaunch: the elevated instance takes over.
        std::process::exit(0);
    }
    // User cancelled the UAC prompt → fall through to a limited attempt
    // below (mirrors the original's "no privileges" path).

    let source = source.unwrap_or_else(|| "manual".to_string());
    perform_clean(&app, mask, &source, is_autoclean)
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Config {
    lock_config(&state).clone()
}

#[tauri::command]
fn save_config(app: AppHandle, state: State<'_, AppState>, config: Config) -> Result<(), String> {
    // Only re-arm the global hotkey when it actually changed: the setting is
    // saved on every tweak (sliders included), and re-registering costs a
    // thread round-trip.
    let hotkey_changed = {
        let current = lock_config(&state);
        current.hotkey_clean_enable != config.hotkey_clean_enable
            || current.hotkey_clean != config.hotkey_clean
    };
    // The frontend round-trips whatever it last received, so a stale or
    // hand-edited payload must be normalised before it reaches the rest of the
    // app. `config::save` sanitises again on the way to disk.
    let mut config = config;
    config.sanitize();
    *lock_config(&state) = config.clone();
    if hotkey_changed {
        register_hotkey(&app);
    }
    config::save(&config).map_err(|e| e.to_string())
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

/// Open a URL / file with the default system handler.
///
/// Shared by the `open_external` command and the tray's "project page" entry —
/// those two used to carry identical copies of this `ShellExecuteW` dance.
pub fn shell_open(target: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = target.encode_utf16().chain(core::iter::once(0)).collect();
    let verb: Vec<u16> = "open".encode_utf16().chain(core::iter::once(0)).collect();

    // SAFETY: both buffers are NUL-terminated and outlive the call.
    let result = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // ShellExecuteW returns a value greater than 32 on success.
    result.0 as isize > 32
}

/// Open an external URL in the default browser.
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if shell_open(&url) {
        Ok(())
    } else {
        Err("打开链接失败".into())
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

/// Apply frontend-supplied tray menu labels (so the native menu follows the
/// selected app language) and rebuild the menu in place.
#[tauri::command]
fn apply_tray_labels(
    app: AppHandle,
    state: State<'_, AppState>,
    labels: tray::TrayMenuLabels,
) -> Result<(), String> {
    let menu = tray::build_menu(&app, &labels).map_err(|e| e.to_string())?;
    let guard = lock_or_recover(&state.tray);
    match guard.as_ref() {
        Some(tray) => tray.set_menu(Some(menu)).map_err(|e| e.to_string()),
        None => Err("托盘图标未就绪".into()),
    }
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Show the main window (restoring it if minimised) and give it focus.
pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Clean memory triggered from the tray menu.
pub fn clean_from_tray(app: &AppHandle) {
    let mask = lock_config(&app.state::<AppState>()).reduct_mask;
    let _ = perform_clean(app, mask, "tray", false);
}

/// Execute a tray action: 0 = show window, 1 = clean memory.
pub fn run_tray_action(app: &AppHandle, action: u32) {
    match action {
        config::TRAY_ACTION_CLEAN => clean_from_tray(app),
        _ => show_main_window(app),
    }
}

/// (Re)register the global clean hotkey from the current config.
///
/// The previous listener is stopped and *joined* before the new one starts, so
/// two registrations can never overlap (which would make a re-registration of
/// the same combo fail because the old one was still held).
///
/// A failed registration is reported to the frontend: previously the toggle in
/// the UI kept claiming the hotkey was armed while another application actually
/// owned the combination, so the feature silently did nothing.
fn register_hotkey(app: &AppHandle) {
    let state = app.state::<AppState>();
    let cfg = lock_config(&state).clone();

    stop_hotkey(&state);

    if !cfg.hotkey_clean_enable {
        return;
    }

    let (mods, vk) = hotkey::decode(cfg.hotkey_clean);
    let stop = Arc::new(AtomicBool::new(false));
    let app_handle = app.clone();
    let start = hotkey::start(1, mods, vk, stop.clone(), move || {
        let app = app_handle.clone();
        let mask = lock_config(&app.state::<AppState>()).reduct_mask;
        let _ = perform_clean(&app, mask, "hotkey", false);
    });

    match start {
        Ok(join) => {
            *lock_or_recover(&state.hotkey) = Some(HotkeyHandle {
                stop,
                join: Some(join),
            });
        }
        Err(err) => {
            *lock_or_recover(&state.hotkey) = None;
            let _ = app.emit("hotkey-error", err.to_string());
        }
    }
}

/// Stop the current hotkey listener (if any) and wait for its thread to exit.
fn stop_hotkey(state: &AppState) {
    let handle = lock_or_recover(&state.hotkey).take();
    if let Some(mut handle) = handle {
        handle.stop.store(true, Ordering::Relaxed);
        if let Some(join) = handle.join.take() {
            let _ = join.join();
        }
    }
}

/// Decide whether auto-clean should run based on the current usage percent,
/// the on/off flags, the threshold, interval and the last-clean timestamp.
///
/// Mirrors the original: threshold-based cleanup is gated by a 30s cooldown,
/// interval-based cleanup likewise, and both are skipped when disabled.
///
/// The threshold and interval are clamped defensively on top of
/// `Config::sanitize`: a stored `0` would otherwise make the condition
/// unconditionally true and turn the 1 Hz background loop into a permanent
/// clean-every-30-seconds loop.
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
    if enable_by_threshold && percent >= threshold.max(1) {
        return true;
    }
    if enable_by_interval && interval_minutes > 0 && elapsed >= interval_minutes as i64 * 60 {
        return true;
    }
    false
}

/// Periodic background loop: auto-clean + refresh tray + emit info to UI.
fn spawn_background(app: AppHandle) {
    std::thread::spawn(move || {
        // Last rendered tray state; the icon bitmap is only re-rasterised when
        // something it depends on actually changes.
        let mut last_tray: Option<(u32, trayicon::TrayIconStyle, String)> = None;

        // The original uses a 1000ms timer; we use it for tray + data.
        loop {
            std::thread::sleep(Duration::from_millis(1000));

            let state = app.state::<AppState>();
            let cfg = lock_config(&state).clone();

            // One sample per tick, shared by the tray, the UI event and
            // auto-clean.
            let info = memory::get_memory_info();
            let pct = info.physical_memory.percent;

            // Refresh the tray tooltip/bitmap.
            {
                let guard = lock_or_recover(&state.tray);
                if let Some(tray) = guard.as_ref() {
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
                    // Language-neutral tooltip (the tray labels are localised by
                    // the frontend; the numbers are not).
                    let tooltip = format!(
                        "Mem Reduct\n{pct}% · {:.1} GB / {:.1} GB",
                        info.physical_memory.used_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                        info.physical_memory.total_bytes as f64 / 1024.0 / 1024.0 / 1024.0,
                    );

                    let next = (pct, style, tooltip);
                    let changed = last_tray.as_ref() != Some(&next);
                    if changed {
                        // NOTE: `set_title` is a no-op on Windows (tray-icon
                        // only implements it on macOS/Linux), so the percent
                        // reaches the user through the rendered bitmap and the
                        // tooltip instead of an icon label.
                        let _ = tray.set_tooltip(Some(next.2.clone()));
                        let rgba = trayicon::render(pct, &style);
                        let icon = tauri::image::Image::new_owned(rgba, 32, 32);
                        let _ = tray.set_icon(Some(icon));
                        last_tray = Some(next);
                    }
                }
            }

            // Emit fresh memory info to the frontend.
            let _ = app.emit("memory-update", info);

            // Auto-clean by threshold or interval (with a shared 30s cooldown).
            if should_autoclean(
                info.physical_memory.percent,
                cfg.autoreduct_enable,
                cfg.autoreduct_value,
                cfg.autoreduct_interval_enable,
                cfg.autoreduct_interval_value,
                cfg.statistic_last_reduct,
            ) {
                // `perform_clean` persists the timestamp, so a restart right
                // after an auto-clean does not trigger another one immediately.
                let _ = perform_clean(&app, cfg.reduct_mask, "auto", true);
                let _ = app.emit("autoclean-done", ());
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Only one interactive instance may run: a second one would register a
    // second tray icon and start a second 1 Hz background loop competing for
    // it. The one-shot CLI / elevation-helper modes are handled in `main()`
    // *before* this point, so they are intentionally exempt from this guard.
    let takeover = std::env::args().any(|arg| arg == single_instance::TAKEOVER_ARG);
    let _instance = match single_instance::acquire(takeover) {
        single_instance::Acquire::Primary(guard) => guard,
        single_instance::Acquire::Duplicate => {
            // A duplicate *logon* launch stays silent: the user configured
            // "start minimized", so popping the window would be wrong. A
            // duplicate manual launch, on the other hand, should surface the
            // window the user already has instead of doing nothing at all.
            if !autostart::is_startup_launch() {
                single_instance::focus_existing_window();
            }
            return;
        }
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState {
            config: Mutex::new(config::load()),
            tray: Mutex::new(None),
            hotkey: Mutex::new(None),
        })
        .setup(|app| {
            // Create the tray icon and store it in state for background updates.
            if let Ok(tray) = tray::create_tray(app.handle()) {
                *lock_or_recover(&app.state::<AppState>().tray) = Some(tray);
            }

            let handle = app.handle().clone();

            // Window visibility: the window is created hidden (see
            // tauri.conf.json) so startup never flickers. It stays hidden when
            // the user asked for "start minimized to tray" or when we were
            // launched by the logon task (`-startup`); otherwise it is shown.
            let start_minimized = lock_config(&app.state::<AppState>()).start_minimized;
            let silent_launch = autostart::is_startup_launch();
            if let Some(window) = app.get_webview_window("main") {
                if start_minimized || silent_launch {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                }
            }

            // NOTE: `-clean`, `-clean:full` and `-clean-once` are handled in
            // `main()` *before* the UI starts — they clean and exit without ever
            // creating a window.

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

                // Explain it once. Without this the window just disappears and
                // users reasonably conclude that the app failed to close.
                let app = window.app_handle();
                let first_time = {
                    let state = app.state::<AppState>();
                    let mut cfg = lock_config(&state);
                    if cfg.tray_tip_shown {
                        false
                    } else {
                        cfg.tray_tip_shown = true;
                        let _ = config::save(&cfg);
                        true
                    }
                };
                if first_time {
                    // The frontend owns all localisation, so it renders the hint
                    // (and mirrors it to a system notification).
                    let _ = app.emit("hidden-to-tray", ());
                }
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
            apply_tray_labels,
            updater::check_for_update,
            updater::download_and_install,
            updater::get_updater_info
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

    #[test]
    fn autoclean_ignores_zeroed_threshold_and_interval() {
        let now = unix_now();
        // A stored 0 for either value must not turn the rule into
        // "clean every 30 seconds" (see `Config::sanitize`).
        assert!(!should_autoclean(0, true, 0, false, 30, now - 60));
        assert!(!should_autoclean(0, false, 90, true, 0, now - 60));
        // The threshold is clamped to 1 rather than dropped, so a threshold of 0
        // still matches as soon as there is anything at all to reclaim.
        assert!(should_autoclean(1, true, 0, false, 30, now - 60));
        // Both disabled is always a no-op.
        assert!(!should_autoclean(99, false, 90, false, 0, now - 99 * 60));
    }
}
