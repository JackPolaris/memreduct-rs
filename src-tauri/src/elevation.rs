//! Elevation helpers (mirrors the original `_r_app_runasadmin`).
//!
//! Mem Reduct requires administrator rights for its undocumented NT memory
//! calls. On startup we check whether we are elevated; if not, we relaunch
//! ourselves through the UAC `runas` verb and exit the current process.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// True when the current process token is elevated.
pub fn is_elevated() -> bool {
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut TOKEN_ELEVATION as *mut core::ffi::c_void),
            core::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        let _ = CloseHandle(token);
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// Relaunch the current executable with the `runas` verb (UAC prompt).
/// Returns true if the ShellExecute call succeeded.
pub fn relaunch_as_admin() -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return false,
    };
    let exe_wide: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();

    // static NUL-terminated "runas" wide string.
    static RUNAS: [u16; 6] = [0x72, 0x75, 0x6e, 0x61, 0x73, 0x00]; // "runas\0"

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR(RUNAS.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
        result.0 as isize > 32
    }
}

/// Startup elevation check: if not elevated, relaunch via UAC and exit.
pub fn ensure_elevated_or_exit() {
    if is_elevated() {
        return;
    }
    if relaunch_as_admin() {
        std::process::exit(0);
    }
}
