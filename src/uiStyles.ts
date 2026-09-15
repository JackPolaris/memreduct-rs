// UI skins ("styles") for the app shell.
//
// A skin is a whole visual language — corner radius, border weight, elevation
// model, typography, density and the amount of decoration — while the existing
// `theme` config (`light` / `dark` / `system`) only selects which palette of
// that language to paint with. The two are deliberately orthogonal: every skin
// that can be shown in both modes has a light and a dark palette in
// `styles.css`, and the ones that are dark-native declare it here so the app
// can pin the mode instead of rendering a broken light-on-neon screen.
//
// Keys MUST stay in sync with `UI_STYLES` in `src-tauri/src/config.rs` (which
// whitelists them) and with the `[data-ui="..."]` blocks in `styles.css`.

/** Which colour modes a skin can actually be rendered in. */
export type SkinMode =
  /** Ships a light and a dark palette; follows the user's `theme` setting. */
  | "both"
  /** Has only a dark palette — `theme` is ignored while it is selected. */
  | "dark";

export interface UiStyle {
  /** Config key stored in `config.ui_style`. */
  key: string;
  /** i18n key under `style.*` for the human-readable name. */
  nameKey: string;
  /** i18n key under `style.*` for the one-line description. */
  descKey: string;
  /** Colour modes this skin supports. */
  mode: SkinMode;
}

export const UI_STYLES: UiStyle[] = [
  {
    key: "glass",
    nameKey: "style.glass",
    descKey: "style.glassDesc",
    mode: "both",
  },
  {
    key: "industrial",
    nameKey: "style.industrial",
    descKey: "style.industrialDesc",
    mode: "both",
  },
  {
    key: "neon",
    nameKey: "style.neon",
    descKey: "style.neonDesc",
    mode: "dark",
  },
  {
    key: "terminal",
    nameKey: "style.terminal",
    descKey: "style.terminalDesc",
    mode: "dark",
  },
  {
    key: "minimal",
    nameKey: "style.minimal",
    descKey: "style.minimalDesc",
    mode: "both",
  },
];

/** Fallback used whenever the stored key is missing or unrecognised. */
export const DEFAULT_UI_STYLE = UI_STYLES[0];

export function uiStyleByKey(key: string | undefined): UiStyle {
  return UI_STYLES.find((s) => s.key === key) ?? DEFAULT_UI_STYLE;
}
