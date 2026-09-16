// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use mem_reduct_lib::cmdline::{self, CommandLineAction};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --- One-shot elevated helpers (exit immediately, no UI) ---
    // These are fired from the UAC `runas` verb; none of them open a window.
    //
    // Each one *reports back* through `autostart::publish_result` before
    // exiting. This process is a throwaway and nothing else can see whether
    // `schtasks` worked, so without the acknowledgement the app has to guess
    // and can only show a switch that silently flips back.

    // `-ensure-autostart`: create the silent elevated logon task.
    if args.iter().any(|a| a == "-ensure-autostart") {
        match mem_reduct_lib::autostart::install() {
            Ok(()) => {
                mem_reduct_lib::autostart::publish_result(mem_reduct_lib::autostart::RESULT_OK)
            }
            Err(detail) => mem_reduct_lib::autostart::publish_result(&format!("failed:{detail}")),
        }
        return;
    }

    // `-disable-autostart`: remove the logon task.
    if args.iter().any(|a| a == "-disable-autostart") {
        match mem_reduct_lib::autostart::uninstall() {
            Ok(()) => {
                mem_reduct_lib::autostart::publish_result(mem_reduct_lib::autostart::RESULT_OK)
            }
            Err(detail) => mem_reduct_lib::autostart::publish_result(&format!("failed:{detail}")),
        }
        return;
    }

    // --- Command-line cleanup: clean, then exit without ever showing the UI ---
    match cmdline::parse() {
        CommandLineAction::CleanDefault => run_cli_clean(None),
        CommandLineAction::CleanFull => run_cli_clean(Some(mem_reduct_lib::memory::mask::ALL)),
        CommandLineAction::CleanOnce(mask) => run_cli_clean(Some(mask)),
        CommandLineAction::None => {}
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

/// Perform a command-line cleanup and exit (never returns).
///
/// `-clean` / `-clean:full` / `-clean-once <mask>` all land here. When the
/// process is not elevated the same action is re-launched through the UAC
/// `runas` verb (the elevated copy performs the cleanup) and this process exits;
/// if the user declines the prompt we still attempt the cleanup un-elevated,
/// exactly like the original.
///
/// The exit code is meaningful for scripts: `0` when at least one region was
/// cleaned (or nothing was requested), `1` when every requested region failed —
/// previously this always exited `0`, so automation could not tell a successful
/// cleanup from a completely refused one.
fn run_cli_clean(mask: Option<u32>) -> ! {
    use mem_reduct_lib::{config, elevation, memory};

    let cfg = config::load();
    // `-clean-once <mask>` carries an explicit mask; `-clean` uses the
    // configured one and `-clean:full` passes `mask::ALL`.
    let mask = mask.unwrap_or(cfg.reduct_mask);

    if !elevation::is_elevated() && elevation::relaunch_with_args(&format!("-clean-once {mask}")) {
        std::process::exit(0);
    }

    let result = memory::clean_memory(mask, cfg.allow_standby_list_cleanup, false);
    println!(
        "Mem Reduct: cleaned {} region(s), freed {} bytes",
        result.regions.len().saturating_sub(result.failed.len()),
        result.freed_bytes
    );
    if !result.failed.is_empty() {
        eprintln!(
            "Mem Reduct: {} region(s) failed: {}",
            result.failed.len(),
            result.failed.join(", ")
        );
    }

    // Only a *complete* failure is worth reporting to the caller: a partial
    // cleanup is still a successful run (this is the "user declined the UAC
    // prompt and we cleaned what we could" path).
    let all_failed = !result.regions.is_empty() && result.failed.len() >= result.regions.len();
    std::process::exit(if all_failed { 1 } else { 0 });
}
