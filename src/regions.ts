// Memory region definitions (masks mirror the original WTS mask and the
// backend's `memory::mask`).
//
// Region display names live in the i18n dictionaries; here we keep the mask, a
// translation key, an optional platform note key, and the minimum Windows
// version the region actually works on (so the UI can disable — not silently
// ignore — regions the running system cannot clean).
//
// The order MUST match `mask::names()` in `src-tauri/src/memory.rs`.

/** Minimum Windows version a region requires. */
export type MinWindows = "any" | "win81" | "win10";

export interface Region {
  bit: number;
  key: string;
  noteKey: string;
  min: MinWindows;
}

export const REGIONS: Region[] = [
  { bit: 0x01, key: "workingSet", noteKey: "", min: "any" },
  { bit: 0x02, key: "systemFileCache", noteKey: "", min: "any" },
  { bit: 0x04, key: "standbyPriority0", noteKey: "", min: "any" },
  { bit: 0x08, key: "standbyList", noteKey: "note.freeze", min: "any" },
  { bit: 0x10, key: "modifiedList", noteKey: "note.freeze", min: "any" },
  { bit: 0x20, key: "combineMemoryLists", noteKey: "note.win10", min: "win10" },
  { bit: 0x40, key: "registryCache", noteKey: "note.win81", min: "win81" },
  { bit: 0x80, key: "modifiedFileCache", noteKey: "", min: "any" },
];

export const MASK_DEFAULT = 0x01 | 0x02 | 0x04 | 0x40 | 0x20 | 0x80;
export const MASK_ALL = 0xff;

/** OS capabilities as reported by the `get_os_info` command. */
export interface OsCapabilities {
  is_win8_1: boolean;
  is_win10: boolean;
}

/**
 * Whether the running system supports a region.
 *
 * `os === null` means the capability query has not answered yet; the region is
 * treated as supported so the UI never flashes as disabled on startup.
 */
export function isRegionSupported(region: Region, os: OsCapabilities | null): boolean {
  if (!os) return true;
  if (region.min === "win10") return os.is_win10;
  if (region.min === "win81") return os.is_win8_1;
  return true;
}

/** Mask with every region this system cannot clean removed. */
export function supportedMask(mask: number, os: OsCapabilities | null): number {
  return REGIONS.filter((region) => isRegionSupported(region, os)).reduce(
    (acc, region) => acc | (mask & region.bit),
    0
  );
}
