//! Memory information gathering and cleanup operations.
//!
//! This reimplements the core of Mem Reduct: reading physical memory, page
//! file and system cache statistics via `NtQuerySystemInformation`, and
//! performing the cleanup via `NtSetSystemInformation`.

use crate::ntapi::*;
use windows::Win32::Foundation::HANDLE;

/// Memory cleaning mask bits (mirrors the original Mem Reduct `REDUCT_*`).
pub mod mask {
    /// Empty all process working sets.
    pub const WORKINGSET: u32 = 0x01;
    /// Compress system file cache.
    pub const SYSTEMFILECACHE: u32 = 0x02;
    /// Purge standby priority-0 list.
    pub const STANDBYPRIORITY0LIST: u32 = 0x04;
    /// Purge all standby lists.
    pub const STANDBYLIST: u32 = 0x08;
    /// Flush modified pages to disk.
    pub const MODIFIEDLIST: u32 = 0x10;
    /// Combine memory lists (win10+).
    pub const COMBINEMEMORYLISTS: u32 = 0x20;
    /// Flush registry cache (win8.1+).
    pub const REGISTRYCACHE: u32 = 0x40;
    /// Flush modified file cache.
    pub const MODIFIEDFILECACHE: u32 = 0x80;

    /// All cleanable regions.
    pub const ALL: u32 = WORKINGSET
        | SYSTEMFILECACHE
        | STANDBYPRIORITY0LIST
        | STANDBYLIST
        | MODIFIEDLIST
        | COMBINEMEMORYLISTS
        | REGISTRYCACHE
        | MODIFIEDFILECACHE;

    /// Default clean mask (excludes the "freezing" standby/modified lists).
    pub const DEFAULT: u32 = WORKINGSET
        | SYSTEMFILECACHE
        | STANDBYPRIORITY0LIST
        | REGISTRYCACHE
        | COMBINEMEMORYLISTS
        | MODIFIEDFILECACHE;

    /// Regions that can cause freezes (standby list + modified list).
    pub const FREEZES: u32 = STANDBYLIST | MODIFIEDLIST;

    /// Region keys (matching the frontend i18n `regions.*` keys) for display
    /// and notifications.
    ///
    /// The order MUST match `REGIONS` in `src/regions.ts` so a result list read
    /// by the UI is in the same order as the checkboxes the user ticked.
    pub fn names(value: u32) -> Vec<&'static str> {
        let mut out = Vec::new();
        if value & WORKINGSET != 0 {
            out.push("workingSet");
        }
        if value & SYSTEMFILECACHE != 0 {
            out.push("systemFileCache");
        }
        if value & STANDBYPRIORITY0LIST != 0 {
            out.push("standbyPriority0");
        }
        if value & STANDBYLIST != 0 {
            out.push("standbyList");
        }
        if value & MODIFIEDLIST != 0 {
            out.push("modifiedList");
        }
        if value & COMBINEMEMORYLISTS != 0 {
            out.push("combineMemoryLists");
        }
        if value & REGISTRYCACHE != 0 {
            out.push("registryCache");
        }
        if value & MODIFIEDFILECACHE != 0 {
            out.push("modifiedFileCache");
        }
        out
    }
}

/// A measured memory region (physical / page-file / system cache).
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct MemoryObject {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub percent: u32,
    pub percent_f: f64,
}

/// Aggregated memory snapshot exposed to the UI.
#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct MemoryInfo {
    pub physical_memory: MemoryObject,
    pub page_file: MemoryObject,
    pub system_cache: MemoryObject,
}

/// Percent helper (scaled the same way as the original `PR_CALC_PERCENTOF`).
fn calc_percent(used: u64, total: u64) -> (u32, f64) {
    if total == 0 {
        return (0, 0.0);
    }
    let p = (used as f64 / total as f64) * 100.0;
    (p as u32, p)
}

/// Cached OS version.
///
/// `RtlGetVersion` cannot change during a process' lifetime, but the cleanup
/// path used to call it twice per clean (via `is_win8_1_plus` /
/// `is_win10_plus`) on top of every memory sample.
static OS_VERSION: std::sync::OnceLock<(u32, u32)> = std::sync::OnceLock::new();

