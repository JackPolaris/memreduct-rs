// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --- One-shot elevated helpers (exit immediately, no UI) ---
    // These are fired from the UAC `runas` verb; none of them open a window.

    // `-ensure-autostart`: create the silent elevated logon task.
    if args.iter().any(|a| a == "-ensure-autostart") {
        mem_reduct_lib::elevation::enable_memory_privileges();
        let _ = mem_reduct_lib::autostart::install();
        return;
    }

    // `-disable-autostart`: remove the logon task.
    if args.iter().any(|a| a == "-disable-autostart") {
        mem_reduct_lib::elevation::enable_memory_privileges();
        let _ = mem_reduct_lib::autostart::uninstall();
        return;
    }

    // --- Normal app launch ---
    //
    // Mirrors the original Mem Reduct: the app starts WITHOUT elevation (no
    // UAC on launch). When the user triggers a manual cleanup while un-elevated,
    // the whole app relaunches itself through the UAC `runas` verb and the old
    // instance exits (see `elevation::relaunch_self_as_admin`). The elevated
    // instance then handles all subsequent cleanups with no further prompts.
    mem_reduct_lib::elevation::enable_memory_privileges();
    mem_reduct_lib::run()
}
