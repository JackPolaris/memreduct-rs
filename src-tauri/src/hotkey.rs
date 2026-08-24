//! Global hotkey handling for "clean memory". Uses `RegisterHotKey`/`GetMessage`
//! in a background thread; supports re-registration via a stop signal.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
};

pub const MOD_ALT_VAL: u32 = 0x0001;
pub const MOD_CONTROL_VAL: u32 = 0x0002;
pub const MOD_SHIFT_VAL: u32 = 0x0004;
pub const MOD_WIN_VAL: u32 = 0x0008;

/// Decode a stored hotkey value `(mods << 16) | vk` into (modifiers, vk).
pub fn decode(value: u32) -> (u32, u32) {
    ((value >> 16) & 0xffff, value & 0xffff)
}

/// Encode modifiers + virtual key into a single u32.
pub fn encode(mods: u32, vk: u32) -> u32 {
    ((mods & 0xffff) << 16) | (vk & 0xffff)
}

/// Register a global hotkey and run a message loop until `stop` is set.
pub fn run_hotkey_loop(
    id: i32,
    modifiers: u32,
    key: u32,
    stop: Arc<AtomicBool>,
    on_hotkey: impl Fn() + Send + 'static,
) {
    std::thread::spawn(move || {
        // Map our modifier bits to windows crate HOT_KEY_MODIFIERS.
        let mut fs_mods = HOT_KEY_MODIFIERS(0);
        if modifiers & MOD_ALT_VAL != 0 {
            fs_mods |= MOD_ALT;
        }
        if modifiers & MOD_CONTROL_VAL != 0 {
            fs_mods |= MOD_CONTROL;
        }
        if modifiers & MOD_SHIFT_VAL != 0 {
            fs_mods |= MOD_SHIFT;
        }
        if modifiers & MOD_WIN_VAL != 0 {
            fs_mods |= MOD_WIN;
        }

        let hwnd = HWND::default();
        let ok = unsafe { RegisterHotKey(hwnd, id, fs_mods, key) };
        if ok.is_err() {
            return; // registration failed (e.g. already in use)
        }

        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                // Peek with a short timeout so we can observe `stop`.
                let r = GetMessageW(&mut msg, hwnd, 0, 0);
                if r.0 == 0 || r.0 == -1 {
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
    });
}