/// Query the system build version via `RtlGetVersion` (cached).
pub fn os_version() -> (u32, u32) {
    *OS_VERSION.get_or_init(|| unsafe {
        let mut vi: RTL_OSVERSIONINFOW = RTL_OSVERSIONINFOW {
            dwOSVersionInfoSize: core::mem::size_of::<RTL_OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        let status = RtlGetVersion(&mut vi);
        if NT_SUCCESS(status) {
            (vi.dwMajorVersion, vi.dwMinorVersion)
        } else {
            // fallback: assume Win10+ (matches common modern systems)
            (10, 0)
        }
    })
}

/// True on Windows 8.1+ (registry cache feature).
pub fn is_win8_1_plus() -> bool {
    let (major, minor) = os_version();
    (major > 6) || (major == 6 && minor >= 3)
}

/// True on Windows 10+ (combine memory lists feature).
pub fn is_win10_plus() -> bool {
    let (major, _) = os_version();
    major >= 10
}

/// Gather physical memory, page file and system cache information.
pub fn get_memory_info() -> MemoryInfo {
    let mut info = MemoryInfo::default();

    unsafe {
        // Physical memory (standard, reliable API).
        //
        // `GlobalMemoryStatusEx` returns the physical memory totals and the
        // current memory load as a percentage — this is the most robust source
        // and matches what other system utilities display. The previous
        // approach (reading a byte buffer from SystemPerformanceInformation)
        // could fail when the buffer was too small, leaving usage at 0%.
        let mut mem_status: windows::Win32::System::SystemInformation::MEMORYSTATUSEX =
            std::mem::zeroed();
        mem_status.dwLength = core::mem::size_of::<
            windows::Win32::System::SystemInformation::MEMORYSTATUSEX,
        >() as u32;
        if windows::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut mem_status).is_ok()
        {
            info.physical_memory.total_bytes = mem_status.ullTotalPhys;
            info.physical_memory.free_bytes = mem_status.ullAvailPhys;
            info.physical_memory.used_bytes = mem_status
                .ullTotalPhys
                .saturating_sub(mem_status.ullAvailPhys);
            info.physical_memory.percent = mem_status.dwMemoryLoad;
            info.physical_memory.percent_f = mem_status.dwMemoryLoad as f64;
        }

        // System file cache
        let mut sfci: SYSTEM_FILECACHE_INFORMATION = Default::default();
        let status = NtQuerySystemInformation(
            SystemInformationClass::SystemFileCacheInformation as i32,
            &mut sfci as *mut _ as *mut core::ffi::c_void,
            core::mem::size_of::<SYSTEM_FILECACHE_INFORMATION>() as u32,
            core::ptr::null_mut(),
        );
        if NT_SUCCESS(status) {
            info.system_cache.total_bytes = sfci.PeakSize as u64;
            info.system_cache.used_bytes = sfci.CurrentSize as u64;
            info.system_cache.free_bytes =
                (sfci.PeakSize as u64).saturating_sub(sfci.CurrentSize as u64);
            let (p, pf) = calc_percent(info.system_cache.used_bytes, info.system_cache.total_bytes);
            info.system_cache.percent = p;
            info.system_cache.percent_f = pf;
        }

        // Page file (needs the page size from SYSTEM_BASIC_INFORMATION).
        let mut basic: SYSTEM_BASIC_INFORMATION = Default::default();
        let status = NtQuerySystemInformation(
            SystemInformationClass::SystemBasicInformation as i32,
            &mut basic as *mut _ as *mut core::ffi::c_void,
            core::mem::size_of::<SYSTEM_BASIC_INFORMATION>() as u32,
            core::ptr::null_mut(),
        );
        if NT_SUCCESS(status) {
            info.page_file = read_pagefile_info(basic.PageSize as u64);
        }
    }

    info
}

