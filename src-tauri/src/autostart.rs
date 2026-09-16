//! Silent autostart + permanent elevation via Windows Task Scheduler.
//!
//! The scheduled task "Mem Reduct" runs at user logon with highest privileges
//! (`HighestAvailable`), so the app starts silently as administrator on every
//! boot with no UAC prompt. The *only* UAC prompt the user ever sees is the
//! single one needed to create the task the first time.
//!
//! All `schtasks.exe` invocations use `CREATE_NO_WINDOW` so no console window
//! ever flashes on screen (production polish).
//!
//! # Why the task is defined by an XML file
//!
//! `schtasks /create` cannot express a correct logon task for a long-running
//! tray app. It offers **no switch at all** for the two settings that matter
//! most, and its defaults are actively wrong here:
//!
//! * `DisallowStartIfOnBatteries` / `StopIfGoingOnBatteries` default to `true`,
//!   so on a laptop the task never starts while on battery and the running app
//!   is *terminated* the moment the charger is unplugged;
//! * `ExecutionTimeLimit` defaults to 72 hours, so Task Scheduler kills the
//!   process three days after logon — for a resident tray app that is a
//!   guaranteed failure, just a slow one.
//!
//! Both can only be set through the task XML (`schtasks /create /tn … /xml …`),
//! which is why [`task_xml`] exists. It must be written as **UTF-16LE with a
//! BOM**: that is the encoding `schtasks` itself emits and the one it accepts.
//!
//! # Why the target path is recorded in the registry
//!
//! [`task_state`] has to answer "is the task ours?", not merely "does a task
//! with this name exist?" — otherwise a leftover task from an install into a
//! different directory makes the settings switch claim autostart is on while
//! nothing starts at logon.
//!
//! Reading the path back out of `schtasks` would mean parsing its output, and
//! that is not safe here: the XML it prints when redirected is **single-byte**
//! (no BOM, despite the `encoding="UTF-16"` declaration) in the console
//! codepage, so an install path containing non-ASCII characters — the norm for
//! a per-user install under `C:\Users\<name>\` — can come back mangled and look
//! like a foreign task. The path is therefore stored as a `REG_SZ` value
//! alongside the task, which is encoding-safe by construction.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::registry::{self, APP_SUBKEY};

/// `CREATE_NO_WINDOW` — never show a console window for schtasks.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The scheduled task name.
pub const TASK_NAME: &str = "Mem Reduct";

/// The argument the task passes so the app knows it was launched at startup
/// (used to start minimized to the tray instead of opening the window).
pub const STARTUP_ARG: &str = "-startup";

/// `HKCU` value holding the executable path the task was created for.
const TARGET_VALUE: &str = "Autostart Target";

/// `HKCU` value carrying the outstanding [`publish_result`] acknowledgement.
const RESULT_VALUE: &str = "Autostart Result";

/// Marker written before the elevated helper is launched. The helper overwrites
/// it with [`RESULT_OK`] or a `failed:…` string, so a reader that still sees
/// this value knows the helper has not reported yet.
pub const RESULT_PENDING: &str = "pending";

/// Written by the helper (or directly by the app) once the task was changed.
pub const RESULT_OK: &str = "ok";

/// Run `schtasks.exe` with the given args, hiding any console window.
fn schtasks(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("schtasks.exe")
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
}

/// Existence of the task, regardless of what it launches.
///
/// `schtasks /query` reports success or failure through its exit code alone,
/// which is language-independent — the human-readable text is localised and
/// deliberately not inspected.
fn task_exists() -> bool {
    matches!(schtasks(&["/query", "/tn", TASK_NAME]), Ok(o) if o.status.success())
}

/// Path this executable lives at, as the task must spell it.
fn current_exe() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Compare two Windows paths the way the shell would: case-insensitively, with
/// either separator, and ignoring the `\\?\` prefix `canonicalize` may add.
fn same_path(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.trim_start_matches(r"\\?\")
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    !a.is_empty() && norm(a) == norm(b)
}

/// What [`task_state`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    /// The task exists *and* was created for this executable.
    pub enabled: bool,
    /// The task exists but is not ours — a leftover from an install into another
    /// directory, or a same-named task from a different program.
    pub stale: bool,
}

