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
  isElevated,
  saveConfig,
  type CleanResult,
  type Config,
  type MemoryInfo,
  type OsInfo,
} from "./api";
import { MASK_ALL, MASK_DEFAULT, REGIONS } from "./regions";
import { SUPPORTED_LANGUAGES } from "./i18n";
import {
  IconBell,
  IconBolt,
  IconCache,
  IconChip,
  IconDrive,
  IconGauge,
  IconKeyboard,
  IconPalette,
  IconSettings,
  IconShield,
  IconSparkles,
  IconTray,
} from "./icons";

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
  if (p >= 90) return "#ef4444";
  if (p >= 70) return "#f59e0b";
  return "#0b9d5e";
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
  const [elevated, setElevated] = useState<boolean>(true);

  useEffect(() => {
    getMemoryInfo().then(setInfo);
    getOsInfo().then(setOsInfo);
    getConfigLocation().then(setConfigLocation);
    isElevated().then(setElevated).catch(() => setElevated(false));
    getConfig().then((c) => {
      setConfig(c);
      setSelectedMask(c.reduct_mask);
      if (c.language && c.language !== i18n.language) {
        i18n.changeLanguage(c.language);
      }
    });

    const unlistenMemory = listen<MemoryInfo>("memory-update", (e) => {
      setInfo(e.payload);
    });
    const unlistenAuto = listen("autoclean-done", () => {
      getMemoryInfo().then(setInfo).catch(() => {});
    });
    const unlistenSettings = listen("open-settings", () => {
      setTab("settings");
    });
    const unlistenAbout = listen("show-about", () => {
      setTab("main");
    });

    const poll = setInterval(() => {
      getMemoryInfo().then(setInfo).catch(() => {});
    }, 1000);

    return () => {
      clearInterval(poll);
      unlistenMemory.then((fn) => fn());
      unlistenAuto.then((fn) => fn());
      unlistenSettings.then((fn) => fn());
      unlistenAbout.then((fn) => fn());
    };
  }, []);

  const runClean = async (mask: number) => {
    if (cleaning) return;
    setCleaning(true);
    setLastResult(null);
    try {
      const res = await cleanMemory(mask, "manual");
      setLastResult(res);
      getMemoryInfo().then(setInfo).catch(() => {});
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
  const pressure = physPct >= 90 ? "crit" : physPct >= 70 ? "warn" : "ok";
  const selectedCount = REGIONS.filter((r) => selectedMask & r.bit).length;

  return (
    <div className={`app ${config?.use_dark_theme ? "dark" : ""}`}>
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark">
            <IconBolt size={18} />
          </div>
          <span className="brand-name">{t("app.name")}</span>
        </div>
        <nav className="tabs">
          <button
            className={tab === "main" ? "active" : ""}
            onClick={() => setTab("main")}
          >
            <IconGauge size={15} />
            {t("app.main")}
          </button>
          <button
            className={tab === "settings" ? "active" : ""}
            onClick={() => setTab("settings")}
          >
            <IconSettings size={15} />
            {t("app.settings")}
          </button>
        </nav>
      </header>

      <main className="content">
        {tab === "main" ? (
          <>
            <div className="statusbar">
              <span className={`statuschip ${pressure}`}>
                <span className="dot" />
                {t(`status.${pressure}`)}
              </span>
              {!elevated && (
                <span className="statuschip crit">
                  <IconShield size={13} />
                  {t("main.notElevated")}
                </span>
              )}
              <span className="statusbar-note">
                {configLocation === "portable" ? t("main.portable") : t("main.appdata")}
                {osInfo && ` · Win ${osInfo.major}.${osInfo.minor}`} · v3.5.3
              </span>
            </div>

            <section className="hero glass">
              <div className="gauge-wrap">
                <div
                  className="gauge"
                  style={{
                    background: `conic-gradient(${colorForPercent(physPct)} ${physPct}%, var(--ring-bg) ${physPct}% 100%)`,
                  }}
                >
                  <div className="gauge-inner">
                    <div className="gauge-value">{physPct}%</div>
                    <div className="gauge-label">{t("main.memoryUsed")}</div>
                    <div className="gauge-sub">
                      {info ? formatBytes(info.physical_memory.used_bytes) : "—"}
                    </div>
                  </div>
                </div>
              </div>

              <div className="metrics">
                <MetricCard
                  icon={<IconChip size={17} />}
                  title={t("main.physical")}
                  obj={info?.physical_memory}
                  t={t}
                />
                <MetricCard
                  icon={<IconDrive size={17} />}
                  title={t("main.pageFile")}
                  obj={info?.page_file}
                  t={t}
                />
                <MetricCard
                  icon={<IconCache size={17} />}
                  title={t("main.systemCache")}
                  obj={info?.system_cache}
                  t={t}
                />
              </div>
            </section>

            <section className="panel glass">
              <div className="panel-head">
                <div className="panel-title">
                  <IconSparkles size={15} />
                  {t("main.cleanRegions")}
                </div>
                <span className="panel-count">{selectedCount}/8</span>
              </div>
              <div className="region-grid">
                {REGIONS.map((r) => (
                  <RegionCard
                    key={r.key}
                    label={t(`regions.${r.key}`)}
                    note={r.noteKey ? t(r.noteKey) : ""}
                    on={Boolean(selectedMask & r.bit)}
                    onClick={() => toggleRegion(r.bit)}
                  />
                ))}
              </div>
              <div className="region-actions">
                <button className="chipbtn" onClick={() => setSelectedMask(MASK_ALL)}>
                  {t("main.all")}
                </button>
                <button className="chipbtn" onClick={() => setSelectedMask(MASK_DEFAULT)}>
                  {t("main.default")}
                </button>
                <button className="chipbtn" onClick={() => setSelectedMask(0)}>
                  {t("main.none")}
                </button>
              </div>
            </section>

            <button className="clean-btn" onClick={handleClean} disabled={cleaning}>
              {cleaning ? null : <IconBolt size={20} />}
              {cleaning ? t("main.cleaning") : t("main.cleanMemory")}
            </button>

            <div className="result">
              {lastResult ? (
                <>
                  {t("main.released")}{" "}
                  <strong>{formatBytes(lastResult.freed_bytes)}</strong>
                  {lastResult.regions.length > 0 &&
                    ` · ${lastResult.regions.length} ${t("main.regionsCount")}`}
                </>
              ) : (
                <>&nbsp;</>
              )}
            </div>
          </>
        ) : config ? (
          <SettingsPanel config={config} t={t} onSave={saveConfigAndReload} />
        ) : null}
      </main>

      {confirmMask !== null && (
        <div className="modal-overlay" onClick={() => setConfirmMask(null)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-title">
              <IconSparkles size={17} />
              {t("confirm.title")}
            </div>
            <ul className="modal-list">
              {REGIONS.filter((r) => confirmMask & r.bit).map((r) => (
                <li key={r.key}>{t(`regions.${r.key}`)}</li>
              ))}
            </ul>
            <div className="modal-actions">
              <button
                className="btn-primary"
                onClick={() => {
                  setConfirmMask(null);
                  runClean(confirmMask);
                }}
              >
                {t("confirm.clean")}
              </button>
              <button className="btn-ghost" onClick={() => setConfirmMask(null)}>
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
  icon,
  title,
  obj,
  t,
}: {
  icon: React.ReactNode;
  title: string;
  obj?: { total_bytes: number; free_bytes: number; used_bytes: number; percent: number };
  t: (k: string) => string;
}) {
  if (!obj) return null;
  const barClass =
    obj.percent >= 90 ? "bar-fill danger" : obj.percent >= 70 ? "bar-fill warn" : "bar-fill";
  return (
    <div className="metric">
      <div className="metric-icon">{icon}</div>
      <div className="metric-body">
        <div className="metric-top">
          <span className="metric-title">{title}</span>
          <span className="metric-value">{formatBytes(obj.used_bytes)}</span>
        </div>
        <div className="metric-sub">
          {t("main.of")} {formatBytes(obj.total_bytes)} · {obj.percent}%
        </div>
        <div className="bar">
          <div className={barClass} style={{ width: `${obj.percent}%` }} />
        </div>
      </div>
    </div>
  );
}

function RegionCard({
  label,
  note,
  on,
  onClick,
}: {
  label: string;
  note: string;
  on: boolean;
  onClick: () => void;
}) {
  return (
    <label className={`region ${on ? "on" : ""}`} onClick={onClick}>
      <span className="check">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M20 6L9 17l-5-5" />
        </svg>
      </span>
      <span className="region-body">
        <span>{label}</span>
        {note && <span className="region-note">{note}</span>}
      </span>
    </label>
  );
}

type Section = "general" | "memory" | "appearance" | "tray" | "advanced";

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
  const [section, setSection] = useState<Section>("general");

  useEffect(() => {
    setDraft(config);
  }, [config]);

  const set = <K extends keyof Config>(k: K, v: Config[K]) => {
    setDraft((d) => ({ ...d, [k]: v }));
  };

  const sections: { id: Section; icon: React.ReactNode }[] = [
    { id: "general", icon: <IconSettings size={14} /> },
    { id: "memory", icon: <IconBolt size={14} /> },
    { id: "appearance", icon: <IconPalette size={14} /> },
    { id: "tray", icon: <IconTray size={14} /> },
    { id: "advanced", icon: <IconKeyboard size={14} /> },
  ];

  return (
    <div className="settings">
      <div className="settings-tabs glass">
        {sections.map((s) => (
          <button
            key={s.id}
            className={section === s.id ? "active" : ""}
            onClick={() => setSection(s.id)}
          >
            {s.icon}
            {t(`settings.${s.id}`)}
          </button>
        ))}
      </div>

      <div className="settings-body">
        <div className="setgroup">
          <div className="setgroup-title">{t(`settings.${section}`)}</div>
          {section === "general" && (
            <>
              <Toggle label={t("settings.alwaysOnTop")} icon={<IconSettings size={15} />} checked={draft.always_on_top} onChange={(v) => set("always_on_top", v)} />
              <Toggle label={t("settings.startMinimized")} icon={<IconSettings size={15} />} checked={draft.start_minimized} onChange={(v) => set("start_minimized", v)} />
              <Toggle label={t("settings.showCleanConfirmation")} icon={<IconSparkles size={15} />} checked={draft.show_reduct_confirmation} onChange={(v) => set("show_reduct_confirmation", v)} />
              <Toggle label={t("settings.checkUpdates")} icon={<IconSparkles size={15} />} checked={draft.check_updates} onChange={(v) => set("check_updates", v)} />
              <Toggle label={t("settings.darkTheme")} icon={<IconPalette size={15} />} checked={draft.use_dark_theme} onChange={(v) => set("use_dark_theme", v)} />
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconDrive size={15} /></span>
                  {t("app.language")}
                </span>
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
              <Toggle label={t("settings.autoReduct")} icon={<IconBolt size={15} />} checked={draft.autoreduct_enable} onChange={(v) => set("autoreduct_enable", v)} />
              <Slider label={t("settings.autoReductThreshold")} value={draft.autoreduct_value} min={0} max={100} onChange={(v) => set("autoreduct_value", v)} />
              <Toggle label={t("settings.autoReductInterval")} icon={<IconBell size={15} />} checked={draft.autoreduct_interval_enable} onChange={(v) => set("autoreduct_interval_enable", v)} />
              <Slider label={t("settings.interval")} value={draft.autoreduct_interval_value} min={1} max={1440} onChange={(v) => set("autoreduct_interval_value", v)} />
              <Toggle label={t("settings.allowStandbyCleanup")} icon={<IconShield size={15} />} checked={draft.allow_standby_list_cleanup} onChange={(v) => set("allow_standby_list_cleanup", v)} />
              <div className="hint">{t("settings.standbyHint")}</div>
            </>
          )}

          {section === "appearance" && (
            <>
              <ColorRow label={t("settings.textColor")} value={configHex(draft.tray_color_text)} onChange={(v) => set("tray_color_text", parseHex(v))} />
              <ColorRow label={t("settings.backgroundColor")} value={configHex(draft.tray_color_bg)} onChange={(v) => set("tray_color_bg", parseHex(v))} />
              <ColorRow label={t("settings.warningColor")} value={configHex(draft.tray_color_warning)} onChange={(v) => set("tray_color_warning", parseHex(v))} />
              <ColorRow label={t("settings.dangerColor")} value={configHex(draft.tray_color_danger)} onChange={(v) => set("tray_color_danger", parseHex(v))} />
              <Toggle label={t("settings.trayBorder")} icon={<IconTray size={15} />} checked={draft.tray_show_border} onChange={(v) => set("tray_show_border", v)} />
              <Toggle label={t("settings.trayRoundCorners")} icon={<IconTray size={15} />} checked={draft.tray_round_corners} onChange={(v) => set("tray_round_corners", v)} />
              <Toggle label={t("settings.trayTransparency")} icon={<IconTray size={15} />} checked={draft.tray_use_transparency} onChange={(v) => set("tray_use_transparency", v)} />
              <Toggle label={t("settings.trayChangeBg")} icon={<IconPalette size={15} />} checked={draft.tray_change_bg} onChange={(v) => set("tray_change_bg", v)} />
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
              <Toggle label={t("settings.notificationSound")} icon={<IconBell size={15} />} checked={draft.notifications_sound} onChange={(v) => set("notifications_sound", v)} />
              <Toggle label={t("settings.showCleanResult")} icon={<IconSparkles size={15} />} checked={draft.balloon_clean_results} onChange={(v) => set("balloon_clean_results", v)} />
              <Toggle label={t("settings.logCleanResults")} icon={<IconDrive size={15} />} checked={draft.log_clean_results} onChange={(v) => set("log_clean_results", v)} />
              <Toggle label={t("settings.hotkeyClean")} icon={<IconKeyboard size={15} />} checked={draft.hotkey_clean_enable} onChange={(v) => set("hotkey_clean_enable", v)} />
              <div className="hint">{t("settings.hotkeyHint")}</div>
            </>
          )}
        </div>
      </div>

      <div className="settings-footer">
        <button className="btn-ghost" onClick={() => setDraft(config)}>
          {t("settings.reset")}
        </button>
        <button className="btn-primary" onClick={() => onSave(draft)}>
          {t("settings.save")}
        </button>
      </div>
    </div>
  );
}

function Toggle({
  label,
  icon,
  checked,
  onChange,
}: {
  label: string;
  icon?: React.ReactNode;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="setrow">
      <span className="setrow-label">
        {icon && <span className="icon">{icon}</span>}
        {label}
      </span>
      <span className="switch">
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
        <span className="track" />
        <span className="thumb" />
      </span>
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
    <div className="setrow slider">
      <div className="slider-head">
        <span className="setrow-label">{label}</span>
        <strong>{value}</strong>
      </div>
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
    <div className="setrow">
      <span className="setrow-label">{label}</span>
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
    <div className="setrow">
      <span className="setrow-label">{label}</span>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <input className="hexinput" value={value} readOnly />
        <input
          type="color"
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      </div>
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