/// Read page file usage. `SYSTEM_PAGEFILE_INFORMATION` is a variable-length
/// array terminated by `NextEntryOffset == 0`.
unsafe fn read_pagefile_info(page_size: u64) -> MemoryObject {
    let mut obj = MemoryObject::default();
    let mut buffer_length: u32 = 0x200;
    let mut attempts = 6;

    loop {
        let mut buf = vec![0u8; buffer_length as usize];
        let mut return_length = 0u32;
        let status = NtQuerySystemInformation(
            SystemInformationClass::SystemPageFileInformation as i32,
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            buffer_length,
            &mut return_length,
        );
        if status.0 == 0xC0000004u32 as i32 {
            // STATUS_INFO_LENGTH_MISMATCH
            buffer_length = buffer_length.checked_mul(2).unwrap_or(0x2000).max(0x2000);
            attempts -= 1;
            if attempts == 0 {
                break;
            }
            continue;
        }
        if !NT_SUCCESS(status) {
            break;
        }

        // Walk the linked list.
        let base = buf.as_ptr();
        let buf_len = buf.len();
        let mut offset: usize = 0;
        loop {
            // Guard against malformed offsets to avoid a panic on bad data.
            if offset >= buf_len {
                break;
            }
            let Some(entry) =
                (unsafe { (base.add(offset) as *const SYSTEM_PAGEFILE_INFORMATION).as_ref() })
            else {
                break;
            };
            obj.total_bytes += entry.TotalSize as u64 * page_size;
            // `TotalInUse` can exceed `TotalSize` in a torn sample; saturate
            // instead of underflowing (which panics in debug builds).
            obj.free_bytes += entry.TotalSize.saturating_sub(entry.TotalInUse) as u64 * page_size;
            obj.used_bytes += entry.TotalInUse as u64 * page_size;
            if entry.NextEntryOffset == 0 {
                break;
            }
            offset += entry.NextEntryOffset as usize;
        }

        let (p, pf) = calc_percent(obj.used_bytes, obj.total_bytes);
        obj.percent = p;
        obj.percent_f = pf;
        break;
    }

    obj
}

/// Result of a memory cleanup.
#[derive(Debug, serde::Serialize)]
pub struct CleanResult {
    /// Bytes freed (used memory difference).
    pub freed_bytes: u64,
    /// Mask that was actually applied.
    pub applied_mask: u32,
    /// Names of the regions that were cleaned.
    pub regions: Vec<String>,
    /// Region keys whose underlying NT call failed (`STATUS_*`).
    ///
    /// Surfaced to the UI so "freed 0 B" can be explained — without this the
    /// app silently swallowed `STATUS_PRIVILEGE_NOT_HELD` and looked broken.
    #[serde(default)]
    pub failed: Vec<String>,
}

/// Volumes to skip when flushing the modified file cache.
///
/// These are the drive types whose `CreateFileW` can block for seconds (a
/// spun-down optical drive, a mapped-but-disconnected network share) or fail
/// outright; flushing them is not worth freezing the UI for.
///
/// Values mirror `DRIVE_*` in `Win32::System::WindowsProgramming` (that feature
/// is not enabled, so they are declared locally).
const DRIVE_NO_ROOT_DIR: u32 = 1;
const DRIVE_REMOTE: u32 = 4;
const DRIVE_CDROM: u32 = 5;

/// Flush volume cache by opening each *existing* volume and calling
/// `FlushFileBuffers`.
///
/// This mirrors the original `_app_flushvolumecache` (modified file cache).
/// The original enumerates mount points; the previous implementation blindly
/// probed `\\?\A:` … `\\?\Z:`, which could stall the calling thread on media
/// that is not ready. `GetLogicalDrives` + `GetDriveTypeW` restrict the work to
/// volumes that actually exist and are safe to open synchronously.
///
/// Returns `false` when at least one volume could not be opened or flushed.
fn flush_volume_cache() -> bool {
    use windows::Win32::Foundation::{CloseHandle, GENERIC_WRITE};
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FlushFileBuffers, GetDriveTypeW, GetLogicalDrives, FILE_ATTRIBUTE_NORMAL,
        FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };

    let present = unsafe { GetLogicalDrives() };
    if present == 0 {
        // No bit set means the query itself failed; treat it as "nothing done".
        return true;
    }

    let mut ok = true;

    for index in 0..26u32 {
        if present & (1 << index) == 0 {
            continue;
        }

        let root = format!("{}:\\", (b'A' + index as u8) as char);
        let mut root_wide: Vec<u16> = root.encode_utf16().collect();
        root_wide.push(0);

        let drive_type = unsafe { GetDriveTypeW(windows::core::PCWSTR(root_wide.as_ptr())) };
        if matches!(drive_type, DRIVE_NO_ROOT_DIR | DRIVE_REMOTE | DRIVE_CDROM) {
            continue;
        }

        // `\\?\` bypasses path parsing so a volume handle is opened directly.
        let path = format!(r"\\?\{}", root);
        let mut wide: Vec<u16> = path.encode_utf16().collect();
        wide.push(0);

        unsafe {
            match CreateFileW(
                windows::core::PCWSTR(wide.as_ptr()),
                GENERIC_WRITE.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                HANDLE(std::ptr::null_mut()),
            ) {
                Ok(handle) => {
                    if FlushFileBuffers(handle).is_err() {
                        ok = false;
                    }
                    let _ = CloseHandle(handle);
                }
                // A volume we cannot open (e.g. no write access without admin)
                // counts as a failure so the UI can say which region did not run.
                Err(_) => ok = false,
            }
        }
    }

    ok
}

