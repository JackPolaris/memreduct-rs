//! System tray icon: memory percent in the icon bitmap, tooltip, right-click
//! menu and click/double-click actions.
//!
//! Mirrors the original Mem Reduct tray behaviour:
//! - the icon renders the current memory percent (see `trayicon`)
//! - tooltip shows detailed memory status
//! - right-click menu: show window / clean memory / settings / project page / about / exit
//! - single click shows the window, double-click runs the configured action

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

/// Menu item ids (kept in sync with the original tray menu).
pub mod menu_id {
    pub const SHOW: &str = "tray-show";
    pub const CLEAN: &str = "tray-clean";
    pub const SETTINGS: &str = "tray-settings";
    pub const WEBSITE: &str = "tray-website";
    pub const ABOUT: &str = "tray-about";
    pub const EXIT: &str = "tray-exit";
}

/// Where the "project page" menu entry points.
const PROJECT_URL: &str = "https://github.com/JackPolaris/memreduct-rs";

/// Token for the deferred single-click action.
///
/// Windows always delivers a *single* click before the double-click event, so
/// acting on the first click immediately would make a double-click configured as
/// "clean memory" also pop the window open first. The show action is therefore
/// deferred by one double-click interval and cancelled when a second click
/// arrives.
static PENDING_CLICK: AtomicU64 = AtomicU64::new(0);

/// Localised tray menu labels.
///
/// The native menu is built in Rust, so it cannot use the frontend's i18n
/// bundles directly: the UI pushes the translated strings (see the
/// `apply_tray_labels` command) and the menu is rebuilt whenever the language
/// changes. The `Default` is Simplified Chinese, matching the launcher default.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TrayMenuLabels {
    pub show: String,
    pub clean: String,
    pub settings: String,
    pub website: String,
    pub about: String,
    pub exit: String,
}

impl Default for TrayMenuLabels {
    fn default() -> Self {
        Self {
            show: "显示窗口".into(),
            clean: "清理内存".into(),
            settings: "设置".into(),
            website: "官方网站".into(),
            about: "关于".into(),
            exit: "退出".into(),
        }
    }
}

/// Build the tray context menu with the given labels.
pub fn build_menu(app: &AppHandle, labels: &TrayMenuLabels) -> tauri::Result<Menu<Wry>> {
    MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id(menu_id::SHOW, &labels.show).build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::CLEAN, &labels.clean).build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(menu_id::SETTINGS, &labels.settings).build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::WEBSITE, &labels.website).build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::ABOUT, &labels.about).build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(menu_id::EXIT, &labels.exit).build(app)?)
        .build()
}

/// Build and attach the tray icon + context menu + event handlers.
pub fn create_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let menu = build_menu(app, &TrayMenuLabels::default())?;

    let app_handle = app.clone();

    // NOTE: no `.title(...)` — `TrayIcon::set_title` is implemented only on
    // macOS/Linux, so on Windows it is a silent no-op. The percent is drawn into
    // the icon bitmap and repeated in the tooltip instead.
    let mut tray_builder = TrayIconBuilder::with_id("main-tray")
        .tooltip("Mem Reduct")
        .menu(&menu)
        // On Windows the menu should open with right-click only; left click
        // is reserved for showing the window.
        .show_menu_on_left_click(false);

    // Set the icon only if available; a missing icon must not crash the app.
    if let Some(icon) = app.default_window_icon() {
        tray_builder = tray_builder.icon(icon.clone());
    }

    let tray = tray_builder
        .on_menu_event(move |_app, event| handle_menu(&app_handle, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick {
                button: MouseButton::Left,
                ..
            } => {
                // Invalidate the pending single-click show before acting.
                cancel_pending_click();
                let app = tray.app_handle();
                let action = crate::lock_config(&app.state::<crate::AppState>()).tray_action_dc;
                crate::run_tray_action(app, action);
            }
            TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } => {
                let app = tray.app_handle();
                let action = crate::lock_config(&app.state::<crate::AppState>()).tray_action_dc;
                if action == crate::config::TRAY_ACTION_SHOW {
                    // The double-click action is the same, acting right away is
                    // safe and keeps the window from feeling laggy.
                    crate::show_main_window(app);
                } else {
                    schedule_deferred_show(app);
                }
            }
            TrayIconEvent::Click {
                button: MouseButton::Middle,
                ..
            } => {
                let app = tray.app_handle();
                let action = crate::lock_config(&app.state::<crate::AppState>()).tray_action_mc;
                crate::run_tray_action(app, action);
            }
            _ => {}
        })
        .build(app)?;

    Ok(tray)
}

/// Double-click interval as configured in Windows, clamped to a sane range so a
/// broken desktop setting cannot make the single-click action feel dead.
fn double_click_ms() -> u64 {
    let ms = unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() };
    (ms as u64).clamp(200, 1000)
}

/// Defer the single-click "show window" action by one double-click interval.
fn schedule_deferred_show(app: &AppHandle) {
    let token = PENDING_CLICK.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(double_click_ms()));
        // Only the most recent click still owns the action: a second click (i.e.
        // a double-click) bumped the token and already handled itself.
        if PENDING_CLICK.load(Ordering::SeqCst) == token {
            crate::show_main_window(&app);
        }
    });
}

/// Cancel a deferred single-click action.
fn cancel_pending_click() {
    PENDING_CLICK.fetch_add(1, Ordering::SeqCst);
}

/// Handle tray menu clicks.
fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        menu_id::SHOW => crate::show_main_window(app),
        menu_id::CLEAN => {
            crate::clean_from_tray(app);
        }
        menu_id::SETTINGS => {
            crate::show_main_window(app);
            // Emit an event so the frontend can switch to the settings tab.
            let _ = app.emit("open-settings", ());
        }
        menu_id::WEBSITE => {
            // Shares the `ShellExecuteW` helper with the `open_external` command.
            let _ = crate::shell_open(PROJECT_URL);
        }
        menu_id::ABOUT => {
            let _ = app.emit("show-about", ());
        }
        menu_id::EXIT => {
            app.exit(0);
        }
        _ => {}
    }
}
