// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --- One-shot elevated helpers (exit immediately, no UI) ---

    // `-clean-once <mask>`: perform one cleanup with the given mask and exit.
    if let mem_reduct_lib::cmdline::CommandLineAction::CleanOnce(mask) =
        mem_reduct_lib::cmdline::parse_args(&args)
    {
        mem_reduct_lib::elevation::enable_memory_privileges();
        let _ = mem_reduct_lib::memory::clean_memory(mask, true, false);
        return;
    }

    // `-ensure-autostart`: create the silent elevated logon task (one UAC lift).
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

    // --- Normal app launch with permanent on-demand elevation ---
    mem_reduct_lib::elevation::enable_memory_privileges();

    // Skip elevation logic when already elevated (e.g. launched by the task).
    if mem_reduct_lib::elevation::is_elevated() {
        mem_reduct_lib::run();
        return;
    }

    // Not elevated. Two cases:
    // 1) The elevated task already exists → silently trigger it and exit
    //    (no UAC, the elevated instance takes over).
    // 2) No task yet → this is the first launch: create the task via a single
    //    UAC prompt (unless the user previously declined).
    if mem_reduct_lib::autostart::is_enabled() {
        let _ = mem_reduct_lib::autostart::run_task();
        return;
    }

    let mut cfg = mem_reduct_lib::config::load();
    if !cfg.elevation_attempted {
        cfg.elevation_attempted = true;
        let _ = mem_reduct_lib::config::save(&cfg);
        // First launch: one UAC prompt to persist elevation forever.
        let _ = mem_reduct_lib::elevation::relaunch_with_args("-ensure-autostart");
        return;
    }

    // User declined elevation before: run normally without admin rights.
    mem_reduct_lib::run()
}
