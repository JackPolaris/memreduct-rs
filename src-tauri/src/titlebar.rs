//! Paint the *native* title bar to match the app's theme.
//!
//! The window deliberately keeps its native decorations: the system buttons
//! bring snap layouts, resize borders, the system menu and accessibility with
//! them, and a hand-rolled title bar only imitates those. What the OS cannot
//! know is our skin, so the caption is re-tinted through DWM instead.
//!
//! Colours arrive from the frontend, which resolves the CSS variables the skin
//! defines. That keeps the palette in exactly one place (`styles.css`) — Rust
//! never holds a second copy that could drift out of step with it.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};

/// `DWMWA_USE_IMMERSIVE_DARK_MODE` — Windows 10 1809 (17763) and later.
const USE_IMMERSIVE_DARK_MODE: u32 = 20;
/// `DWMWA_BORDER_COLOR` — Windows 11 (22000) and later.
const BORDER_COLOR: u32 = 34;
/// `DWMWA_CAPTION_COLOR` — Windows 11 (22000) and later.
const CAPTION_COLOR: u32 = 35;
/// `DWMWA_TEXT_COLOR` — Windows 11 (22000) and later.
const TEXT_COLOR: u32 = 36;

/// `#rrggbb` (what the frontend reads out of the CSS variables) to the
/// `0x00bbggrr` COLORREF that DWM expects.
///
/// Also accepts the `rgb(r, g, b)` / `rgba(r, g, b, a)` form: a custom property
/// usually keeps the token it was authored with, but a computed one can come
/// back resolved. Alpha has no meaning for a caption, so it is dropped.
pub fn parse_hex(value: &str) -> Option<u32> {
    let v = value.trim();
    let (r, g, b) = if let Some(rest) = v.strip_prefix("rgb(").or_else(|| v.strip_prefix("rgba(")) {
        let parts: Vec<u32> = rest
            .trim_end_matches(')')
            .split(',')
            .filter_map(|p| p.trim().parse::<f64>().ok().map(|n| n.round() as u32))
            .collect();
        if parts.len() < 3 {
            return None;
        }
        (parts[0], parts[1], parts[2])
    } else {
        let h = v.strip_prefix('#').unwrap_or(v);
        if h.len() != 6 {
            return None;
        }
        let n = u32::from_str_radix(h, 16).ok()?;
        ((n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff)
    };
    Some((r & 0xff) | ((g & 0xff) << 8) | ((b & 0xff) << 16))
}

/// Set one DWM attribute. Returns whether the OS accepted it: the colour
/// attributes only exist on Windows 11, and on Windows 10 they fail with
/// `E_INVALIDARG`, which is expected rather than an error to report.
fn set(hwnd: HWND, attribute: u32, value: u32) -> bool {
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(attribute as i32),
            std::ptr::addr_of!(value) as *const core::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
        .is_ok()
    }
}

/// Apply the theme to the window's caption.
///
/// Returns the names of the attributes the OS actually accepted, so the caller
/// can tell "the colour was applied" from "this Windows build ignored it"
/// without guessing.
pub fn apply(
    hwnd_raw: isize,
    dark: bool,
    caption: Option<&str>,
    text: Option<&str>,
) -> Vec<&'static str> {
    let hwnd = HWND(hwnd_raw as *mut core::ffi::c_void);
    let mut applied = Vec::new();

    // `BOOL` is an i32; DWM reads the attribute as a 4-byte integer either way.
    if set(hwnd, USE_IMMERSIVE_DARK_MODE, dark as u32) {
        applied.push("dark-mode");
    }
    if let Some(caption) = caption.and_then(parse_hex) {
        if set(hwnd, CAPTION_COLOR, caption) {
            applied.push("caption-color");
        }
        // The 1px frame around the window is part of the same surface; leaving
        // it at the system colour draws a bright line around a dark caption.
        if set(hwnd, BORDER_COLOR, caption) {
            applied.push("border-color");
        }
    }
    if let Some(text) = text.and_then(parse_hex) {
        if set(hwnd, TEXT_COLOR, text) {
            applied.push("text-color");
        }
    }

    applied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_hex_and_rgb() {
        assert_eq!(parse_hex("#0a0e14"), Some(0x0014_0e0a));
        assert_eq!(parse_hex("0a0e14"), Some(0x0014_0e0a));
        assert_eq!(parse_hex("  #FFFFFF "), Some(0x00FF_FFFF));
        assert_eq!(parse_hex("rgb(255, 255, 255)"), Some(0x00FF_FFFF));
        assert_eq!(parse_hex("rgba(0, 0, 0, 0.5)"), Some(0));
    }

    #[test]
    fn rejects_values_that_are_not_colours() {
        // Guarding the parse matters: a wrong value would paint the caption
        // black or white by accident, which looks like a rendering bug.
        assert_eq!(parse_hex(""), None);
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("var(--bg-grad-1)"), None);
        assert_eq!(parse_hex("rgb(1,2)"), None);
    }

    #[test]
    fn channel_order_is_colorref_not_rgb() {
        // DWM wants 0x00bbggrr. Pure red is 0x000000ff, not 0x00ff0000 — the
        // reversed order would silently swap red and blue in the caption.
        assert_eq!(parse_hex("#ff0000"), Some(0x0000_00ff));
        assert_eq!(parse_hex("#0000ff"), Some(0x00ff_0000));
    }
}
