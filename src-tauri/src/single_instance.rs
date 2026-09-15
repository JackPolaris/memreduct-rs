//! Single-instance guard.
//!
//! Before this module existed, launching the app twice — double-clicking the
//! shortcut again, running the installer, or hitting "run" in the IDE while an
//! instance was alive — produced **two** tray icons and two independent 1 Hz
//! background loops that both fought over the tray icon and both ran the
//! auto-clean rules.
//!
//! A named mutex is the cheapest reliable guard on Windows and needs no extra
//! dependency. The subtlety is the elevation hand-over: a manual cleanup while
//! un-elevated relaunches the whole process through the UAC `runas` verb and
//! exits the outgoing one (see [`crate::elevation::relaunch_self_as_admin`]), so
//! the replacement can start *before* the old instance is gone. That case is
//! flagged with [`TAKEOVER_ARG`] and waits for the hand-over instead of bailing
//! out and leaving the user with no window at all.

use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{
    CloseHandle, BOOL, HANDLE, HWND, LPARAM, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, ReleaseMutex,
    WaitForSingleObject, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible,
    SetForegroundWindow, SetWindowPos, ShowWindow, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE,
    SWP_NOSIZE, SWP_SHOWWINDOW, SW_RESTORE,
};

/// Per-session mutex (`Local\`) so two users logged into the same machine do
/// not block each other.
const MUTEX_NAME: &str = r"Local\MemReduct.App.SingleInstance";

/// How long a hand-over waits for the outgoing instance to release the mutex.
/// The outgoing instance calls `std::process::exit(0)` immediately after
/// submitting the UAC request, so in practice this returns in milliseconds.
const HANDOVER_TIMEOUT_MS: u32 = 15_000;

/// Argument added by the elevated relaunch so the replacement instance knows it
/// is taking over from an instance that is about to exit.
pub const TAKEOVER_ARG: &str = "-takeover";

/// Keeps this process the only interactive instance. Released on drop.
pub struct InstanceGuard {
    mutex: Option<HANDLE>,
}

impl InstanceGuard {
    /// A guard that owns nothing — used when the mutex cannot be created at all,
    /// in which case the app fails open and simply runs unsynchronised.
    fn unavailable() -> Self {
        Self { mutex: None }
    }

    /// Give the mutex up explicitly (used before a controlled hand-over).
    pub fn release(&mut self) {
        if let Some(mutex) = self.mutex.take() {
            // Ownership is per-thread; `run()` and therefore this guard live on
            // the main thread, so the release always happens on the owner.
            let _ = unsafe { ReleaseMutex(mutex) };
            let _ = unsafe { CloseHandle(mutex) };
        }
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        self.release();
    }
}

/// Outcome of [`acquire`].
pub enum Acquire {
    /// This process owns the instance (or the guard was unavailable, which is
    /// treated as success so a broken mutex cannot lock the user out).
    Primary(InstanceGuard),
    /// Another instance is already running; the caller must return immediately.
    Duplicate,
}

/// Try to become the primary instance.
///
/// `allow_takeover` is `true` for the elevated relaunch, which is expected to
/// start while the outgoing instance is still shutting down.
pub fn acquire(allow_takeover: bool) -> Acquire {
    acquire_named(MUTEX_NAME, allow_takeover)
}

/// [`acquire`] against an explicit mutex name — split out so the unit tests can
/// use a private name and never collide with a real running instance.
fn acquire_named(name: &str, allow_takeover: bool) -> Acquire {
    let name: Vec<u16> = name.encode_utf16().chain(core::iter::once(0)).collect();

    // `bInitialOwner = true`: a freshly created mutex is owned by this thread, so
    // the wait below succeeds instantly in the common case.
    let mutex = match unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) } {
        Ok(handle) => handle,
        Err(_) => return Acquire::Primary(InstanceGuard::unavailable()),
    };

    // Waiting (rather than trusting `GetLastError() == ERROR_ALREADY_EXISTS`) is
    // what makes the elevation hand-over safe: the replacement waits for the
    // outgoing process to die instead of concluding "duplicate" and quitting.
    let timeout = if allow_takeover {
        HANDOVER_TIMEOUT_MS
    } else {
        0
    };
    match unsafe { WaitForSingleObject(mutex, timeout) } {
        // `WAIT_ABANDONED` means the previous owner died — the documented
        // behaviour is that we now own the mutex.
        WAIT_OBJECT_0 | WAIT_ABANDONED => Acquire::Primary(InstanceGuard { mutex: Some(mutex) }),
        WAIT_TIMEOUT => {
            let _ = unsafe { CloseHandle(mutex) };
            Acquire::Duplicate
        }
        _ => {
            let _ = unsafe { CloseHandle(mutex) };
            Acquire::Primary(InstanceGuard::unavailable())
        }
    }
}

