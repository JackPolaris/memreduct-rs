//! System tray icon: memory percent title, tooltip, right-click menu and
//! click/double-click actions.
//!
//! Mirrors the original Mem Reduct tray behaviour:
//! - the icon shows the current memory percent as its title (Windows NIF_TITLE)
//! - tooltip shows detailed memory status
//! - right-click menu: show window / clean memory / settings / website / exit
//! - single click shows the window, double-click cleans (configurable later)

use std::sync::Mutex;

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Tray status shared with the background refresh loop.
pub struct TrayState {
    pub percent: Mutex<u32>,
    pub tooltip: Mutex<String>,
}

/// Menu item ids (kept in sync with the original tray menu).
pub mod menu_id {
    pub const SHOW: &str = "tray-show";
    pub const CLEAN: &str = "tray-clean";
    pub const SETTINGS: &str = "tray-settings";
    pub const WEBSITE: &str = "tray-website";
    pub const ABOUT: &str = "tray-about";
    pub const EXIT: &str = "tray-exit";
}

/// Build and attach the tray icon + context menu + event handlers.
pub fn create_tray(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let menu = MenuBuilder::new(app)
        .item(&MenuItemBuilder::with_id(menu_id::SHOW, "显示窗口").build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::CLEAN, "清理内存").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(menu_id::SETTINGS, "设置").build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::WEBSITE, "官方网站").build(app)?)
        .item(&MenuItemBuilder::with_id(menu_id::ABOUT, "关于").build(app)?)
        .separator()
        .item(&MenuItemBuilder::with_id(menu_id::EXIT, "退出").build(app)?)
        .build()?;

    let app_handle = app.clone();

    let tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Mem Reduct")
        .title("0%")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        // On Windows the menu should open with right-click only; left click
        // is reserved for showing the window.
        .show_menu_on_left_click(false)
        .on_menu_event(move |_app, event| handle_menu(&app_handle, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| match event {
            TrayIconEvent::DoubleClick { button, .. } if button == tauri::tray::MouseButton::Left => {
                let app = tray.app_handle();
                let action = app
                    .state::<crate::AppState>()
                    .config
                    .lock()
                    .unwrap()
                    .tray_action_dc;
                crate::run_tray_action(app, action);
            }
            TrayIconEvent::Click { button, .. } if button == tauri::tray::MouseButton::Left => {
                let app = tray.app_handle();
                // Single left click → show window (matches original default).
                crate::run_tray_action(app, 0);
            }
            TrayIconEvent::Click { button, .. } if button == tauri::tray::MouseButton::Middle => {
                let app = tray.app_handle();
                let action = app
                    .state::<crate::AppState>()
                    .config
                    .lock()
                    .unwrap()
                    .tray_action_mc;
                crate::run_tray_action(app, action);
            }
            _ => {}
        })
        .build(app)?;

    Ok(tray)
}

/// Handle tray menu clicks.
fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        menu_id::SHOW => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
        menu_id::CLEAN => {
            crate::clean_from_tray(app);
        }
        menu_id::SETTINGS => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
                // Emit an event so the frontend can switch to the settings tab.
                let _ = app.emit("open-settings", ());
            }
        }
        menu_id::WEBSITE => {
            let _ = open_url("https://github.com/henrypp/memreduct");
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

/// Open a URL via the system shell (fallback if plugin not available).
fn open_url(url: &str) -> Result<(), ()> {
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
            Err(())
        }
    }
}
