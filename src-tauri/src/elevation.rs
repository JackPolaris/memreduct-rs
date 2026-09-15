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

/// Relaunch the entire app elevated (mirrors the original `_r_app_runasadmin`).
///
/// Original Mem Reduct behaviour: on a manual cleanup while un-elevated, it
/// relaunches the *whole* process through the UAC `runas` verb with the
/// original command line and working directory, then exits the current
/// process. The elevated instance takes over and every later cleanup (manual
/// or automatic) runs elevated with no further UAC prompts.
///
/// Returns `true` when the elevated relaunch was submitted (the caller should
/// then exit); `false` when the user cancelled the UAC prompt.
pub fn relaunch_self_as_admin() -> bool {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::{AllowSetForegroundWindow, ASFW_ANY, SW_SHOW};

    let Some(exe) = std::env::current_exe().ok() else {
        return false;
    };

    // Original command line, quoted back together (arguments with spaces would
    // otherwise be split apart), minus `-startup`: the user is interacting with
    // the window right now, so the elevated instance must open its window rather
    // than start hidden in the tray.
    let mut cmdline = std::env::args()
        .skip(1)
        .filter(|arg| {
            arg != crate::autostart::STARTUP_ARG && arg != crate::single_instance::TAKEOVER_ARG
        })
        .map(|arg| quote_arg(&arg))
        .collect::<Vec<_>>();

    // Mark this as a hand-over. The replacement instance starts while *this*
    // process is still shutting down, so it must wait for the single-instance
    // mutex instead of reporting "another instance is already running" and
    // quitting — which would leave the user with no window at all.
    cmdline.push(crate::single_instance::TAKEOVER_ARG.to_string());
    let cmdline = cmdline.join(" ");

    // Current working directory (mirrors _r_sys_getcurrentdirectory).
    let cwd = std::env::current_dir().ok();

    let verb: Vec<u16> = "runas".encode_utf16().chain(core::iter::once(0)).collect();
    let exe_wide: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(core::iter::once(0))
        .collect();
    let args_wide: Vec<u16> = cmdline.encode_utf16().chain(core::iter::once(0)).collect();
    let cwd_wide = cwd.map(|p| {
        p.to_string_lossy()
            .encode_utf16()
            .chain(core::iter::once(0))
            .collect::<Vec<u16>>()
    });

    // Hand over the foreground: after the consent dialog the elevated child is
    // spawned by the UAC service rather than by this process, so it fails the
    // foreground-lock rule "started by the foreground process" and its window
    // would come up *behind* whatever the user was doing. Granting `ASFW_ANY`
    // (allowed because *this* process owns the foreground right now — the user
    // just clicked in its window) lets the child win the lock. The child still
    // enforces it itself (`single_instance::force_foreground`) for the case
    // where the grant has already lapsed by the time it shows its window.
    unsafe {
        let _ = AllowSetForegroundWindow(ASFW_ANY);
    }

    unsafe {
        let result = ShellExecuteW(
            None,
            windows::core::PCWSTR(verb.as_ptr()),
            windows::core::PCWSTR(exe_wide.as_ptr()),
            windows::core::PCWSTR(args_wide.as_ptr()),
            cwd_wide
                .as_ref()
                .map(|w| windows::core::PCWSTR(w.as_ptr()))
                .unwrap_or_else(windows::core::PCWSTR::null),
            SW_SHOW,
        );
        // ShellExecuteW returns a value greater than 32 on success.
        result.0 as usize > 32
    }
}

/// Quote a single command-line argument so it survives being joined back into a
/// command-line string (Windows `CommandLineToArgvW` rules: wrap in quotes and
/// escape embedded quotes/backslashes).
fn quote_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0usize;
    for ch in arg.chars() {
        match ch {
            '\\' => {
                backslashes += 1;
                out.push('\\');
            }
            '"' => {
                // Double the preceding backslashes, then escape the quote.
                for _ in 0..backslashes {
                    out.push('\\');
                }
                backslashes = 0;
                out.push('\\');
                out.push('"');
            }
            _ => {
                backslashes = 0;
                out.push(ch);
            }
        }
    }
    // A trailing backslash would escape the closing quote.
    for _ in 0..backslashes {
        out.push('\\');
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_keeps_plain_arguments_intact() {
        assert_eq!(quote_arg("-clean"), "-clean");
        assert_eq!(quote_arg("C:\\path\\file.txt"), "C:\\path\\file.txt");
    }

    #[test]
    fn quoting_wraps_arguments_with_spaces() {
        assert_eq!(
            quote_arg("C:\\Program Files\\app.exe"),
            "\"C:\\Program Files\\app.exe\""
        );
        assert_eq!(quote_arg(""), "\"\"");
    }

    #[test]
    fn quoting_escapes_embedded_quotes_and_trailing_backslashes() {
        assert_eq!(quote_arg("a\"b"), "\"a\\\"b\"");
        // Unquoted arguments keep their backslashes verbatim …
        assert_eq!(quote_arg("dir\\"), "dir\\");
        // … while a quoted argument needs the trailing backslash doubled, so it
        // cannot escape the closing quote.
        assert_eq!(
            quote_arg("C:\\Program Files\\x\\"),
            "\"C:\\Program Files\\x\\\\\""
        );
    }
}
