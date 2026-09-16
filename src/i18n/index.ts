// i18next setup: Simplified Chinese is the primary language.
//
// Only the locales that actually ship a translation bundle are listed as
// selectable languages — every entry below maps to a JSON file in this folder
// (the picker previously offered Korean/German/French, which silently fell back
// to Chinese, so the UI claimed a language it could not display).
//
// Adding a language means three things, and they must stay in lockstep:
//   1. a new `<code>.json` bundle with exactly the same key set as `en-US.json`
//      (121 leaf keys — `RESOURCES` below is typed against `LanguageCode`, so a
//      missing bundle is a compile error, but a missing *key* is not);
//   2. an entry in `SUPPORTED_LANGUAGES` here;
//   3. the same code in `LANGUAGES` (src-tauri/src/config.rs), otherwise the
//      backend silently rewrites the saved language back to zh-CN on save.
import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import zhCN from "./zh-CN.json";
import zhTW from "./zh-TW.json";
import enUS from "./en-US.json";
import jaJP from "./ja-JP.json";
import koKR from "./ko-KR.json";
import deDE from "./de-DE.json";
import frFR from "./fr-FR.json";
import esES from "./es-ES.json";
import ptBR from "./pt-BR.json";
import itIT from "./it-IT.json";
import ruRU from "./ru-RU.json";
import plPL from "./pl-PL.json";
import trTR from "./tr-TR.json";
import viVN from "./vi-VN.json";
import thTH from "./th-TH.json";
import idID from "./id-ID.json";

// Order is user-visible: it drives the language <select>. Chinese first (the
// primary audience), then the rest roughly by number of speakers.
export const SUPPORTED_LANGUAGES = [
  { code: "zh-CN", label: "简体中文" },
  { code: "zh-TW", label: "繁體中文" },
  { code: "en-US", label: "English" },
  { code: "ja-JP", label: "日本語" },
  { code: "ko-KR", label: "한국어" },
  { code: "de-DE", label: "Deutsch" },
  { code: "fr-FR", label: "Français" },
  { code: "es-ES", label: "Español" },
  { code: "pt-BR", label: "Português (Brasil)" },
  { code: "it-IT", label: "Italiano" },
  { code: "ru-RU", label: "Русский" },
  { code: "pl-PL", label: "Polski" },
  { code: "tr-TR", label: "Türkçe" },
  { code: "vi-VN", label: "Tiếng Việt" },
  { code: "th-TH", label: "ไทย" },
  { code: "id-ID", label: "Bahasa Indonesia" },
] as const;

export type LanguageCode = (typeof SUPPORTED_LANGUAGES)[number]["code"];

export const DEFAULT_LANGUAGE: LanguageCode = "zh-CN";

/** Translation bundles, keyed by the exact language code used everywhere else. */
const RESOURCES: Record<LanguageCode, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  "en-US": enUS,
  "ja-JP": jaJP,
  "ko-KR": koKR,
  "de-DE": deDE,
  "fr-FR": frFR,
  "es-ES": esES,
  "pt-BR": ptBR,
  "it-IT": itIT,
  "ru-RU": ruRU,
  "pl-PL": plPL,
  "tr-TR": trTR,
  "vi-VN": viVN,
  "th-TH": thTH,
  "id-ID": idID,
};

/**
 * Locale prefixes we can map onto a shipped bundle, **longest first**.
 *
 * Order matters: the first match wins, so the Chinese script/region variants
 * must precede the generic `zh`. We ship fewer bundles than the world has
 * locales, so several locales intentionally collapse onto one bundle
 * (`pt-PT` → pt-BR, `en-GB` → en-US, `zh-HK` → zh-TW).
 */
const LANGUAGE_PREFIXES: ReadonlyArray<readonly [string, LanguageCode]> = [
  ["zh-hant", "zh-TW"],
  ["zh-hans", "zh-CN"],
  ["zh-tw", "zh-TW"],
  ["zh-hk", "zh-TW"],
  ["zh-mo", "zh-TW"],
  ["zh", "zh-CN"],
  ["en", "en-US"],
  ["ja", "ja-JP"],
  ["ko", "ko-KR"],
  ["de", "de-DE"],
  ["fr", "fr-FR"],
  ["es", "es-ES"],
  ["pt", "pt-BR"],
  ["it", "it-IT"],
  ["ru", "ru-RU"],
  ["pl", "pl-PL"],
  ["tr", "tr-TR"],
  ["vi", "vi-VN"],
  ["th", "th-TH"],
  // `in` is the legacy ISO 639 code for Indonesian; some browsers still emit it.
  ["in", "id-ID"],
  ["id", "id-ID"],
];

/** Normalise any locale string ("zh-Hans-CN", "en", "pt_BR", …) to a supported code. */
export function normalizeLanguage(lang: string | undefined): LanguageCode | undefined {
  if (!lang) return undefined;
  // BCP-47 uses "-", but POSIX-style tags ("zh_CN") reach us from some environments.
  const lower = lang.toLowerCase().replace(/_/g, "-");
  const exact = SUPPORTED_LANGUAGES.find((l) => l.code.toLowerCase() === lower);
  if (exact) return exact.code;
  for (const [prefix, code] of LANGUAGE_PREFIXES) {
    // Match the bare tag ("en") or a region/script suffix ("en-GB"), never a
    // longer unrelated tag ("enm" is Middle English, not English).
    if (lower === prefix || lower.startsWith(prefix + "-")) return code;
  }
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
