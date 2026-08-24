//! Memory information gathering and cleanup operations.
//!
//! This reimplements the core of Mem Reduct: reading physical memory, page
//! file and system cache statistics via `NtQuerySystemInformation`, and
//! performing the cleanup via `NtSetSystemInformation`.

use crate::ntapi::*;
use windows::Win32::Foundation::{HANDLE, NTSTATUS};

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

    /// Region names for display / notifications.
    /// Region keys (matching the frontend i18n `regions.*` keys) for display.
    pub fn names(value: u32) -> Vec<&'static str> {
        let mut out = Vec::new();
        if value & WORKINGSET != 0 {
            out.push("workingSet");
        }
        if value & SYSTEMFILECACHE != 0 {
            out.push("systemFileCache");
        }
        if value & MODIFIEDFILECACHE != 0 {
            out.push("modifiedFileCache");
        }
        if value & MODIFIEDLIST != 0 {
            out.push("modifiedList");
        }
        if value & STANDBYLIST != 0 {
            out.push("standbyList");
        }
        if value & STANDBYPRIORITY0LIST != 0 {
            out.push("standbyPriority0");
        }
        if value & REGISTRYCACHE != 0 {
            out.push("registryCache");
        }
        if value & COMBINEMEMORYLISTS != 0 {
            out.push("combineMemoryLists");
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

/// Query the system build version via `RtlGetVersion`.
pub fn os_version() -> (u32, u32) {
    unsafe {
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
    }
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
        if windows::Win32::System::SystemInformation::GlobalMemoryStatusEx(&mut mem_status)
            .is_ok()
        {
            info.physical_memory.total_bytes = mem_status.ullTotalPhys;
            info.physical_memory.free_bytes = mem_status.ullAvailPhys;
            info.physical_memory.used_bytes =
                mem_status.ullTotalPhys.saturating_sub(mem_status.ullAvailPhys);
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
        let mut offset: usize = 0;
        let mut first = true;
        loop {
            let entry = (base.add(offset) as *const SYSTEM_PAGEFILE_INFORMATION)
                .as_ref()
                .unwrap();
            if entry.NextEntryOffset == 0 && !first {
                break;
            }
            obj.total_bytes += entry.TotalSize as u64 * page_size;
            obj.free_bytes += (entry.TotalSize - entry.TotalInUse) as u64 * page_size;
            obj.used_bytes += entry.TotalInUse as u64 * page_size;
            first = false;
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
}

/// Flush volume cache by opening each volume and calling `FlushFileBuffers`.
///
/// This mirrors the original `_app_flushvolumecache` (modified file cache).
fn flush_volume_cache() {
    // The original implementation enumerates mount points and flushes each
    // volume. For our clean implementation we enumerate drive letters and
    // flush their root handles.
    let letters = [
        'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R',
        'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
    ];

    for letter in letters {
        let path = format!(r"\\?\{letter}:\");
        if let Some(c) = wide(path.as_str()) {
            unsafe {
                let handle = windows::Win32::Storage::FileSystem::CreateFileW(
                    windows::core::PCWSTR(c.as_ptr()),
                    windows::Win32::Foundation::GENERIC_WRITE.0,
                    windows::Win32::Storage::FileSystem::FILE_SHARE_READ
                        | windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
                    None,
                    windows::Win32::Storage::FileSystem::OPEN_EXISTING,
                    windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
                    HANDLE(std::ptr::null_mut()),
                );
                if let Ok(h) = handle {
                    let _ = windows::Win32::Storage::FileSystem::FlushFileBuffers(h);
                    let _ = windows::Win32::Foundation::CloseHandle(h);
                }
            }
        }
    }
}

fn wide(s: &str) -> Option<Vec<u16>> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    Some(v)
}

/// Perform a memory cleanup for the given mask.
///
/// `is_autoclean` removes the freezing regions if standby-list cleanup is not
/// allowed, exactly as the original does.
pub fn clean_memory(mask: u32, allow_standby_in_auto: bool, is_autoclean: bool) -> CleanResult {
    // Already elevated? We rely on the app requesting admin at startup. If not
    // elevated the OS will reject the NtSetSystemInformation calls; we still
    // attempt and report results.

    // Enable the SeProfileSingleProcessPrivilege / SeIncreaseQuotaPrivilege
    // privileges required by the NT memory calls (as the original does).
    crate::elevation::enable_memory_privileges();

    let mut applied_mask = mask;
    if is_autoclean && !allow_standby_in_auto {
        applied_mask &= !mask::FREEZES;
    }

    let before = get_memory_info().physical_memory.used_bytes;

    unsafe {
        // Working set (vista+)
        if applied_mask & mask::WORKINGSET != 0 {
            let mut command = SystemMemoryListCommand::MemoryEmptyWorkingSets as i32;
            let _ = nt_set_memory_list(&mut command);
        }

        // System file cache
        if applied_mask & mask::SYSTEMFILECACHE != 0 {
            let mut sfci: SYSTEM_FILECACHE_INFORMATION = Default::default();
            sfci.MinimumWorkingSet = usize::MAX;
            sfci.MaximumWorkingSet = usize::MAX;
            let _ = NtSetSystemInformation(
                SystemInformationClass::SystemFileCacheInformationEx as i32,
                &mut sfci as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<SYSTEM_FILECACHE_INFORMATION>() as u32,
            );
        }

        // Flush volume cache
        if applied_mask & mask::MODIFIEDFILECACHE != 0 {
            flush_volume_cache();
        }

        // Modified page list
        if applied_mask & mask::MODIFIEDLIST != 0 {
            let mut command = SystemMemoryListCommand::MemoryFlushModifiedList as i32;
            let _ = nt_set_memory_list(&mut command);
        }

        // Standby list
        if applied_mask & mask::STANDBYLIST != 0 {
            let mut command = SystemMemoryListCommand::MemoryPurgeStandbyList as i32;
            let _ = nt_set_memory_list(&mut command);
        }

        // Standby priority-0 list
        if applied_mask & mask::STANDBYPRIORITY0LIST != 0 {
            let mut command = SystemMemoryListCommand::MemoryPurgeLowPriorityStandbyList as i32;
            let _ = nt_set_memory_list(&mut command);
        }

        // Flush registry cache (win8.1+)
        if is_win8_1_plus() && applied_mask & mask::REGISTRYCACHE != 0 {
            let _ = NtSetSystemInformation(
                SystemInformationClass::SystemRegistryReconciliationInformation as i32,
                core::ptr::null_mut(),
                0,
            );
        }

        // Combine memory lists (win10+)
        if is_win10_plus() && applied_mask & mask::COMBINEMEMORYLISTS != 0 {
            let mut combine_info: MEMORY_COMBINE_INFORMATION_EX = Default::default();
            let status = NtSetSystemInformation(
                SystemInformationClass::SystemCombinePhysicalMemoryInformation as i32,
                &mut combine_info as *mut _ as *mut core::ffi::c_void,
                core::mem::size_of::<MEMORY_COMBINE_INFORMATION_EX>() as u32,
            );
            let _ = status;
        }
    }

    let after = get_memory_info().physical_memory.used_bytes;
    let freed = before.saturating_sub(after);

    CleanResult {
        freed_bytes: freed,
        applied_mask,
        regions: mask::names(applied_mask).into_iter().map(String::from).collect(),
    }
}

/// Call `NtSetSystemInformation` with `SystemMemoryListInformation`.
unsafe fn nt_set_memory_list(command: &mut i32) -> NTSTATUS {
    NtSetSystemInformation(
        SystemInformationClass::SystemMemoryListInformation as i32,
        command as *mut i32 as *mut core::ffi::c_void,
        core::mem::size_of::<i32>() as u32,
    )
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
    fn clean_memory_applies_and_reports() {
        // Calling clean_memory may fail without admin, but it must not panic and
        // must honor the auto exclusion of the freezing regions.
        let result = clean_memory(mask::ALL, false, true);
        // Under autoclean disallow, freezing regions must be stripped.
        assert_eq!(result.applied_mask & mask::FREEZES, 0);
        // Region keys should reflect the applied mask (no freeze regions).
        assert!(
            !result
                .regions
                .iter()
                .any(|n| n == &"standbyList" || n == &"modifiedList")
        );

        // Full manual clean keeps all regions.
        let full = clean_memory(mask::ALL, true, false);
        assert_eq!(full.applied_mask, mask::ALL);
    }

    #[test]
    fn os_version_is_sane() {
        let (major, minor) = os_version();
        assert!(major >= 6, "expected at least Win7 (major>=6), got {major}.{minor}");
    }
}