/// Classify the existing logon task.
pub fn task_state() -> TaskState {
    if !task_exists() {
        return TaskState {
            enabled: false,
            stale: false,
        };
    }
    let recorded = registry::read_string(APP_SUBKEY, TARGET_VALUE);
    let ours = match (&recorded, current_exe()) {
        (Some(recorded), Some(exe)) => same_path(recorded, &exe),
        _ => false,
    };
    TaskState {
        enabled: ours,
        stale: !ours,
    }
}

/// Escape the five characters that cannot appear literally in XML text.
fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

/// The complete task definition handed to `schtasks /create /xml`.
///
/// `sid` scopes both the principal and the logon trigger to the current user:
/// without it the trigger fires for *any* user's logon and Task Scheduler then
/// logs a failure every time somebody else signs in, because an
/// `InteractiveToken` principal cannot run unless that user is logged on.
fn task_xml(exe_path: &str, sid: &str) -> String {
    let exe = xml_escape(exe_path);
    let sid = xml_escape(sid);
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Mem Reduct autostart: starts elevated and silent at logon. Managed by the app.</Description>
    <URI>\{name}</URI>
  </RegistrationInfo>
  <Principals>
    <Principal id="Author">
      <UserId>{sid}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>false</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{sid}</UserId>
    </LogonTrigger>
  </Triggers>
  <Actions Context="Author">
    <Exec>
      <Command>"{exe}"</Command>
      <Arguments>{STARTUP_ARG}</Arguments>
    </Exec>
  </Actions>
</Task>
"#,
        name = xml_escape(TASK_NAME),
    )
}

/// Write `xml` where `schtasks` will accept it.
///
/// UTF-16LE with a BOM, matching what `schtasks /query /xml` produces. A
/// single-byte file is rejected.
fn write_task_xml(path: &Path, xml: &str) -> std::io::Result<()> {
    let mut bytes = Vec::with_capacity(xml.len() * 2 + 2);
    bytes.extend_from_slice(&[0xff, 0xfe]);
    for unit in xml.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

/// A private path for the task definition.
///
/// The process writing this may be elevated while `%TEMP%` stays user-writable,
/// so the name is not predictable: a guessable path in a shared directory is a
/// symlink-swap foothold. The file is removed as soon as `schtasks` has read it.
fn task_xml_path() -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        ^ ((std::process::id() as u128) << 64)
        ^ (SEQ.fetch_add(1, Ordering::Relaxed) as u128);
    std::env::temp_dir().join(format!("mem-reduct-{unique:032x}.xml"))
}

/// Format a raw `SID` structure as its SDDL string (`S-1-5-21-…`).
///
/// Layout is fixed by the platform: revision, sub-authority count, a 6-byte
/// big-endian identifier authority, then that many little-endian `u32`
/// sub-authorities.
fn sid_to_string(sid: &[u8]) -> Option<String> {
    if sid.len() < 8 {
        return None;
    }
    let revision = sid[0];
    let count = sid[1] as usize;
    if sid.len() < 8 + count * 4 {
        return None;
    }
    let mut authority: u64 = 0;
    for byte in &sid[2..8] {
        authority = (authority << 8) | u64::from(*byte);
    }
    let mut out = format!("S-{revision}-{authority}");
    for i in 0..count {
        let base = 8 + i * 4;
        let word = u32::from_le_bytes([sid[base], sid[base + 1], sid[base + 2], sid[base + 3]]);
        out.push_str(&format!("-{word}"));
    }
    Some(out)
}

/// The SID of the user this process runs as, in SDDL form.
///
/// `ConvertSidToStringSidW` sits behind a `windows` crate feature this project
/// does not enable, so the SID is formatted from its raw bytes instead — the
/// struct is a stable, documented layout and [`sid_to_string`] is unit-tested.
fn current_user_sid() -> Option<String> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: the token handle is closed before returning; `buf` is a live,
    // 8-byte-aligned, more-than-large-enough buffer whose byte size is passed
    // to the API, and `len` is written by it.
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }
        // `TOKEN_USER` is a pointer plus a `DWORD`; declared as `u64` words so
        // the read below is aligned rather than a possibly unaligned `u8` read.
        let mut buf = [0u64; 16];
        let mut len = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenUser,
            Some(buf.as_mut_ptr().cast()),
            std::mem::size_of_val(&buf) as u32,
            &mut len,
        );
        let _ = CloseHandle(token);
        ok.ok()?;

        let user = &*(buf.as_ptr().cast::<TOKEN_USER>());
        let sid_ptr = user.User.Sid.0.cast::<u8>();
        if sid_ptr.is_null() {
            return None;
        }
        // Every SID is at least 8 bytes, so reading the count at offset 1 is
        // always in bounds; the slice is then the exact SID length.
        let count = *sid_ptr.add(1) as usize;
        sid_to_string(std::slice::from_raw_parts(sid_ptr, 8 + count * 4))
    }
}

