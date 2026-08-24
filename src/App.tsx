import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import i18n from "./i18n";
import {
  cleanMemory,
  getConfig,
  getConfigLocation,
  getMemoryInfo,
  getOsInfo,
  saveConfig,
  type CleanResult,
  type Config,
  type MemoryInfo,
  type OsInfo,
} from "./api";
import { MASK_ALL, MASK_DEFAULT, REGIONS } from "./regions";
import { SUPPORTED_LANGUAGES } from "./i18n";

type Tab = "main" | "settings";

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function colorForPercent(p: number): string {
  if (p >= 90) return "#ec1c24";
  if (p >= 70) return "#ff8040";
  return "#008040";
}

export default function App() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("main");
  const [info, setInfo] = useState<MemoryInfo | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [osInfo, setOsInfo] = useState<OsInfo | null>(null);
  const [configLocation, setConfigLocation] = useState<string>("");
  const [selectedMask, setSelectedMask] = useState<number>(MASK_DEFAULT);
  const [cleaning, setCleaning] = useState(false);
  const [lastResult, setLastResult] = useState<CleanResult | null>(null);
  const [confirmMask, setConfirmMask] = useState<number | null>(null);

  useEffect(() => {
    getMemoryInfo().then(setInfo);
    getOsInfo().then(setOsInfo);
    getConfigLocation().then(setConfigLocation);
    getConfig().then((c) => {
      setConfig(c);
      setSelectedMask(c.reduct_mask);
      // Apply persisted language.
      if (c.language && c.language !== i18n.language) {
        i18n.changeLanguage(c.language);
      }
    });

    // Live updates pushed from the backend background loop.
    const unlistenMemory = listen<MemoryInfo>("memory-update", (e) => {
      setInfo(e.payload);
    });
    const unlistenAuto = listen("autoclean-done", () => {
      getMemoryInfo().then(setInfo).catch(() => {});
    });

    // Fallback polling in case events are unavailable.
    const t = setInterval(() => {
      getMemoryInfo().then(setInfo).catch(() => {});
    }, 1000);

    return () => {
      clearInterval(t);
      unlistenMemory.then((fn) => fn());
      unlistenAuto.then((fn) => fn());
    };
  }, []);

  const regionLabel = (key: string) => t(`regions.${key}`);

  const runClean = async (mask: number) => {
    if (cleaning) return;
    setCleaning(true);
    setLastResult(null);
    try {
      const res = await cleanMemory(mask, "manual");
      setLastResult(res);
    } catch (e) {
      console.error("clean failed", e);
    } finally {
      setCleaning(false);
    }
  };

  const handleClean = () => {
    if (cleaning) return;
    if (config?.show_reduct_confirmation) {
      setConfirmMask(selectedMask);
    } else {
      runClean(selectedMask);
    }
  };

  const saveConfigAndReload = async (next: Config) => {
    setConfig(next);
    setSelectedMask(next.reduct_mask);
    await saveConfig(next);
  };

  const toggleRegion = (bit: number) => {
    setSelectedMask((m) => (m & bit ? m & ~bit : m | bit));
  };

  const phys = info?.physical_memory;
  const physPct = phys?.percent ?? 0;

  return (
    <div className={`app ${config?.use_dark_theme ? "dark" : ""}`}>
      <header className="topbar">
        <div className="brand">
          <span className="brand-icon">◆</span>
          <span className="brand-name">{t("app.name")}</span>
        </div>
        <nav className="tabs">
          <button
            className={tab === "main" ? "active" : ""}
            onClick={() => setTab("main")}
          >
            {t("app.main")}
          </button>
          <button
            className={tab === "settings" ? "active" : ""}
            onClick={() => setTab("settings")}
          >
            {t("app.settings")}
          </button>
        </nav>
      </header>

      <main className="content">
        {tab === "main" ? (
          <>
            <section className="hero">
              <div
                className="ring"
                style={{
                  background: `conic-gradient(${colorForPercent(physPct)} ${physPct}%, var(--muted) 0)`,
                }}
              >
                <div className="ring-inner">
                  <div className="ring-value">{physPct}%</div>
                  <div className="ring-label">{t("main.memoryUsed")}</div>
                </div>
              </div>
              <div className="hero-cards">
                <MetricCard title={t("main.physical")} obj={info?.physical_memory} t={t} />
                <MetricCard title={t("main.pageFile")} obj={info?.page_file} t={t} />
                <MetricCard title={t("main.systemCache")} obj={info?.system_cache} t={t} />
              </div>
            </section>

            <section className="panel">
              <div className="panel-title">{t("main.cleanRegions")}</div>
              <div className="region-grid">
                {REGIONS.map((r) => (
                  <label key={r.key} className="region">
                    <input
                      type="checkbox"
                      checked={(selectedMask & r.bit) !== 0}
                      onChange={() => toggleRegion(r.bit)}
                    />
                    <span className="region-body">
                      <span>{regionLabel(r.key)}</span>
                      {r.noteKey && <span className="region-note">{t(r.noteKey)}</span>}
                    </span>
                  </label>
                ))}
              </div>
              <div className="region-actions">
                <button className="ghost" onClick={() => setSelectedMask(MASK_ALL)}>
                  {t("main.all")}
                </button>
                <button className="ghost" onClick={() => setSelectedMask(MASK_DEFAULT)}>
                  {t("main.default")}
                </button>
              </div>
            </section>

            <button className="clean-btn" onClick={handleClean} disabled={cleaning}>
              {cleaning ? t("main.cleaning") : t("main.cleanMemory")}
            </button>

            {lastResult && (
              <div className="result">
                {t("main.released")}{" "}
                <strong>{formatBytes(lastResult.freed_bytes)}</strong> —{" "}
                {lastResult.regions.length
                  ? lastResult.regions.map((k) => t(`regions.${k}`)).join(", ")
                  : t("main.nothing")}
              </div>
            )}

            <footer className="footnote">
              {t("main.configLocation")}:{" "}
              {configLocation === "portable" ? t("main.portable") : t("main.appdata")}
              {osInfo && ` · Win ${osInfo.major}.${osInfo.minor}`} · v3.5.3
            </footer>
          </>
        ) : config ? (
          <SettingsPanel config={config} t={t} onSave={saveConfigAndReload} />
        ) : null}
      </main>

      {confirmMask !== null && (
        <div className="modal-overlay" onClick={() => setConfirmMask(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-title">{t("confirm.title")}</div>
            <ul className="modal-list">
              {REGIONS.filter((r) => confirmMask & r.bit).map((r) => (
                <li key={r.key}>{t(`regions.${r.key}`)}</li>
              ))}
            </ul>
            <div className="modal-actions">
              <button className="primary" onClick={() => { setConfirmMask(null); runClean(confirmMask); }}>
                {t("confirm.clean")}
              </button>
              <button className="ghost" onClick={() => setConfirmMask(null)}>
                {t("confirm.cancel")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function MetricCard({
  title,
  obj,
  t,
}: {
  title: string;
  obj?: { total_bytes: number; free_bytes: number; used_bytes: number; percent: number };
  t: (k: string) => string;
}) {
  if (!obj) return <div className="metric">—</div>;
  const barClass =
    obj.percent >= 90 ? "bar-fill danger" : obj.percent >= 70 ? "bar-fill warn" : "bar-fill";
  return (
    <div className="metric">
      <div className="metric-title">{title}</div>
      <div className="metric-value">{formatBytes(obj.used_bytes)}</div>
      <div className="metric-sub">
        {t("main.of")} {formatBytes(obj.total_bytes)} ({obj.percent}%)
      </div>
      <div className="bar">
        <div className={barClass} style={{ width: `${obj.percent}%` }} />
      </div>
    </div>
  );
}

function SettingsPanel({
  config,
  t,
  onSave,
}: {
  config: Config;
  t: (k: string) => string;
  onSave: (c: Config) => void;
}) {
  const [draft, setDraft] = useState<Config>(config);
  const [section, setSection] = useState<
    "general" | "memory" | "appearance" | "tray" | "advanced"
  >("general");

  useEffect(() => {
    setDraft(config);
  }, [config]);

  const set = <K extends keyof Config>(k: K, v: Config[K]) => {
    setDraft((d) => ({ ...d, [k]: v }));
  };

  const sections = [
    "general",
    "memory",
    "appearance",
    "tray",
    "advanced",
  ] as const;

  return (
    <div className="settings">
      <div className="settings-nav">
        {sections.map((s) => (
          <button
            key={s}
            className={section === s ? "active" : ""}
            onClick={() => setSection(s)}
          >
            {t(`settings.${s}`)}
          </button>
        ))}
      </div>

      <div className="settings-body">
        {section === "general" && (
          <>
            <Toggle label={t("settings.alwaysOnTop")} checked={draft.always_on_top} onChange={(v) => set("always_on_top", v)} />
            <Toggle label={t("settings.startMinimized")} checked={draft.start_minimized} onChange={(v) => set("start_minimized", v)} />
            <Toggle label={t("settings.showCleanConfirmation")} checked={draft.show_reduct_confirmation} onChange={(v) => set("show_reduct_confirmation", v)} />
            <Toggle label={t("settings.checkUpdates")} checked={draft.check_updates} onChange={(v) => set("check_updates", v)} />
            <Toggle label={t("settings.darkTheme")} checked={draft.use_dark_theme} onChange={(v) => set("use_dark_theme", v)} />
            <div className="row">
              <span>{t("app.language")}</span>
              <select
                value={draft.language}
                onChange={(e) => {
                  set("language", e.target.value);
                  i18n.changeLanguage(e.target.value);
                }}
              >
                {SUPPORTED_LANGUAGES.map((l) => (
                  <option key={l.code} value={l.code}>
                    {l.label}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}

        {section === "memory" && (
          <>
            <Toggle label={t("settings.autoReduct")} checked={draft.autoreduct_enable} onChange={(v) => set("autoreduct_enable", v)} />
            <Slider label={t("settings.autoReductThreshold")} value={draft.autoreduct_value} min={0} max={100} onChange={(v) => set("autoreduct_value", v)} />
            <Toggle label={t("settings.autoReductInterval")} checked={draft.autoreduct_interval_enable} onChange={(v) => set("autoreduct_interval_enable", v)} />
            <Slider label={t("settings.interval")} value={draft.autoreduct_interval_value} min={1} max={1440} onChange={(v) => set("autoreduct_interval_value", v)} />
            <Toggle label={t("settings.allowStandbyCleanup")} checked={draft.allow_standby_list_cleanup} onChange={(v) => set("allow_standby_list_cleanup", v)} />
            <div className="hint">{t("settings.standbyHint")}</div>
          </>
        )}

        {section === "appearance" && (
          <>
            <ColorRow label={t("settings.textColor")} value={configHex(draft.tray_color_text)} onChange={(v) => set("tray_color_text", parseHex(v))} />
            <ColorRow label={t("settings.backgroundColor")} value={configHex(draft.tray_color_bg)} onChange={(v) => set("tray_color_bg", parseHex(v))} />
            <ColorRow label={t("settings.warningColor")} value={configHex(draft.tray_color_warning)} onChange={(v) => set("tray_color_warning", parseHex(v))} />
            <ColorRow label={t("settings.dangerColor")} value={configHex(draft.tray_color_danger)} onChange={(v) => set("tray_color_danger", parseHex(v))} />
            <Toggle label={t("settings.trayBorder")} checked={draft.tray_show_border} onChange={(v) => set("tray_show_border", v)} />
            <Toggle label={t("settings.trayRoundCorners")} checked={draft.tray_round_corners} onChange={(v) => set("tray_round_corners", v)} />
            <Toggle label={t("settings.trayTransparency")} checked={draft.tray_use_transparency} onChange={(v) => set("tray_use_transparency", v)} />
            <Toggle label={t("settings.trayChangeBg")} checked={draft.tray_change_bg} onChange={(v) => set("tray_change_bg", v)} />
          </>
        )}

        {section === "tray" && (
          <>
            <Select label={t("settings.doubleClickAction")} value={draft.tray_action_dc} onChange={(v) => set("tray_action_dc", v)} options={[[0, t("tray.show")], [1, t("tray.clean")]]} />
            <Select label={t("settings.middleClickAction")} value={draft.tray_action_mc} onChange={(v) => set("tray_action_mc", v)} options={[[0, t("tray.show")], [1, t("tray.clean")]]} />
            <Slider label={t("settings.warningLevel")} value={draft.tray_level_warning} min={0} max={100} onChange={(v) => set("tray_level_warning", v)} />
            <Slider label={t("settings.dangerLevel")} value={draft.tray_level_danger} min={0} max={100} onChange={(v) => set("tray_level_danger", v)} />
          </>
        )}

        {section === "advanced" && (
          <>
            <Toggle label={t("settings.notificationSound")} checked={draft.notifications_sound} onChange={(v) => set("notifications_sound", v)} />
            <Toggle label={t("settings.showCleanResult")} checked={draft.balloon_clean_results} onChange={(v) => set("balloon_clean_results", v)} />
            <Toggle label={t("settings.logCleanResults")} checked={draft.log_clean_results} onChange={(v) => set("log_clean_results", v)} />
            <Toggle label={t("settings.hotkeyClean")} checked={draft.hotkey_clean_enable} onChange={(v) => set("hotkey_clean_enable", v)} />
            <div className="hint">{t("settings.hotkeyHint")}</div>
          </>
        )}
      </div>

      <div className="settings-footer">
        <button className="primary" onClick={() => onSave(draft)}>
          {t("settings.save")}
        </button>
        <button className="ghost" onClick={() => setDraft(config)}>
          {t("settings.reset")}
        </button>
      </div>
    </div>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="row toggle">
      <span>{label}</span>
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
    </label>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  onChange: (v: number) => void;
}) {
  return (
    <div className="row slider">
      <span>
        {label}: <strong>{value}</strong>
      </span>
      <input
        type="range"
        min={min}
        max={max}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
    </div>
  );
}

function Select({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  options: [number, string][];
}) {
  return (
    <div className="row">
      <span>{label}</span>
      <select value={value} onChange={(e) => onChange(Number(e.target.value))}>
        {options.map(([v, l]) => (
          <option key={v} value={v}>
            {l}
          </option>
        ))}
      </select>
    </div>
  );
}

function ColorRow({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="row">
      <span>{label}</span>
      <input type="color" value={value} onChange={(e) => onChange(e.target.value)} />
    </div>
  );
}

function configHex(rgb: number): string {
  const r = (rgb >> 16) & 0xff;
  const g = (rgb >> 8) & 0xff;
  const b = rgb & 0xff;
  return `#${[r, g, b].map((x) => x.toString(16).padStart(2, "0")).join("")}`;
}

function parseHex(hex: string): number {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return (r << 16) | (g << 8) | b;
}
