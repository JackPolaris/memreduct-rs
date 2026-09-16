//! Minimal `HKCU` registry helpers.
//!
//! Two modules need the same primitives, and both only ever touch a string
//! value under one key below `HKEY_CURRENT_USER`:
//!
//! * `installer_lang` reads the language the NSIS installer recorded;
//! * `autostart` publishes the outcome of the elevated helper back to the app.
//!
//! The raw API is a little awkward in Rust: `RegOpenKeyExW` / `RegCreateKeyExW`
//! / `RegQueryValueExW` / `RegSetValueExW` return `WIN32_ERROR`, which is a
//! newtype over `u32` rather than a `Result` — success is `.0 == 0`, and there
//! is no `?` to lean on. Wrapping them once keeps that detail out of the
//! callers.

use windows::core::PCWSTR;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

/// The `HKCU` subkey this application owns.
///
/// The NSIS installer builds it as `Software\<manufacturer>\<product_name>` and
/// `manufacturer` falls back to the second segment of `identifier` when
/// `bundle > publisher` is unset — hence `memreduct`. Setting `publisher` in
/// `tauri.conf.json` moves this key, so the two must change together; a drift
/// fails soft (nothing is inherited, no acknowledgement arrives) rather than
/// breaking anything.
pub const APP_SUBKEY: &str = r"Software\memreduct\Mem Reduct";

/// NUL-terminated UTF-16 copy of `s`, for the `W` registry entry points.
///
/// The `w!` macro cannot be used here: it only accepts string literals, and
/// every path this module handles is built at runtime.
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Largest `REG_SZ` payload [`read_string`] returns, in UTF-16 units.
///
/// Every value we store is a language id, a short status word or a diagnostic
/// message; 512 units is far beyond any of them and keeps this allocation-free.
const MAX_VALUE_UNITS: usize = 512;

/// Read the `REG_SZ` value `value` at `HKCU\<subkey>`.
///
/// `None` when the key is missing, when the value is missing or is not a
/// string, when it is empty, or when it does not fit — callers only ever need
/// to tell "absent" from "here is the string", so the reasons are deliberately
/// not distinguished.
pub fn read_string(subkey: &str, value: &str) -> Option<String> {
    let subkey = wide(subkey);
    let name = wide(value);
    let mut buf = [0u16; MAX_VALUE_UNITS];
    let mut len = std::mem::size_of_val(&buf) as u32;

    // SAFETY: both wide strings outlive every call; `buf`/`len` describe a live
    // buffer whose byte capacity is passed in `len`; the key handle is closed on
    // every path that leaves the block.
    let status = unsafe {
        let mut hkey = HKEY(std::ptr::null_mut());
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
        .0 != 0
        {
            return None;
        }
        let status = RegQueryValueExW(
            hkey,
            PCWSTR(name.as_ptr()),
            None,
            None,
            Some(buf.as_mut_ptr().cast::<u8>()),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        status
    };
    if status.0 != 0 {
        return None;
    }

    // `len` is a byte count and normally includes the terminating NUL, but a
    // value is not required to be NUL-terminated — clamp to the buffer and trim
    // rather than trusting it.
    let units = (len as usize / 2).min(buf.len());
    let raw = String::from_utf16_lossy(&buf[..units]);
    let trimmed = raw.trim_end_matches('\0');
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Write `data` as a `REG_SZ` value at `HKCU\<subkey>`, creating the key when
/// it does not exist yet. Returns `true` on success.
pub fn write_string(subkey: &str, value: &str, data: &str) -> bool {
    let subkey = wide(subkey);
    let name = wide(value);

    // `REG_SZ` data is the UTF-16 string plus its terminating NUL. The buffer
    // has to be materialised so its length can be reported to the API.
    let bytes: Vec<u8> = wide(data).iter().flat_map(|u| u.to_le_bytes()).collect();

    // SAFETY: the wide strings and the data buffer outlive every call; no
    // security attributes or disposition output are requested; the key handle is
    // closed on every path that leaves the block.
    unsafe {
        let mut hkey = HKEY(std::ptr::null_mut());
        if RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
        .0 != 0
        {
            return false;
        }
        let status = RegSetValueExW(hkey, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(&bytes));
        let _ = RegCloseKey(hkey);
        status.0 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wide_string_is_nul_terminated() {
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0x00]);
        assert_eq!(wide(""), vec![0x00]);
    }

    #[test]
    fn app_subkey_matches_the_installer_layout() {
        // `installer.nsi` derives this from `publisher` + `productName`; the
        // test documents the coupling so a rename in `tauri.conf.json` is not
        // silently missed here.
        assert_eq!(APP_SUBKEY, r"Software\memreduct\Mem Reduct");
    }
}
