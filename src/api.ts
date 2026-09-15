// Type definitions shared with the Tauri backend, and a thin typed wrapper
// around the `invoke` bridge.

import { invoke } from "@tauri-apps/api/core";

export interface MemoryObject {
  total_bytes: number;
  free_bytes: number;
  used_bytes: number;
  percent: number;
  percent_f: number;
}

export interface MemoryInfo {
  physical_memory: MemoryObject;
  page_file: MemoryObject;
  system_cache: MemoryObject;
}

export interface CleanResult {
  freed_bytes: number;
  applied_mask: number;
  regions: string[];
  /**
   * Region keys whose underlying NT call failed. Empty on a fully successful
   * cleanup; non-empty usually means the app is not running elevated.
   */
  failed: string[];
}

/** Payload of the `clean-done` event emitted for non-UI cleanup sources. */
export interface CleanDonePayload {
  source: "manual" | "tray" | "hotkey" | "auto" | "cmdline";
  result: CleanResult;
}

export interface Config {
  start_minimized: boolean;
  show_reduct_confirmation: boolean;
  theme: string;
  /**
   * UI skin: `glass` | `industrial` | `neon` | `terminal` | `minimal`.
   *
   * Independent of `theme`: the skin decides the visual language, `theme` only
   * picks the light or dark palette within it. Keys must match
   * `UI_STYLES` in `src-tauri/src/config.rs` and `UI_STYLES` in `src/uiStyles.ts`.
   */
  ui_style: string;
  accent_color: string;
  use_dark_theme: boolean;
  language: string;

  autoreduct_enable: boolean;
  autoreduct_value: number;
  autoreduct_interval_enable: boolean;
  autoreduct_interval_value: number;
  allow_standby_list_cleanup: boolean;
  reduct_mask: number;

  hotkey_clean_enable: boolean;
  hotkey_clean: number;

  tray_use_transparency: boolean;
  tray_show_border: boolean;
  tray_round_corners: boolean;
  tray_change_bg: boolean;
  tray_color_text: number;
  tray_color_bg: number;
  tray_color_warning: number;
  tray_color_danger: number;

  tray_action_dc: number;
  tray_action_mc: number;
  tray_level_warning: number;
  tray_level_danger: number;
  /** Whether the one-off "still running in the tray" hint has been shown. */
  tray_tip_shown: boolean;

  balloon_clean_results: boolean;

  statistic_last_reduct: number;
}

export interface OsInfo {
  major: number;
  minor: number;
  is_win8_1: boolean;
  is_win10: boolean;
}

export const getMemoryInfo = () => invoke<MemoryInfo>("get_memory_info");
export const isElevated = () => invoke<boolean>("is_elevated");
export const cleanMemory = (mask: number, source: string) =>
  invoke<CleanResult>("clean_memory", { mask, source });
export const notify = (title: string, body: string, system = true) =>
  invoke<void>("notify", { title, body, system });

/** Tray menu labels, supplied by the frontend so they follow the app language. */
export interface TrayLabels {
  show: string;
  clean: string;
  settings: string;
  website: string;
  about: string;
  exit: string;
}
export const applyTrayLabels = (labels: TrayLabels) =>
  invoke<void>("apply_tray_labels", { labels });

// Automatic update via tauri-plugin-updater (source fixed to the official repo).
export interface UpdateInfo {
  available: boolean;
  version: string;
  date: string;
  body: string;
  current_version: string;
}

/**
 * Static updater facts (no network I/O).
 *
 * Only the release page is exposed: the manifest URL is machine JSON that no
 * user can act on, and a failed check already names the endpoint it tried.
 */
export interface UpdaterInfo {
  current_version: string;
  release_page: string;
}

export const checkForUpdate = () => invoke<UpdateInfo>("check_for_update");
export const downloadAndInstall = () => invoke<void>("download_and_install");
export const getUpdaterInfo = () => invoke<UpdaterInfo>("get_updater_info");
export const getConfig = () => invoke<Config>("get_config");
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });
export const getOsInfo = () => invoke<OsInfo>("get_os_info");
export const getVersion = () => invoke<string>("get_version");
export const openExternal = (url: string) => invoke<void>("open_external", { url });
export const getAutostart = () => invoke<boolean>("get_autostart");
export const setAutostart = (enabled: boolean) =>
  invoke<string>("set_autostart", { enabled });