/// Create (or update) the logon task that runs this executable elevated and
/// silently at every logon. Requires elevation — call from an elevated context
/// (the helper process).
pub fn install() -> Result<(), String> {
    let exe = current_exe().ok_or("无法确定程序自身路径")?;
    let sid = current_user_sid().ok_or("无法获取当前用户 SID")?;

    let path = task_xml_path();
    write_task_xml(&path, &task_xml(&exe, &sid)).map_err(|e| format!("写入任务定义失败: {e}"))?;

    let path_str = path.to_string_lossy().to_string();
    let out = schtasks(&["/create", "/tn", TASK_NAME, "/xml", path_str.as_str(), "/f"]);
    // The definition has been consumed either way; never leave it behind.
    let _ = std::fs::remove_file(&path);

    match out {
        Ok(o) if o.status.success() => {
            // Recorded only after the task exists, so a failed creation cannot
            // leave a target behind that would make `task_state` lie.
            registry::write_string(APP_SUBKEY, TARGET_VALUE, &exe);
            Ok(())
        }
        Ok(o) => Err(format!(
            "schtasks /create 退出码: {}",
            o.status.code().unwrap_or(-1)
        )),
        Err(e) => Err(format!("无法运行 schtasks: {e}")),
    }
}

/// Remove the scheduled task (disables autostart). Requires elevation.
pub fn uninstall() -> Result<(), String> {
    let out = schtasks(&["/delete", "/tn", TASK_NAME, "/f"]).map_err(|e| e.to_string())?;
    if out.status.success() {
        // Only clear the record once the task is really gone: if the delete was
        // refused the task is still ours, and pretending otherwise would make
        // the settings switch report a foreign task.
        registry::write_string(APP_SUBKEY, TARGET_VALUE, "");
        Ok(())
    } else {
        Err(format!(
            "schtasks /delete 退出码: {}",
            out.status.code().unwrap_or(-1)
        ))
    }
}

/// Publish the outcome of an elevated helper so the app can report it.
///
/// The helper is a separate process that exits immediately, so this registry
/// value is the only channel back: the app polls it and shows a real error
/// instead of a switch that silently flips back.
pub fn publish_result(result: &str) {
    registry::write_string(APP_SUBKEY, RESULT_VALUE, result);
}

/// The outstanding acknowledgement, if any.
pub fn task_result() -> Option<String> {
    registry::read_string(APP_SUBKEY, RESULT_VALUE)
}

/// Mark that an elevated helper has been launched and has not reported yet.
pub fn mark_pending() {
    publish_result(RESULT_PENDING);
}