/// Perform a memory cleanup for the given mask.
///
/// `is_autoclean` removes the freezing regions if standby-list cleanup is not
/// allowed, exactly as the original does.
pub fn clean_memory(mask: u32, allow_standby_in_auto: bool, is_autoclean: bool) -> CleanResult {
    // Enable the SeProfileSingleProcessPrivilege / SeIncreaseQuotaPrivilege
    // privileges required by the NT memory calls (as the original does).
    crate::elevation::enable_memory_privileges();

    let mut applied_mask = mask;
    if is_autoclean && !allow_standby_in_auto {
        applied_mask &= !mask::FREEZES;
    }

    let before = get_memory_info().physical_memory.used_bytes;

    // Region keys whose NT call did not succeed. Collected instead of ignored
    // so the caller can distinguish "nothing to free" from "we were not allowed".
    let mut failed: Vec<String> = Vec::new();

    /// Record a failing region.
    fn note(failed: &mut Vec<String>, region: &str, success: bool) {
        if !success {
            failed.push(region.to_string());
        }
    }

    unsafe {
        // Working set (vista+)
        if applied_mask & mask::WORKINGSET != 0 {
            let ok = nt_set_memory_list(SystemMemoryListCommand::MemoryEmptyWorkingSets);
            note(&mut failed, "workingSet", ok);
        }

        // System file cache
        if applied_mask & mask::SYSTEMFILECACHE != 0 {
            let mut sfci = SYSTEM_FILECACHE_INFORMATION {
                MinimumWorkingSet: usize::MAX,
                MaximumWorkingSet: usize::MAX,
                ..Default::default()
            };
            let status = NtSetSystemInformation(
                SystemInformationClass::SystemFileCacheInformationEx as i32,
                &mut sfci as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<SYSTEM_FILECACHE_INFORMATION>() as u32,
            );
            note(&mut failed, "systemFileCache", NT_SUCCESS(status));
        }

        // Flush volume cache
        if applied_mask & mask::MODIFIEDFILECACHE != 0 {
            let ok = flush_volume_cache();
            note(&mut failed, "modifiedFileCache", ok);
        }

        // Modified page list
        if applied_mask & mask::MODIFIEDLIST != 0 {
            let ok = nt_set_memory_list(SystemMemoryListCommand::MemoryFlushModifiedList);
            note(&mut failed, "modifiedList", ok);
        }

        // Standby list
        if applied_mask & mask::STANDBYLIST != 0 {
            let ok = nt_set_memory_list(SystemMemoryListCommand::MemoryPurgeStandbyList);
            note(&mut failed, "standbyList", ok);
        }

        // Standby priority-0 list
        if applied_mask & mask::STANDBYPRIORITY0LIST != 0 {
            let ok = nt_set_memory_list(SystemMemoryListCommand::MemoryPurgeLowPriorityStandbyList);
            note(&mut failed, "standbyPriority0", ok);
        }

        // Flush registry cache (win8.1+)
        if is_win8_1_plus() && applied_mask & mask::REGISTRYCACHE != 0 {
            let status = NtSetSystemInformation(
                SystemInformationClass::SystemRegistryReconciliationInformation as i32,
                core::ptr::null_mut(),
                0,
            );
            note(&mut failed, "registryCache", NT_SUCCESS(status));
        }

        // Combine memory lists (win10+)
        if is_win10_plus() && applied_mask & mask::COMBINEMEMORYLISTS != 0 {
            let mut combine_info: MEMORY_COMBINE_INFORMATION_EX = Default::default();
            let status = NtSetSystemInformation(
                SystemInformationClass::SystemCombinePhysicalMemoryInformation as i32,
                &mut combine_info as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<MEMORY_COMBINE_INFORMATION_EX>() as u32,
            );
            note(&mut failed, "combineMemoryLists", NT_SUCCESS(status));
        }
    }

    let after = get_memory_info().physical_memory.used_bytes;
    let freed = before.saturating_sub(after);

    CleanResult {
        freed_bytes: freed,
        applied_mask,
        regions: mask::names(applied_mask)
            .into_iter()
            .map(String::from)
            .collect(),
        failed,
    }
}

