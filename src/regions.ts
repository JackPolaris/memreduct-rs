// Memory region definitions (mirrors the original WTS mask and labels).

export const REGIONS = [
  { bit: 0x01, key: "workingset", label: "Working Set", note: "" },
  { bit: 0x02, key: "systemfilecache", label: "System File Cache", note: "" },
  { bit: 0x04, key: "standbypriority0", label: "Standby Priority-0 List", note: "" },
  { bit: 0x08, key: "standbylist", label: "Standby List", note: "may freeze" },
  { bit: 0x10, key: "modifiedlist", label: "Modified Page List", note: "may freeze" },
  { bit: 0x20, key: "combinememorylists", label: "Combine Memory Lists", note: "win10+" },
  { bit: 0x40, key: "registrycache", label: "Registry Cache", note: "win8.1+" },
  { bit: 0x80, key: "modifiedfilecache", label: "Modified File Cache", note: "" },
] as const;

export const MASK_DEFAULT =
  0x01 | 0x02 | 0x04 | 0x40 | 0x20 | 0x80;
export const MASK_ALL = 0xff;
export const MASK_FREEZES = 0x08 | 0x10;

export function maskNames(mask: number): string[] {
  return REGIONS.filter((r) => (mask & r.bit) !== 0).map((r) => r.label);
}
