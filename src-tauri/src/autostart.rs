//! Silent autostart + permanent elevation via Windows Task Scheduler.
//!
//! The scheduled task "Mem Reduct" runs at user logon with highest privileges
//! (`/rl HIGHEST`), so the app starts silently as administrator on every boot
//! with no UAC prompt. The *only* UAC prompt the user ever sees is the single
//! one needed to create the task the first time.
//!
//! Because the app then runs elevated, manual *and* automatic cleanups work
//! directly without any further elevation requests.

use std::process::Command;

/// The scheduled task name.
pub const TASK_NAME: &str = "Mem Reduct";

/// The argument the task passes so the app knows it was launched at startup
/// (used to start minimized to the tray instead of opening the window).
pub const STARTUP_ARG: &str = "-startup";

/// Query whether the scheduled task exists.
pub fn is_enabled() -> bool {
    let out = Command::new("schtasks.exe")
        .args(["/query", "/tn", TASK_NAME])
        .output();
    matches!(out, Ok(o) if o.status.success())
}

/// Create (or update) the logon task that runs this executable elevated and
/// silently at every logon. Requires elevation — call from an elevated
/// context (the helper process).
pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_path = exe.to_string_lossy().to_string();
    // schtasks requires the command to be quoted properly.
    let tr = format!("\"{exe_path}\" {STARTUP_ARG}");

    let status = Command::new("schtasks.exe")
        .args([
            "/create", "/tn", TASK_NAME, "/tr", &tr, "/sc", "onlogon", "/rl", "HIGHEST", "/f",
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /create 退出码: {status}"))
    }
}

/// Remove the scheduled task (disables autostart). Requires elevation.
pub fn uninstall() -> Result<(), String> {
    let status = Command::new("schtasks.exe")
        .args(["/delete", "/tn", TASK_NAME, "/f"])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /delete 退出码: {status}"))
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
    let status = Command::new("schtasks.exe")
        .args(["/run", "/tn", TASK_NAME])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("schtasks /run 退出码: {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_arg_constant_is_consistent() {
        assert_eq!(STARTUP_ARG, "-startup");
    }
}
