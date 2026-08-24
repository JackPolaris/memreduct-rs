// Memory region definitions (masks mirror the original WTS mask).
// Region display names live in the i18n dictionaries; here we keep the
// mask, a translation key, and an optional platform note key.

export const REGIONS = [
  { bit: 0x01, key: "workingSet", noteKey: "" },
  { bit: 0x02, key: "systemFileCache", noteKey: "" },
  { bit: 0x04, key: "standbyPriority0", noteKey: "" },
  { bit: 0x08, key: "standbyList", noteKey: "note.freeze" },
  { bit: 0x10, key: "modifiedList", noteKey: "note.freeze" },
  { bit: 0x20, key: "combineMemoryLists", noteKey: "note.win10" },
  { bit: 0x40, key: "registryCache", noteKey: "note.win81" },
  { bit: 0x80, key: "modifiedFileCache", noteKey: "" },
] as const;

export const MASK_DEFAULT = 0x01 | 0x02 | 0x04 | 0x40 | 0x20 | 0x80;
export const MASK_ALL = 0xff;
export const MASK_FREEZES = 0x08 | 0x10;

export function maskNames(mask: number): string[] {
  return REGIONS.filter((r) => (mask & r.bit) !== 0).map((r) => r.key);
}