/// Bring the window of the already-running instance to the foreground.
///
/// Returns `false` when no matching window was found — for example while the
/// other instance is still starting up — which callers may simply ignore.
pub fn focus_existing_window() -> bool {
    let exe = match std::env::current_exe() {
        Ok(path) => path.to_string_lossy().to_lowercase(),
        Err(_) => return false,
    };

    let mut search = Search {
        self_pid: unsafe { GetCurrentProcessId() },
        exe,
        found: false,
    };

    // The callback mutates `search`, so the enumeration must complete before we
    // read the result; `EnumWindows` is synchronous, and a failure simply leaves
    // `found` false.
    let _ = unsafe { EnumWindows(Some(enum_proc), LPARAM(&mut search as *mut Search as isize)) };
    search.found
}

/// Shared state for the window enumeration callback.
struct Search {
    self_pid: u32,
    /// Lower-cased full path of this executable.
    exe: String,
    found: bool,
}

/// `EnumWindows` callback: find the visible top-level window owned by another
/// process running the *same* executable, then restore and focus it.
///
/// Matching on the executable path (rather than the window class) keeps this
/// correct even when other Tauri/WebView-based apps are running — every one of
/// them uses the same `WRY_WEBVIEW` window class.
unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: `lparam` is the `&mut Search` handed to `EnumWindows` above, and
    // that reference outlives the synchronous enumeration.
    let search = unsafe { &mut *(lparam.0 as *mut Search) };

    if search.found || !unsafe { IsWindowVisible(hwnd).as_bool() } {
        return BOOL(1);
    }
    // Only real top-level windows carry a title; tooltips, the tray helper
    // window and message-only windows do not.
    if unsafe { GetWindowTextLengthW(hwnd) } == 0 {
        return BOOL(1);
    }

    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 || pid == search.self_pid {
        return BOOL(1);
    }

    let Some(path) = process_image_path(pid) else {
        return BOOL(1);
    };
    if !path.eq_ignore_ascii_case(&search.exe) {
        return BOOL(1);
    }

    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
    search.found = true;
    // Stop enumerating: this window is the one we wanted.
    BOOL(0)
}

/// Full image path of `pid`, lower-cased. `None` when it cannot be queried
/// (a protected process, or one that already exited).
fn process_image_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is owned by this function and closed before returning;
    // the buffer is sized and its capacity is passed to the API.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        result.ok()?;
        // `len` excludes the terminating null.
        let len = (len as usize).min(buf.len());
        Some(String::from_utf16_lossy(&buf[..len]).to_lowercase())
    }
}

/// Raise a window to the foreground, defeating Windows' foreground lock.
///
/// `hwnd_raw` is the raw `HWND` value, passed as an `isize` so this module does
/// not have to agree with whichever `windows` crate version Tauri links (its
/// `WebviewWindow::hwnd()` re-exports that crate's `HWND`).
///
/// Why this exists: a UAC-elevated relaunch cannot just call
/// `SetForegroundWindow`. The consent dialog runs on the secure desktop, so the
/// elevated child is *not* "started by the foreground process" in the way the
/// foreground-lock rules require — by the time it shows its window the lock is
/// held by whatever the user was working in, the call is rejected, and the
/// freshly restarted app sits behind everything looking like it silently
/// started to the tray.
///
/// The momentary topmost flip is the standard workaround: `SetWindowPos` is not
/// subject to the foreground lock, and the follow-up `HWND_NOTOPMOST` leaves the
/// window above everything else without the always-on-top side effect.
pub fn force_foreground(hwnd_raw: isize) {
    if hwnd_raw == 0 {
        return;
    }
    // SAFETY: `hwnd_raw` is a live top-level window handle handed over by the
    // caller (taken from Tauri's own window object moments earlier).
    let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        if SetForegroundWindow(hwnd).as_bool() {
            return;
        }
        let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW;
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, flags);
        let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, flags);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name that cannot collide with a real running instance.
    const TEST_MUTEX: &str = r"Local\MemReduct.Test.SingleInstance";

    #[test]
    fn acquisition_succeeds_and_release_is_idempotent() {
        let Acquire::Primary(mut guard) = acquire_named(TEST_MUTEX, false) else {
            panic!("the test mutex should be free");
        };
        // Releasing twice must not panic or double-release a closed handle.
        guard.release();
        guard.release();
    }

    #[test]
    fn unavailable_guard_releases_safely() {
        let mut guard = InstanceGuard::unavailable();
        guard.release();
        guard.release();
    }

    #[test]
    fn focusing_without_a_peer_is_a_no_op() {
        // There is no second instance of this test binary, so the enumeration
        // must simply find nothing instead of panicking.
        let found = focus_existing_window();
        // A previously started *app* instance would still match the executable
        // check, but in CI there is none; accept either outcome and only require
        // that the call is safe.
        let _ = found;
    }
}