/// True when the process was launched by the scheduled task (`-startup` arg).
pub fn is_startup_launch() -> bool {
    std::env::args().any(|a| a == STARTUP_ARG)
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

    #[test]
    fn task_xml_neutralises_every_schtasks_default() {
        let xml = task_xml(
            r"C:\Program Files\Mem Reduct\mem-reduct.exe",
            "S-1-5-21-1-2-3-1001",
        );

        // The defaults that break a logon task for a resident tray app.
        assert!(xml.contains("<DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>"));
        assert!(xml.contains("<StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>"));
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"));
        // A missed logon start (machine asleep, or the task edited later) should
        // still run once the condition clears.
        assert!(xml.contains("<StartWhenAvailable>true</StartWhenAvailable>"));
        assert!(xml.contains("<MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>"));
        // HighestAvailable is what makes the logon start elevated with no UAC.
        assert!(xml.contains("<RunLevel>HighestAvailable</RunLevel>"));
        // The trigger must be scoped to this user, not "any user".
        assert!(xml.contains("<LogonTrigger>"));
        assert!(xml.contains("<UserId>S-1-5-21-1-2-3-1001</UserId>"));
        // Quoted because the install directory contains a space.
        assert!(xml.contains(r#"<Command>"C:\Program Files\Mem Reduct\mem-reduct.exe"</Command>"#));
        assert!(xml.contains("<Arguments>-startup</Arguments>"));
    }

    #[test]
    fn task_xml_escapes_metacharacters_in_the_path() {
        let xml = task_xml(r"C:\A&B\a<b>c\d.exe", "S-1-5-21-1");
        assert!(xml.contains(r#"<Command>"C:\A&amp;B\a&lt;b&gt;c\d.exe"</Command>"#));
        // The raw characters must be gone, or the XML would be malformed.
        assert!(!xml.contains("A&B"));
        assert!(!xml.contains("a<b>c"));
    }

    #[test]
    fn task_xml_declares_the_encoding_schtasks_demands() {
        assert!(
            task_xml("x", "S-1-5-21-1").starts_with(r#"<?xml version="1.0" encoding="UTF-16"?>"#)
        );
    }

    #[test]
    fn task_xml_write_is_utf16le_with_a_bom() {
        let dir = std::env::temp_dir();
        let path = dir.join("mem-reduct-encoding-test.xml");
        write_task_xml(&path, "<a>é</a>").expect("write");
        let bytes = std::fs::read(&path).expect("read");
        let _ = std::fs::remove_file(&path);

        assert_eq!(&bytes[..2], &[0xff, 0xfe], "BOM");
        // `<a>` then a non-ASCII char: two bytes per unit, little-endian.
        assert_eq!(&bytes[2..8], &[0x3c, 0x00, 0x61, 0x00, 0x3e, 0x00]);
        assert_eq!(&bytes[8..10], &[0xe9, 0x00]);
        assert_eq!(bytes.len() % 2, 0);
    }

    #[test]
    fn sid_formatting_matches_the_sddl_layout() {
        // A real user SID, values and all: S-1-5-21-<machine>-<machine>-<machine>-<rid>.
        // The machine components overflow `u16`, so this also pins the fact that
        // the six-byte identifier authority and the `u32` sub-authorities are not
        // truncated — a mangled SID would make `schtasks` reject the definition.
        let mut raw = vec![1u8, 5u8];
        raw.extend_from_slice(&[0, 0, 0, 0, 0, 5]); // identifier authority, big-endian
        for sub in [21u32, 2145379151, 409696732, 3204728361, 1001] {
            raw.extend_from_slice(&sub.to_le_bytes());
        }
        assert_eq!(
            sid_to_string(&raw).as_deref(),
            Some("S-1-5-21-2145379151-409696732-3204728361-1001")
        );
    }

    #[test]
    fn sid_formatting_handles_an_empty_sub_authority_list() {
        // Well-known SIDs such as the local authority carry no sub-authorities.
        assert_eq!(
            sid_to_string(&[1, 0, 0, 0, 0, 0, 0, 5]).as_deref(),
            Some("S-1-5")
        );
    }

    #[test]
    fn sid_formatting_rejects_truncated_input() {
        assert_eq!(sid_to_string(&[]), None);
        // Claims two sub-authorities but carries only one.
        let mut raw = vec![1u8, 2u8, 0, 0, 0, 0, 0, 5];
        raw.extend_from_slice(&7u32.to_le_bytes());
        assert_eq!(sid_to_string(&raw), None);
    }

    #[test]
    fn path_comparison_ignores_case_separators_and_prefix() {
        assert!(same_path(
            r"C:\Users\a\Mem Reduct\mem-reduct.exe",
            r"c:/users/A/MEM REDUCT/mem-reduct.exe"
        ));
        assert!(same_path(r"\\?\C:\x\y.exe", r"C:\x\y.exe"));
        assert!(!same_path(r"C:\x\y.exe", r"C:\x\z.exe"));
        // An absent record must never read as "ours".
        assert!(!same_path("", r"C:\x\y.exe"));
    }

    #[test]
    fn task_xml_paths_are_unique_and_not_guessable() {
        let a = task_xml_path();
        let b = task_xml_path();
        assert_ne!(a, b);
        assert_eq!(a.extension().and_then(|e| e.to_str()), Some("xml"));
        assert!(a.parent().is_some());
    }
}
