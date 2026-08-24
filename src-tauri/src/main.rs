// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Single-use elevated cleanup helper: perform one cleanup with the given
    // mask and exit without starting the UI. Invoked by the normal process via
    // `elevation::relaunch_as_admin` (UAC `runas`), so the app never asks for
    // elevation at launch — only when the user triggers a manual cleanup.
    if let mem_reduct_lib::cmdline::CommandLineAction::CleanOnce(mask) =
        mem_reduct_lib::cmdline::parse()
    {
        mem_reduct_lib::elevation::enable_memory_privileges();
        // Allow standby lists only to be excluded in auto-clean; for a manual
        // elevation helper we clean exactly the requested mask.
        let _ = mem_reduct_lib::memory::clean_memory(mask, true, false);
        return;
    }

    // Enable the memory privileges (no-op unless elevated) and run the app.
    mem_reduct_lib::elevation::enable_memory_privileges();
    mem_reduct_lib::run()
}
