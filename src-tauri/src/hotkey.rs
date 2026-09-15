//! Global hotkey handling for "clean memory". Uses `RegisterHotKey`/`GetMessage`
//! in a background thread; supports re-registration via a stop signal.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    MOD_SHIFT, MOD_WIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE, WM_HOTKEY,
};

pub const MOD_ALT_VAL: u32 = 0x0001;
pub const MOD_CONTROL_VAL: u32 = 0x0002;
pub const MOD_SHIFT_VAL: u32 = 0x0004;
pub const MOD_WIN_VAL: u32 = 0x0008;

/// Why a global hotkey could not be registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyError {
    /// `RegisterHotKey` failed — almost always because another application
    /// already owns the combination.
    AlreadyInUse,
    /// The stored combination has no virtual key, so it can never be registered.
    InvalidCombination,
}

impl std::fmt::Display for HotkeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotkeyError::AlreadyInUse => {
                write!(f, "hotkey already in use")
            }
            HotkeyError::InvalidCombination => write!(f, "invalid hotkey combination"),
        }
    }
}

/// Decode a stored hotkey value `(mods << 16) | vk` into (modifiers, vk).
pub fn decode(value: u32) -> (u32, u32) {
    ((value >> 16) & 0xffff, value & 0xffff)
}

/// Encode modifiers + virtual key into a single u32.
pub fn encode(mods: u32, vk: u32) -> u32 {
    ((mods & 0xffff) << 16) | (vk & 0xffff)
}

/// Register a global hotkey and start a message loop that runs until `stop` is
/// set.
///
/// `RegisterHotKey` posts `WM_HOTKEY` to the queue of the thread that called it,
/// so registration has to happen *inside* the thread that pumps the messages.
/// The outcome is handed back over a channel, which lets the caller warn the
/// user when the combination is already taken — previously the failure was
/// swallowed and the UI kept claiming the hotkey was armed.
pub fn start(
    id: i32,
    modifiers: u32,
    key: u32,
    stop: Arc<AtomicBool>,
    on_hotkey: impl Fn() + Send + 'static,
) -> Result<JoinHandle<()>, HotkeyError> {
    if key == 0 {
        return Err(HotkeyError::InvalidCombination);
    }

    let (tx, rx) = std::sync::mpsc::channel::<bool>();

    let join = std::thread::spawn(move || {
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
        // Without MOD_NOREPEAT, holding the combination down makes Windows
        // synthesise a repeat every ~30 ms and the app cleans in a tight loop.
        fs_mods |= MOD_NOREPEAT;

        let hwnd = HWND::default();
        let registered = unsafe { RegisterHotKey(hwnd, id, fs_mods, key) }.is_ok();
        let _ = tx.send(registered);
        if !registered {
            return;
        }

        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            // The queue is drained with a non-blocking `PeekMessageW` instead of
            // the blocking `GetMessageW`: a blocked `GetMessageW` only returns
            // when a message arrives, so a re-registered hotkey used to keep its
            // thread (and its `RegisterHotKey` registration) alive forever, which
            // both leaked threads and left the old hotkey active.
            while !stop.load(Ordering::Relaxed) {
                while PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_HOTKEY && msg.wParam.0 as i32 == id {
                        on_hotkey();
                    }
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }

        let _ = unsafe { UnregisterHotKey(hwnd, id) };
    });

    // Wait for the in-thread registration so the caller knows the truth. The
    // thread either registers (or fails) almost immediately.
    match rx.recv() {
        Ok(true) => Ok(join),
        Ok(false) => {
            let _ = join.join();
            Err(HotkeyError::AlreadyInUse)
        }
        Err(_) => {
            let _ = join.join();
            Err(HotkeyError::AlreadyInUse)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let raw = encode(MOD_CONTROL_VAL, 0x71);
        let (mods, vk) = decode(raw);
        assert_eq!(mods, MOD_CONTROL_VAL);
        assert_eq!(vk, 0x71);
        // Default: Ctrl+F1.
        assert_eq!(decode((0x0002u32 << 16) | 0x71), (MOD_CONTROL_VAL, 0x71));
    }

    #[test]
    fn stop_flag_ends_the_loop() {
        use std::sync::atomic::AtomicUsize;
        let hits = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let counter = hits.clone();
        // A deliberately free id + unlikely combo: registration may still fail
        // on a busy desktop, which is fine — there is then no thread to stop.
        match start(
            0x7fff,
            MOD_CONTROL_VAL | MOD_ALT_VAL | MOD_SHIFT_VAL,
            0x7b, // F12
            stop.clone(),
            move || {
                counter.fetch_add(1, Ordering::Relaxed);
            },
        ) {
            Ok(handle) => {
                std::thread::sleep(Duration::from_millis(80));
                stop.store(true, Ordering::Relaxed);
                handle.join().expect("hotkey thread should stop promptly");
                assert_eq!(hits.load(Ordering::Relaxed), 0);
            }
            Err(e) => {
                // The combination was already taken on this desktop.
                assert_eq!(e, HotkeyError::AlreadyInUse);
            }
        }
    }

    #[test]
    fn keyless_combination_is_rejected() {
        let stop = Arc::new(AtomicBool::new(false));
        let err = start(1, MOD_CONTROL_VAL, 0, stop, || {}).err();
        assert_eq!(err, Some(HotkeyError::InvalidCombination));
    }

    #[test]
    fn norepeat_bit_is_not_part_of_the_stored_encoding() {
        // MOD_NOREPEAT is applied at registration time only; storing it would
        // corrupt the `(mods << 16) | vk` encoding the config and UI share.
        let raw = encode(MOD_CONTROL_VAL, 0x71);
        assert_eq!(decode(raw), (MOD_CONTROL_VAL, 0x71));
        assert_eq!(raw & 0xffff, 0x71);
    }
}
