// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Administrator rights are requested by the embedded Windows manifest
    // (requireAdministrator) for release builds, so the UAC prompt appears up
    // front with no flash-and-exit. Debug builds run as a normal process.
    mem_reduct_lib::elevation::enable_memory_privileges();
    mem_reduct_lib::run()
}
