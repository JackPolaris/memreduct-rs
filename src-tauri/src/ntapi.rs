//! Undocumented / internal NT API bindings used for memory management.
//!
//! These functions are not part of the public Windows API but are stable and
//! widely used by system utilities (e.g. Sysinternals RAMMap). They are the
//! same low-level entry points the original Mem Reduct uses.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

use windows::Win32::Foundation::{HANDLE, NTSTATUS};

/// System information classes relevant to memory management.
///
/// These are a subset of `SYSTEM_INFORMATION_CLASS` (undocumented values).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemInformationClass {
    /// q: SYSTEM_BASIC_INFORMATION
    SystemBasicInformation = 0,
    /// q: SYSTEM_PERFORMANCE_INFORMATION
    SystemPerformanceInformation = 2,
    /// q: SYSTEM_PAGEFILE_INFORMATION
    SystemPageFileInformation = 18,
    /// q: SYSTEM_FILECACHE_INFORMATION (info for WorkingSetTypeSystemCache)
    SystemFileCacheInformation = 21,
    /// q: SYSTEM_MEMORY_LIST_INFORMATION; s: SYSTEM_MEMORY_LIST_COMMAND (requires SeProfileSingleProcessPrivilege)
    SystemMemoryListInformation = 80,
    /// q: SYSTEM_FILECACHE_INFORMATION (requires SeIncreaseQuotaPrivilege)
    SystemFileCacheInformationEx = 81,
    /// s: MEMORY_COMBINE_INFORMATION, MEMORY_COMBINE_INFORMATION_EX (win10+)
    SystemCombinePhysicalMemoryInformation = 130,
    /// s: NULL (requires admin) (flushes registry hives, win8.1+)
    SystemRegistryReconciliationInformation = 134,
}

/// `SYSTEM_MEMORY_LIST_COMMAND` — commands passed to `NtSetSystemInformation`
/// with `SystemMemoryListInformation`.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemMemoryListCommand {
    MemoryCaptureAccessedBits = 0,
    MemoryCaptureAndResetAccessedBits = 1,
    /// Empty all process working sets (vista+).
    MemoryEmptyWorkingSets = 2,
    /// Flush modified pages to disk (vista+).
    MemoryFlushModifiedList = 3,
    /// Purge standby list (all priorities) (vista+).
    MemoryPurgeStandbyList = 4,
    /// Purge standby priority-0 list (vista+).
    MemoryPurgeLowPriorityStandbyList = 5,
    MemoryCommandMax = 6,
}

/// `SYSTEM_BASIC_INFORMATION`.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SYSTEM_BASIC_INFORMATION {
    pub Reserved: u32,
    pub TimerResolution: u32,
    pub PageSize: u32,
    pub NumberOfPhysicalPages: u32,
    pub HighestPhysicalPageNumber: u32,
    pub LowestPhysicalPageNumber: u32,
    pub AllocationGranularity: u32,
    pub LowestUserAddress: usize,
    pub HighestUserAddress: usize,
    pub ActiveProcessorsAffinityMask: usize,
    pub NumberOfProcessors: u32,
}

/// `SYSTEM_FILECACHE_INFORMATION`.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SYSTEM_FILECACHE_INFORMATION {
    pub CurrentSize: usize,
    pub PeakSize: usize,
    pub PageFaultCount: u32,
    pub MinimumWorkingSet: usize,
    pub MaximumWorkingSet: usize,
    pub CurrentSizeIncludingTransitionInPages: usize,
    pub PeakSizeIncludingTransitionInPages: usize,
    pub TransitionRePurposeCount: u32,
    pub Flags: u32,
}

/// `MEMORY_COMBINE_INFORMATION_EX`.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct MEMORY_COMBINE_INFORMATION_EX {
    pub Handle: HANDLE,
    pub PagesCombined: usize,
    pub Flags: u32,
}

/// `SYSTEM_PAGEFILE_INFORMATION` (variable length structure).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SYSTEM_PAGEFILE_INFORMATION {
    pub NextEntryOffset: u32,
    pub TotalSize: u32,
    pub TotalInUse: u32,
    pub PeakUsage: u32,
    pub CurrentUsage: u32,
    pub FileName: [u16; 1], // UNICODE_STRING
}

extern "system" {
    /// `NtQuerySystemInformation` from ntdll.
    pub fn NtQuerySystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> NTSTATUS;

    /// `NtSetSystemInformation` from ntdll.
    pub fn NtSetSystemInformation(
        system_information_class: i32,
        system_information: *mut core::ffi::c_void,
        system_information_length: u32,
    ) -> NTSTATUS;

    /// `RtlGetVersion` from ntdll.
    pub fn RtlGetVersion(version_info: *mut RTL_OSVERSIONINFOW) -> NTSTATUS;
}

/// NTSTATUS success macro.
#[inline]
pub fn NT_SUCCESS(status: NTSTATUS) -> bool {
    status.0 >= 0
}

/// `RTL_OSVERSIONINFOW` used by `RtlGetVersion`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RTL_OSVERSIONINFOW {
    pub dwOSVersionInfoSize: u32,
    pub dwMajorVersion: u32,
    pub dwMinorVersion: u32,
    pub dwBuildNumber: u32,
    pub dwPlatformId: u32,
    pub szCSDVersion: [u16; 128],
}

impl Default for RTL_OSVERSIONINFOW {
    fn default() -> Self {
        Self {
            dwOSVersionInfoSize: core::mem::size_of::<RTL_OSVERSIONINFOW>() as u32,
            dwMajorVersion: 0,
            dwMinorVersion: 0,
            dwBuildNumber: 0,
            dwPlatformId: 0,
            szCSDVersion: [0; 128],
        }
    }
}
