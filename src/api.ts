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
}

export interface Config {
  always_on_top: boolean;
  start_minimized: boolean;
  show_reduct_confirmation: boolean;
  check_updates: boolean;
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
  tray_use_antialiasing: boolean;
  tray_color_text: number;
  tray_color_bg: number;
  tray_color_warning: number;
  tray_color_danger: number;
  tray_font: string;

  tray_action_dc: number;
  tray_action_mc: number;
  tray_level_warning: number;
  tray_level_danger: number;

  notifications_sound: boolean;
  balloon_clean_results: boolean;
  log_clean_results: boolean;

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
export const getConfig = () => invoke<Config>("get_config");
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });
export const getConfigLocation = () => invoke<string>("get_config_location");
export const getOsInfo = () => invoke<OsInfo>("get_os_info");
