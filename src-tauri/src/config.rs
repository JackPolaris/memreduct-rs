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
    pub start_minimized: bool,
    pub show_reduct_confirmation: bool,
    // Theme: "light" | "dark" | "system"
    pub theme: String,
    // Accent color preset key (e.g. "green", "purple", "blue", ...).
    pub accent_color: String,
    // Legacy dark-theme flag (kept for migration from older configs).
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
    pub tray_color_text: u32,
    pub tray_color_bg: u32,
    pub tray_color_warning: u32,
    pub tray_color_danger: u32,

    // Tray behaviour
    pub tray_action_dc: u32, // double-click
    pub tray_action_mc: u32, // middle-click
    pub tray_level_warning: u32,
    pub tray_level_danger: u32,
    /// Whether the one-off "still running in the tray" hint has been shown.
    pub tray_tip_shown: bool,

    // Notifications
    pub balloon_clean_results: bool,

    // Statistics
    pub statistic_last_reduct: i64,
}

/// Theme values accepted by the UI (`light` | `dark` | `system`).
pub const THEMES: [&str; 3] = ["light", "dark", "system"];

/// Accent preset keys — MUST stay in sync with `src/accents.ts`.
pub const ACCENT_KEYS: [&str; 7] = ["green", "purple", "blue", "orange", "red", "cyan", "pink"];

/// Locales that actually ship a translation bundle — MUST stay in sync with
/// `src/i18n/index.ts`.
pub const LANGUAGES: [&str; 4] = ["zh-CN", "zh-TW", "en-US", "ja-JP"];

/// Tray click actions.
pub const TRAY_ACTION_SHOW: u32 = 0;
pub const TRAY_ACTION_CLEAN: u32 = 1;

impl Config {
    /// Clamp / validate every field to a value the rest of the app can rely on.
    ///
    /// The config file is user-editable (and older builds wrote values the
    /// current UI can no longer produce), so nothing downstream may assume the
    /// stored numbers are sane. Without this, for example, an
    /// `autoreduct_interval_value` of `0` makes the interval condition
    /// (`elapsed >= 0`) always true and the app cleans every 30 s forever.
    pub fn sanitize(&mut self) {
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        if !ACCENT_KEYS.contains(&self.accent_color.as_str()) {
            self.accent_color = "green".into();
        }
        if !LANGUAGES.contains(&self.language.as_str()) {
            self.language = "zh-CN".into();
        }

        // Only the 8 documented region bits are meaningful.
        self.reduct_mask &= crate::memory::mask::ALL;

        // A threshold of 0 would clean on every tick; an interval of 0 would
        // satisfy `elapsed >= 0`. Both are clamped to the UI's own ranges.
        self.autoreduct_value = self.autoreduct_value.clamp(1, 100);
        self.autoreduct_interval_value = self.autoreduct_interval_value.clamp(1, 1440);

        // Tray thresholds must stay ordered: the icon picks the danger colour
        // first, so `warning >= danger` would silently hide the warning state.
        self.tray_level_warning = self.tray_level_warning.min(99);
        self.tray_level_danger = self.tray_level_danger.clamp(1, 100);
        if self.tray_level_warning >= self.tray_level_danger {
            self.tray_level_warning = self.tray_level_danger - 1;
        }

        // Only the two documented click actions exist.
        if self.tray_action_dc > TRAY_ACTION_CLEAN {
            self.tray_action_dc = TRAY_ACTION_SHOW;
        }
        if self.tray_action_mc > TRAY_ACTION_CLEAN {
            self.tray_action_mc = TRAY_ACTION_SHOW;
        }

        // Colours are stored as 0x00RRGGBB.
        self.tray_color_text &= 0x00FF_FFFF;
        self.tray_color_bg &= 0x00FF_FFFF;
        self.tray_color_warning &= 0x00FF_FFFF;
        self.tray_color_danger &= 0x00FF_FFFF;

        // A hotkey with virtual key 0 can never be registered; fall back to the
        // documented default (Ctrl+F1).
        if self.hotkey_clean & 0xFFFF == 0 {
            self.hotkey_clean = DEFAULT_HOTKEY_CLEAN;
        }

        if self.statistic_last_reduct < 0 {
            self.statistic_last_reduct = 0;
        }
    }
}

/// Default global clean hotkey: Ctrl + F1, encoded as `(mods << 16) | vk`.
pub const DEFAULT_HOTKEY_CLEAN: u32 = (0x0002u32 << 16) | 0x71;

