//! Application configuration storage.
//!
//! Mirrors the original Mem Reduct behaviour: when a `memreduct.ini` marker
//! (here: `memreduct.json` in the executable directory) exists, the app runs
//! in *portable* mode and stores its config there. Otherwise config lives in
//! `%APPDATA%\Henry++\Mem Reduct`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Location where config is persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLocation {
    Portable,
    AppData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // General
    pub always_on_top: bool,
    pub start_minimized: bool,
    pub show_reduct_confirmation: bool,
    pub check_updates: bool,
    pub use_dark_theme: bool,
    pub language: String,

    // Memory / auto-clean
    pub autoreduct_enable: bool,
    pub autoreduct_value: u32,
    pub autoreduct_interval_enable: bool,
    pub autoreduct_interval_value: u32,
    pub allow_standby_list_cleanup: bool,
    pub reduct_mask: u32,

    // Hotkey
    pub hotkey_clean_enable: bool,
    pub hotkey_clean: u32,

    // Tray appearance
    pub tray_use_transparency: bool,
    pub tray_show_border: bool,
    pub tray_round_corners: bool,
    pub tray_change_bg: bool,
    pub tray_use_antialiasing: bool,
    pub tray_color_text: u32,
    pub tray_color_bg: u32,
    pub tray_color_warning: u32,
    pub tray_color_danger: u32,
    pub tray_font: String,

    // Tray behaviour
    pub tray_action_dc: u32, // double-click
    pub tray_action_mc: u32, // middle-click
    pub tray_level_warning: u32,
    pub tray_level_danger: u32,

    // Notifications
    pub notifications_sound: bool,
    pub balloon_clean_results: bool,
    pub log_clean_results: bool,

    // Statistics
    pub statistic_last_reduct: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            always_on_top: false,
            start_minimized: false,
            show_reduct_confirmation: true,
            check_updates: false,
            use_dark_theme: false,
            language: "en-US".into(),

            autoreduct_enable: false,
            autoreduct_value: 90,
            autoreduct_interval_enable: false,
            autoreduct_interval_value: 30,
            allow_standby_list_cleanup: false,
            reduct_mask: crate::memory::mask::DEFAULT,

            hotkey_clean_enable: false,
            hotkey_clean: 0x71, // VK_F1

            tray_use_transparency: false,
            tray_show_border: false,
            tray_round_corners: false,
            tray_change_bg: true,
            tray_use_antialiasing: false,
            tray_color_text: 0x00FFFFFF, // white
            tray_color_bg: 0x00008040,   // green
            tray_color_warning: 0x00FF8040,
            tray_color_danger: 0x00EC1C24,
            tray_font: "Lucida Console".into(),

            tray_action_dc: 0, // show
            tray_action_mc: 1, // clean
            tray_level_warning: 70,
            tray_level_danger: 90,

            notifications_sound: true,
            balloon_clean_results: true,
            log_clean_results: false,

            statistic_last_reduct: 0,
        }
    }
}

/// Portable marker file name (in the executable directory).
const PORTABLE_MARKER: &str = "memreduct.json";

/// AppData subfolder.
const APPDATA_SUBDIR: &str = "Henry++\\Mem Reduct";

fn executable_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn appdata_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join(APPDATA_SUBDIR)
    } else {
        // Fallback to home.
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(APPDATA_SUBDIR)
    }
}

/// Determine where config should live.
pub fn config_location() -> ConfigLocation {
    let portable = executable_dir().join(PORTABLE_MARKER);
    if portable.exists() {
        ConfigLocation::Portable
    } else {
        ConfigLocation::AppData
    }
}

fn config_path() -> PathBuf {
    match config_location() {
        ConfigLocation::Portable => executable_dir().join(PORTABLE_MARKER),
        ConfigLocation::AppData => appdata_dir().join("config.json"),
    }
}

/// Load config from disk; returns defaults if missing or malformed.
pub fn load() -> Config {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str::<Config>(&raw).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

/// Persist config to disk (creating directories as needed).
pub fn save(config: &Config) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(config)?;
    fs::write(&path, raw)
}

/// Convert a Win32 RGB value to a CSS-style hex string for the UI.
pub fn color_to_hex(rgb: u32) -> String {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_sane() {
        let c = Config::default();
        assert_eq!(c.autoreduct_value, 90);
        assert_eq!(c.autoreduct_interval_value, 30);
        assert_eq!(c.reduct_mask, crate::memory::mask::DEFAULT);
        assert!(c.show_reduct_confirmation);
        assert_eq!(c.tray_level_warning, 70);
        assert_eq!(c.tray_level_danger, 90);
    }

    #[test]
    fn serde_roundtrip() {
        let c = Config::default();
        let raw = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.autoreduct_value, c.autoreduct_value);
        assert_eq!(back.reduct_mask, c.reduct_mask);
    }

    #[test]
    fn color_conversion() {
        // White
        assert_eq!(color_to_hex(0x00FFFFFF), "#ffffff");
        // Green bg (0x008040 -> #008040)
        assert_eq!(color_to_hex(0x00008040), "#008040");
    }
}
