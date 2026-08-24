//! Silent autostart + permanent elevation via Windows Task Scheduler.
//!
//! The scheduled task "Mem Reduct" runs at user logon with highest privileges
//! (`/rl HIGHEST`), so the app starts silently as administrator on every boot
//! with no UAC prompt. The *only* UAC prompt the user ever sees is the single
//! one needed to create the task the first time.
//!
//! All `schtasks.exe` invocations use `CREATE_NO_WINDOW` so no console window
//! ever flashes on screen (production polish).

use std::os::windows::process::CommandExt;
use std::process::Command;

/// `CREATE_NO_WINDOW` — never show a console window for schtasks.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The scheduled task name.
pub const TASK_NAME: &str = "Mem Reduct";

/// The argument the task passes so the app knows it was launched at startup
/// (used to start minimized to the tray instead of opening the window).
pub const STARTUP_ARG: &str = "-startup";

/// Run `schtasks.exe` with the given args, hiding any console window.
fn schtasks(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// Query whether the scheduled task exists.
pub fn is_enabled() -> bool {
    matches!(schtasks(&["/query", "/tn", TASK_NAME]), Ok(o) if o.status.success())
}

/// Create (or update) the logon task that runs this executable elevated and
/// silently at every logon. Requires elevation — call from an elevated
/// context (the helper process).
pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_path = exe.to_string_lossy().to_string();
    // schtasks requires the command to be quoted properly.
    let tr = format!("\"{exe_path}\" {STARTUP_ARG}");

    let out = schtasks(&[
        "/create", "/tn", TASK_NAME, "/tr", &tr, "/sc", "onlogon", "/rl", "HIGHEST", "/f",
    ])
    .map_err(|e| e.to_string())?;

    if out.status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /create 退出码: {}", out.status))
    }
}

/// Remove the scheduled task (disables autostart). Requires elevation.
pub fn uninstall() -> Result<(), String> {
    let out = schtasks(&["/delete", "/tn", TASK_NAME, "/f"]).map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /delete 退出码: {}", out.status))
    }
}

/// True when the process was launched by the scheduled task (`-startup` arg).
pub fn is_startup_launch() -> bool {
    std::env::args().any(|a| a == STARTUP_ARG)
}

/// Silently start the scheduled task (no UAC prompt). Used when the user
/// launched the app manually but elevation is already persisted via the
/// task: we trigger the elevated instance and the current one exits.
pub fn run_task() -> Result<(), String> {
    let out = schtasks(&["/run", "/tn", TASK_NAME]).map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /run 退出码: {}", out.status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_arg_constant_is_consistent() {
        assert_eq!(STARTUP_ARG, "-startup");
    }

    #[test]
    fn no_window_flag_is_create_no_window() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }
}
