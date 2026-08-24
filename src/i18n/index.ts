// i18next setup: Simplified Chinese is the primary language, with support for
// multiple locales. The app language is configurable in Settings; default
// fallbacks to the system locale, then to zh-CN.
import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import zhCN from "./zh-CN.json";
import zhTW from "./zh-TW.json";
import enUS from "./en-US.json";
import jaJP from "./ja-JP.json";

export const SUPPORTED_LANGUAGES = [
  { code: "zh-CN", label: "简体中文" },
  { code: "zh-TW", label: "繁體中文" },
  { code: "en-US", label: "English" },
  { code: "ja-JP", label: "日本語" },
  { code: "ko-KR", label: "한국어" },
  { code: "de-DE", label: "Deutsch" },
  { code: "fr-FR", label: "Français" },
] as const;

export const DEFAULT_LANGUAGE = "zh-CN";

// Load a language bundle synchronously (small JSON). For locales without a
// dedicated bundle we fall back to zh-CN (primary) so the UI never shows raw
// keys.
function loadResource(lang: string): Record<string, unknown> {
  switch (lang) {
    case "en-US":
      return enUS;
    case "zh-TW":
      return zhTW;
    case "ja-JP":
      return jaJP;
    default:
      return zhCN;
  }
}

function detectLanguage(): string {
  // Prefer the persisted config language (set on startup), else system locale.
  try {
    const nav = navigator.language;
    if (nav && nav.toLowerCase().startsWith("en")) return "en-US";
    if (nav && nav.toLowerCase().startsWith("zh")) return "zh-CN";
  } catch {
    // ignore
  }
  return DEFAULT_LANGUAGE;
}

const initialLang = detectLanguage();
export const CURRENT_LANGS = SUPPORTED_LANGUAGES.map((l) => l.code);

i18n.use(initReactI18next).init({
  lng: initialLang,
  fallbackLng: "zh-CN",
  resources: {
    "zh-CN": { translation: loadResource("zh-CN") },
    "zh-TW": { translation: loadResource("zh-TW") },
    "en-US": { translation: loadResource("en-US") },
    "ja-JP": { translation: loadResource("ja-JP") },
    "ko-KR": { translation: loadResource("ko-KR") },
    "de-DE": { translation: loadResource("de-DE") },
    "fr-FR": { translation: loadResource("fr-FR") },
  },
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
