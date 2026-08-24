//! Global hotkey handling for "clean memory" (mirrors the original Ctrl+F1
//! default). Uses `RegisterHotKey`/`GetMessage` in a background thread.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
};

/// Register a global hotkey and run a message loop until the thread is told to
/// stop. `on_hotkey` is invoked whenever the hotkey fires.
///
/// `modifiers` and `key` are the same values used by `RegisterHotKey`
/// (a `MOD_*` bitmask and a virtual-key code).
pub fn run_hotkey_loop(
    id: i32,
    modifiers: u32,
    key: u32,
    on_hotkey: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || loop {
        let mods = windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(modifiers);
        let hwnd = HWND::default();
        let ok = unsafe { RegisterHotKey(hwnd, id, mods, key) };
        if ok.is_err() {
            // Hotkey already in use / registration failed; retry after a wait.
            std::thread::sleep(std::time::Duration::from_secs(3));
            continue;
        }

        unsafe {
            // WM_HOTKEY is delivered to the thread's message queue even without
            // a window, so a raw-pump here works.
            let mut msg: MSG = std::mem::zeroed();
            loop {
                let r = GetMessageW(&mut msg, hwnd, 0, 0);
                if r.0 == 0 {
                    // WM_QUIT
                    break;
                }
                if r.0 == -1 {
                    break;
                }
                if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == id {
                    on_hotkey();
                }
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }

        let _ = unsafe { UnregisterHotKey(hwnd, id) };

        // Avoid busy spinning if the loop exits unexpectedly.
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
}
