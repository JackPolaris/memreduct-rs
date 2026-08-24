// Accent color presets for the app theme. Each entry defines the primary and
// secondary accent (used as --accent / --accent2 CSS variables).

export interface Accent {
  key: string;
  name: string;
  primary: string;
  secondary: string;
}

export const ACCENTS: Accent[] = [
  { key: "green", name: "绿", primary: "#0b9d5e", secondary: "#07914f" },
  { key: "purple", name: "紫", primary: "#676ebb", secondary: "#555a9e" },
  { key: "blue", name: "蓝", primary: "#2563eb", secondary: "#1d4ed8" },
  { key: "orange", name: "橙", primary: "#ea7500", secondary: "#d16400" },
  { key: "red", name: "红", primary: "#ef4444", secondary: "#dc2626" },
  { key: "cyan", name: "青", primary: "#0891b2", secondary: "#0e7490" },
  { key: "pink", name: "粉", primary: "#db2777", secondary: "#be185d" },
];

export function accentByKey(key: string): Accent {
  return ACCENTS.find((a) => a.key === key) ?? ACCENTS[0];
}
