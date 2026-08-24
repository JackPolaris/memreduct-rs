// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Mem Reduct requires administrator rights for its NT memory operations;
    // request elevation before starting the app (mirrors the original).
    mem_reduct_lib::elevation::ensure_elevated_or_exit();
    // Enable the privileges needed by the NT memory calls (as the original
    // does in _app_initialize).
    mem_reduct_lib::elevation::enable_memory_privileges();
    mem_reduct_lib::run()
}
