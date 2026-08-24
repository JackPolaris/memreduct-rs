//! Elevation helpers (mirrors the original `_r_app_runasadmin`).
//!
//! Mem Reduct requires administrator rights for its undocumented NT memory
//! calls. On startup we check whether we are elevated; if not, we relaunch
//! ourselves through the UAC `runas` verb and exit the current process.

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    AdjustTokenPrivileges, GetTokenInformation, LookupPrivilegeValueW, TokenElevation,
    LUID_AND_ATTRIBUTES, SE_PRIVILEGE_ENABLED, TOKEN_ADJUST_PRIVILEGES, TOKEN_ELEVATION,
    TOKEN_PRIVILEGES, TOKEN_QUERY,
};
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

/// Enable the privileges required by the NT memory calls (mirrors the
/// original `_r_sys_setprocessprivilege` list used at startup):
///
/// - `SeProfileSingleProcessPrivilege` — required by `SystemMemoryListInformation`
///   (`MemoryEmptyWorkingSets` / `MemoryPurgeStandbyList` etc.)
/// - `SeIncreaseQuotaPrivilege` — required by `SystemFileCacheInformationEx`
///
/// Without these, the calls fail with `STATUS_PRIVILEGE_NOT_HELD`, which is
/// why cleanup frees far less than the original.
pub fn enable_memory_privileges() {
    const SE_PROFILE_SINGLE_PROCESS: &str = "SeProfileSingleProcessPrivilege";
    const SE_INCREASE_QUOTA: &str = "SeIncreaseQuotaPrivilege";

    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )
        .is_err()
        {
            return;
        }

        for name in [SE_PROFILE_SINGLE_PROCESS, SE_INCREASE_QUOTA] {
            let wide: Vec<u16> = name.encode_utf16().chain(core::iter::once(0)).collect();
            let mut luid = windows::Win32::Foundation::LUID::default();
            if LookupPrivilegeValueW(None, windows::core::PCWSTR(wide.as_ptr()), &mut luid).is_err()
            {
                continue;
            }

            let mut tp: TOKEN_PRIVILEGES = core::mem::zeroed();
            tp.PrivilegeCount = 1;
            tp.Privileges[0] = LUID_AND_ATTRIBUTES {
                Luid: luid,
                Attributes: SE_PRIVILEGE_ENABLED,
            };

            let _ = AdjustTokenPrivileges(
                token,
                false,
                Some(&tp as *const TOKEN_PRIVILEGES),
                0,
                None,
                None,
            );
        }

        let _ = CloseHandle(token);
    }
}

/// Relaunch the current executable elevated via the UAC `runas` verb, passing
/// a single-use `-clean-once <mask>` argument so the elevated copy performs one
/// cleanup and exits (no second window, no duplicate tray icon).
///
/// Returns `true` when the elevation request was successfully submitted.
pub fn relaunch_as_admin(mask: u32) -> bool {
    relaunch_with_args(&format!("-clean-once {mask}"))
}

/// Relaunch the current executable elevated via the UAC `runas` verb with the
/// given arguments. Used for one-shot elevated helpers (cleanup / install).
pub fn relaunch_with_args(args: &str) -> bool {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let Some(exe) = std::env::current_exe().ok() else {
        return false;
    };

    // Build verb "runas" and the argument string.
    let verb: Vec<u16> = "runas".encode_utf16().chain(core::iter::once(0)).collect();
    let exe_wide: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();
    let args_wide: Vec<u16> = args.encode_utf16().chain(core::iter::once(0)).collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::PCWSTR(verb.as_ptr()),
            windows::core::PCWSTR(exe_wide.as_ptr()),
            windows::core::PCWSTR(args_wide.as_ptr()),
            windows::core::PCWSTR::null(),
            SW_HIDE,
        );
        // ShellExecuteW returns a value greater than 32 on success.
        result.0 as usize > 32
    }
}
