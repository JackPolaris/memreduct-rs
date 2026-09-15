// Accent color presets for the app theme.
//
// Only the base accent colour is pushed to CSS (`--accent-base`); every derived
// tone (hover/gradient/soft tint/glow) is computed from it in styles.css, so a
// preset switch really re-tints the whole UI — including dark mode.

export interface Accent {
  /** Config key stored in `config.accent_color`. */
  key: string;
  /** i18n key under `accent.*` for the human-readable name. */
  nameKey: string;
  /** Base accent colour (light theme). */
  primary: string;
}

export const ACCENTS: Accent[] = [
  { key: "green", nameKey: "accent.green", primary: "#0b9d5e" },
  { key: "purple", nameKey: "accent.purple", primary: "#676ebb" },
  { key: "blue", nameKey: "accent.blue", primary: "#2563eb" },
  { key: "orange", nameKey: "accent.orange", primary: "#ea7500" },
  { key: "red", nameKey: "accent.red", primary: "#ef4444" },
  { key: "cyan", nameKey: "accent.cyan", primary: "#0891b2" },
  { key: "pink", nameKey: "accent.pink", primary: "#db2777" },
];

export function accentByKey(key: string): Accent {
  return ACCENTS.find((a) => a.key === key) ?? ACCENTS[0];
}