impl Default for Config {
    fn default() -> Self {
        Self {
            start_minimized: false,
            show_reduct_confirmation: true,
            theme: "system".into(),
            accent_color: "green".into(),
            use_dark_theme: false,
            language: "zh-CN".into(),

            autoreduct_enable: false,
            autoreduct_value: 90,
            autoreduct_interval_enable: false,
            autoreduct_interval_value: 30,
            allow_standby_list_cleanup: false,
            reduct_mask: crate::memory::mask::DEFAULT,

            hotkey_clean_enable: false,
            hotkey_clean: DEFAULT_HOTKEY_CLEAN,

            tray_use_transparency: false,
            tray_show_border: false,
            tray_round_corners: false,
            tray_change_bg: true,
            tray_color_text: 0x00FFFFFF, // white
            tray_color_bg: 0x00008040,   // green
            tray_color_warning: 0x00FF8040,
            tray_color_danger: 0x00EC1C24,

            tray_action_dc: TRAY_ACTION_SHOW,
            tray_action_mc: TRAY_ACTION_CLEAN,
            tray_level_warning: 70,
            tray_level_danger: 90,
            tray_tip_shown: false,

            balloon_clean_results: true,

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

/// Directory where the config (and log) lives.
pub fn data_dir() -> PathBuf {
    match config_location() {
        ConfigLocation::Portable => executable_dir(),
        ConfigLocation::AppData => appdata_dir(),
    }
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

/// Load config from disk; returns defaults if missing or malformed.
///
/// A malformed file is moved aside to `config.json.bak` instead of being
/// silently overwritten by the next save, so the user's settings can still be
/// recovered by hand.
pub fn load() -> Config {
    let path = config_path();
    let Ok(raw) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match serde_json::from_str::<Config>(&raw) {
        Ok(mut config) => {
            // Never hand out unvalidated values (the file is user-editable and
            // older builds persisted ranges the current UI cannot produce).
            config.sanitize();
            config
        }
        Err(_) => {
            let _ = fs::rename(&path, path.with_extension("json.bak"));
            Config::default()
        }
    }
}

/// Persist config to disk (creating directories as needed).
///
/// The value is sanitised first, so a malformed caller can never write a
/// config the next `load()` would have to repair.
///
/// The file is written to a temporary sibling and then renamed over the real
/// one, so an interrupted write (crash, power loss) can never leave a truncated
/// `config.json` behind.
pub fn save(config: &Config) -> std::io::Result<()> {
    let mut config = config.clone();
    config.sanitize();

    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let raw = serde_json::to_string_pretty(&config)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw)?;
    match fs::rename(&tmp, &path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
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
    fn sanitize_clamps_autoclean_ranges() {
        // 0 would make the interval condition always true → clean every 30 s.
        let mut c = Config {
            autoreduct_value: 0,
            autoreduct_interval_value: 0,
            ..Config::default()
        };
        c.sanitize();
        assert_eq!(c.autoreduct_value, 1);
        assert_eq!(c.autoreduct_interval_value, 1);

        let mut c = Config {
            autoreduct_value: 500,
            autoreduct_interval_value: 99_999,
            ..Config::default()
        };
        c.sanitize();
        assert_eq!(c.autoreduct_value, 100);
        assert_eq!(c.autoreduct_interval_value, 1440);
    }

    #[test]
    fn sanitize_keeps_tray_thresholds_ordered() {
        // Crossed thresholds would hide the warning colour entirely.
        let mut c = Config {
            tray_level_warning: 95,
            tray_level_danger: 60,
            ..Config::default()
        };
        c.sanitize();
        assert!(c.tray_level_warning < c.tray_level_danger, "{c:?}");

        let mut c = Config {
            tray_level_warning: 100,
            tray_level_danger: 100,
            ..Config::default()
        };
        c.sanitize();
        assert_eq!(c.tray_level_danger, 100);
        assert_eq!(c.tray_level_warning, 99);
    }

    #[test]
    fn sanitize_rejects_unknown_enums_and_bits() {
        let mut c = Config {
            theme: "neon".into(),
            accent_color: "../etc/passwd".into(),
            language: "xx-YY".into(),
            reduct_mask: 0xFFFF_FFFF,
            tray_action_dc: 42,
            hotkey_clean: 0,
            ..Config::default()
        };
        c.sanitize();
        assert_eq!(c.theme, "system");
        assert_eq!(c.accent_color, "green");
        assert_eq!(c.language, "zh-CN");
        assert_eq!(c.reduct_mask, crate::memory::mask::ALL);
        assert_eq!(c.tray_action_dc, TRAY_ACTION_SHOW);
        assert_eq!(c.hotkey_clean, DEFAULT_HOTKEY_CLEAN);
    }
}
