//! Inherit the language the user picked in the NSIS installer.
//!
//! Tauri's NSIS template defines `MUI_LANGDLL_REGISTRY_*` (so the language
//! picker can remember the choice) but never inserts `MUI_LANGDLL_SAVELANGUAGE`
//! — the value is only ever *read*. `nsis-lang/hooks.nsh` writes it back from the
//! `NSIS_HOOK_POSTINSTALL` hook; this module reads it, so a fresh install opens
//! in the language the user chose rather than the one the OS locale implies.
//!
//! The stored value is an NSIS language id, i.e. a Windows LCID in decimal
//! (`1031` for German).

use crate::config::Config;

/// `HKCU` subkey the installer writes to.
///
/// `installer.nsi` builds this as `Software\<manufacturer>\<product_name>`, and
/// `manufacturer` falls back to the second segment of `identifier` when
/// `bundle > publisher` is unset — hence `memreduct`. Setting `publisher` in
/// `tauri.conf.json` moves this key, so the two must change together. A drift
/// here fails soft (no language is inherited) rather than breaking anything.
const REG_SUBKEY: &str = r"Software\memreduct\Mem Reduct";

/// The value name, as defined by `MUI_LANGDLL_REGISTRY_VALUENAME` in the template.
const REG_VALUE: &str = "Installer Language";

/// Windows LCID → shipped language code.
///
/// Only ids the installer can actually produce are listed, plus the common
/// regional variants that collapse onto the same bundle (`en-GB` → `en-US`,
/// `pt-PT` → `pt-BR`). The unit tests keep this table in step with `LANGUAGES`.
const LCIDS: &[(u32, &str)] = &[
    (2052, "zh-CN"),  // SimpChinese
    (1028, "zh-TW"),  // TradChinese
    (3076, "zh-TW"),  // zh-HK
    (5124, "zh-TW"),  // zh-MO
    (1033, "en-US"),  // English
    (2057, "en-US"),  // en-GB
    (3081, "en-US"),  // en-AU
    (16393, "en-US"), // en-IN
    (1041, "ja-JP"),  // Japanese
    (1042, "ko-KR"),  // Korean
    (1031, "de-DE"),  // German
    (1036, "fr-FR"),  // French
    (1034, "es-ES"),  // Spanish
    (3082, "es-ES"),  // SpanishInternational
    (1046, "pt-BR"),  // PortugueseBR
    (2070, "pt-BR"),  // Portuguese (pt-PT)
    (1040, "it-IT"),  // Italian
    (1049, "ru-RU"),  // Russian
    (1045, "pl-PL"),  // Polish
    (1055, "tr-TR"),  // Turkish
    (1066, "vi-VN"),  // Vietnamese
    (1054, "th-TH"),  // Thai
    (1057, "id-ID"),  // Indonesian
];

/// Map an NSIS language id to one of our bundles, or `None` if we don't ship it.
pub fn language_for_lcid(lcid: u32) -> Option<&'static str> {
    LCIDS
        .iter()
        .find(|(id, _)| *id == lcid)
        .map(|(_, code)| *code)
}

/// NUL-terminated UTF-16 copy of `s`, for the `W` registry entry points.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The language the installer recorded, if it recorded one we ship.
pub fn installer_language() -> Option<&'static str> {
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    };

    let subkey = wide(REG_SUBKEY);
    let value = wide(REG_VALUE);
    // An LCID is a handful of digits; 32 UTF-16 units is ample headroom.
    let mut buf = [0u16; 32];
    let mut len = std::mem::size_of_val(&buf) as u32;

    // SAFETY: `subkey`/`value` outlive every call, `buf`/`len` describe a live
    // buffer whose capacity is `len` bytes, and the opened key is closed on
    // every path that leaves this block.
    let queried = unsafe {
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
            PCWSTR(value.as_ptr()),
            None,
            None,
            Some(buf.as_mut_ptr().cast::<u8>()),
            Some(&mut len),
        );
        let _ = RegCloseKey(hkey);
        status.0 == 0
    };
    if !queried {
        return None;
    }

    // `len` is a byte count and includes the terminating NUL; strip it (and any
    // padding) before parsing rather than trusting the exact length.
    let units = (len as usize / 2).min(buf.len());
    let raw = String::from_utf16_lossy(&buf[..units]);
    language_for_lcid(raw.trim_matches('\0').trim().parse::<u32>().ok()?)
}

/// Adopt the installer language for a brand-new config.
///
/// Called only while no `config.json` exists yet: on every later start the
/// user's in-app choice wins, so an upgrade that re-runs the installer can never
/// silently undo a language the user picked inside the app.
///
/// Returns `true` when `config` changed, so the caller can persist it and stop
/// consulting the registry on every subsequent start.
pub fn apply_first_run_language(config: &mut Config) -> bool {
    match installer_language() {
        Some(lang) if config.language != lang => {
            config.language = lang.to_string();
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_lcid_maps_to_nothing() {
        assert_eq!(language_for_lcid(0), None);
        assert_eq!(language_for_lcid(9999), None);
        // `English`'s LCID is 1033; 1032 is Greek and we ship no Greek bundle.
        assert_eq!(language_for_lcid(1032), None);
    }

    #[test]
    fn known_lcids_resolve() {
        assert_eq!(language_for_lcid(2052), Some("zh-CN"));
        assert_eq!(language_for_lcid(1031), Some("de-DE"));
        assert_eq!(language_for_lcid(1041), Some("ja-JP"));
        assert_eq!(language_for_lcid(1066), Some("vi-VN"));
    }

    #[test]
    fn regional_variants_collapse_onto_a_shipped_bundle() {
        assert_eq!(language_for_lcid(2057), Some("en-US")); // en-GB
        assert_eq!(language_for_lcid(3081), Some("en-US")); // en-AU
        assert_eq!(language_for_lcid(2070), Some("pt-BR")); // pt-PT
        assert_eq!(language_for_lcid(3076), Some("zh-TW")); // zh-HK
        assert_eq!(language_for_lcid(3082), Some("es-ES")); // es-419-ish
    }

    #[test]
    fn every_shipped_language_has_an_lcid() {
        // Guards the classic mistake: adding a language to `LANGUAGES` (and the
        // i18n bundles) but forgetting the installer mapping, which would make
        // the installer's choice silently ignored for that language.
        for code in crate::config::LANGUAGES {
            assert!(
                LCIDS.iter().any(|(_, c)| *c == code),
                "no NSIS LCID maps to {code}"
            );
        }
    }

    #[test]
    fn lcid_table_has_no_stale_languages() {
        // The reverse direction: a code here that is not a real language would
        // be written to the config and then silently rejected by `sanitize()`.
        for (lcid, code) in LCIDS {
            assert!(
                crate::config::LANGUAGES.contains(code),
                "LCID {lcid} maps to {code}, which is not in LANGUAGES"
            );
        }
    }

    #[test]
    fn lcid_table_has_no_duplicate_ids() {
        let mut seen = std::collections::HashSet::new();
        for (lcid, _) in LCIDS {
            assert!(seen.insert(*lcid), "duplicate LCID {lcid}");
        }
    }

    #[test]
    fn wide_string_is_nul_terminated() {
        assert_eq!(wide("ab"), vec![0x61, 0x62, 0x00]);
        assert_eq!(wide(""), vec![0x00]);
    }
}
