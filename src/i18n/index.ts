// i18next setup: Simplified Chinese is the primary language.
//
// Only the locales that actually ship a translation bundle are listed as
// selectable languages — every entry below maps to a JSON file in this folder
// (the picker previously offered Korean/German/French, which silently fell back
// to Chinese, so the UI claimed a language it could not display).
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
] as const;

export type LanguageCode = (typeof SUPPORTED_LANGUAGES)[number]["code"];

export const DEFAULT_LANGUAGE: LanguageCode = "zh-CN";

/** Translation bundles, keyed by the exact language code used everywhere else. */
const RESOURCES: Record<LanguageCode, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  "en-US": enUS,
  "ja-JP": jaJP,
};

/** Normalise any locale string ("zh-Hans-CN", "en", "zh-HK", …) to a supported code. */
export function normalizeLanguage(lang: string | undefined): LanguageCode | undefined {
  if (!lang) return undefined;
  const lower = lang.toLowerCase();
  const exact = SUPPORTED_LANGUAGES.find((l) => l.code.toLowerCase() === lower);
  if (exact) return exact.code;
  if (lower.startsWith("zh")) {
    return /hant|tw|hk|mo/.test(lower) ? "zh-TW" : "zh-CN";
  }
  if (lower.startsWith("en")) return "en-US";
  if (lower.startsWith("ja")) return "ja-JP";
  return undefined;
}

/** Detect the initial language from the system locale, falling back to zh-CN. */
function detectLanguage(): LanguageCode {
  try {
    return normalizeLanguage(navigator.language) ?? DEFAULT_LANGUAGE;
  } catch {
    return DEFAULT_LANGUAGE;
  }
}

i18n.use(initReactI18next).init({
  lng: detectLanguage(),
  fallbackLng: DEFAULT_LANGUAGE,
  supportedLngs: SUPPORTED_LANGUAGES.map((l) => l.code),
  resources: Object.fromEntries(
    Object.entries(RESOURCES).map(([code, bundle]) => [code, { translation: bundle }])
  ),
  interpolation: { escapeValue: false },
  returnNull: false,
});

export default i18n;