/// Call `NtSetSystemInformation` with `SystemMemoryListInformation`.
///
/// Returns whether the command was accepted.
unsafe fn nt_set_memory_list(command: SystemMemoryListCommand) -> bool {
    let mut command = command as i32;
    let status = NtSetSystemInformation(
        SystemInformationClass::SystemMemoryListInformation as i32,
        &mut command as *mut i32 as *mut core::ffi::c_void,
        core::mem::size_of::<i32>() as u32,
    );
    NT_SUCCESS(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_memory_reported_sanity() {
        let info = get_memory_info();
        // On a real system the total physical memory is nonzero.
        assert!(info.physical_memory.total_bytes > 0, "total must be > 0");
        // used + free ≈ total; used bytes never exceed total.
        assert!(
            info.physical_memory.used_bytes <= info.physical_memory.total_bytes,
            "used bytes must not exceed total"
        );
        // percent in [0,100].
        assert!(
            (0..=100).contains(&info.physical_memory.percent),
            "percent out of range: {}",
            info.physical_memory.percent
        );
        // percent_f in [0,100].
        assert!(info.physical_memory.percent_f >= 0.0 && info.physical_memory.percent_f <= 100.0);

        // The memory-load percentage must reflect a real (nonzero) usage on a
        // running system; a stuck 0% was the reported bug.
        assert!(
            info.physical_memory.percent > 0,
            "physical memory percent should be > 0 on a running system, got 0%"
        );
        eprintln!(
            "physical: total={} used={} free={} percent={}",
            info.physical_memory.total_bytes,
            info.physical_memory.used_bytes,
            info.physical_memory.free_bytes,
            info.physical_memory.percent
        );
    }

    #[test]
    fn mask_default_excludes_freezes() {
        assert_eq!(mask::DEFAULT & mask::FREEZES, 0);
        assert_ne!(mask::DEFAULT & mask::WORKINGSET, 0);
        assert_ne!(mask::DEFAULT & mask::SYSTEMFILECACHE, 0);
    }

    #[test]
    fn mask_names_order_matches_frontend_regions() {
        // MUST stay identical to `REGIONS` in `src/regions.ts`, so a cleanup
        // result reads in the same order as the checkboxes the user ticked.
        assert_eq!(
            mask::names(mask::ALL),
            vec![
                "workingSet",
                "systemFileCache",
                "standbyPriority0",
                "standbyList",
                "modifiedList",
                "combineMemoryLists",
                "registryCache",
                "modifiedFileCache",
            ]
        );
        assert!(mask::names(0).is_empty());
    }

    #[test]
    fn clean_memory_applies_and_reports() {
        // Calling clean_memory may fail without admin, but it must not panic and
        // must honor the auto exclusion of the freezing regions.
        let result = clean_memory(mask::ALL, false, true);
        // Under autoclean disallow, freezing regions must be stripped.
        assert_eq!(result.applied_mask & mask::FREEZES, 0);
        // Region keys should reflect the applied mask (no freeze regions).
        assert!(!result
            .regions
            .iter()
            .any(|n| n == "standbyList" || n == "modifiedList"));

        // Full manual clean keeps all regions.
        let full = clean_memory(mask::ALL, true, false);
        assert_eq!(full.applied_mask, mask::ALL);
    }

    #[test]
    fn os_version_is_sane() {
        let (major, minor) = os_version();
        assert!(
            major >= 6,
            "expected at least Win7 (major>=6), got {major}.{minor}"
        );
    }
}
