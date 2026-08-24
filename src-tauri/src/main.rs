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

    // --- Normal app launch ---
    mem_reduct_lib::elevation::enable_memory_privileges();
    mem_reduct_lib::run()
}
