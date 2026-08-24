import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import i18n from "./i18n";
import {
  checkForUpdate,
  cleanMemory,
  downloadAndInstall,
  getConfig,
  getConfigLocation,
  getMemoryInfo,
  getOsInfo,
  isElevated,
  notify,
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

interface Toast {
  id: number;
  title: string;
  body: string;
  kind: "info" | "success";
}

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
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [toastSeq, setToastSeq] = useState(0);

  // Theme: "light" | "dark" | "system" (the legacy use_dark_theme flag is
  // honoured only for configs saved before the three-state theme existed).
  const [systemDark, setSystemDark] = useState<boolean>(
    () => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false
  );
  const [resolvedDark, setResolvedDark] = useState<boolean>(false);

  useEffect(() => {
    const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!mq) return;
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    setSystemDark(mq.matches);
    mq.addEventListener?.("change", onChange);
    return () => mq.removeEventListener?.("change", onChange);
  }, []);

  useEffect(() => {
    const theme = config?.theme ?? "system";
    let dark = systemDark;
    if (theme === "light") dark = false;
    else if (theme === "dark") dark = true;
    // Legacy fallback for old configs without a `theme` field.
    else if (config && config.use_dark_theme && theme === "system") dark = true;
    setResolvedDark(dark);
    document.body.classList.toggle("dark", dark);
  }, [config?.theme, config?.use_dark_theme, systemDark]);

  const pushToast = (title: string, body: string, kind: "info" | "success" = "info") => {
    const id = Date.now() + toastSeq;
    setToastSeq((s) => s + 1);
    setToasts((ts) => [...ts, { id, title, body, kind }]);
    setTimeout(() => {
      setToasts((ts) => ts.filter((t) => t.id !== id));
    }, 4200);
  };

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
      // Startup auto-update check (if enabled and a repo is configured).
      if (c.check_updates && c.update_repo.trim()) {
        checkForUpdate(c.update_repo, c.update_pubkey)
          .then((r) => {
            if (r.available) {
              pushToast(t("settings.updateAvailable"), `v${r.version}`, "info");
            }
          })
          .catch(() => {});
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
    const unlistenToast = listen<{ title: string; body: string }>("app-toast", (e) => {
      pushToast(e.payload.title, e.payload.body, "info");
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
      unlistenToast.then((fn) => fn());
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
      // In-app toast + system notification.
      const body = res.elevation_requested
        ? t("main.elevationRequested")
        : `${t("main.released")} ${formatBytes(res.freed_bytes)}${
            res.regions.length > 0
              ? ` · ${res.regions.length} ${t("main.regionsCount")}`
              : ""
          }`;
      if (config?.balloon_clean_results ?? true) {
        pushToast(
          t("main.cleanMemory"),
          body,
          res.elevation_requested ? "info" : "success"
        );
        notify(t("app.name"), body, config?.notifications_sound ?? true).catch(() => {});
      }
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
    <div className={`app ${resolvedDark ? "dark" : ""}`}>
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
                lastResult.elevation_requested ? (
                  <>{t("main.elevationRequested")}</>
                ) : (
                  <>
                    {t("main.released")}{" "}
                    <strong>{formatBytes(lastResult.freed_bytes)}</strong>
                    {lastResult.regions.length > 0 &&
                      ` · ${lastResult.regions.length} ${t("main.regionsCount")}`}
                  </>
                )
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

      <div className="toasts">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.kind}`}>
            <span className="toast-title">{toast.title}</span>
            <span className="toast-body">{toast.body}</span>
          </div>
        ))}
      </div>
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
    setDraft((d) => {
      const next = { ...d, [k]: v };
      onSave(next);
      return next;
    });
  };

  const runUpdateCheck = async (c: Config) => {
    try {
      const r = await checkForUpdate(c.update_repo, c.update_pubkey);
      if (r.available) {
        notify(t("settings.updateAvailable"), `v${r.version}`, true).catch(() => {});
      } else {
        notify(t("settings.updateNone"), `${t("settings.version")} ${r.current_version}`, true).catch(() => {});
      }
    } catch (e) {
      notify(t("settings.updateError"), String(e), true).catch(() => {});
    }
  };

  const runUpdateInstall = async (c: Config) => {
    try {
      await downloadAndInstall(c.update_repo, c.update_pubkey);
    } catch (e) {
      notify(t("settings.updateError"), String(e), true).catch(() => {});
    }
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
              <Toggle label={t("settings.showCleanConfirmation")} icon={<IconSparkles size={15} />} checked={draft.show_reduct_confirmation} onChange={(v) => set("show_reduct_confirmation", v)} />
              <Toggle label={t("settings.autoCheck")} icon={<IconSparkles size={15} />} checked={draft.check_updates} onChange={(v) => set("check_updates", v)} />
              <div className="setrow">
                <span className="setrow-label">{t("settings.updateRepo")}</span>
                <input
                  className="textinput"
                  placeholder="owner/repo"
                  value={draft.update_repo}
                  onChange={(e) => set("update_repo", e.target.value)}
                />
              </div>
              <div className="setrow">
                <span className="setrow-label">{t("settings.updatePubkey")}</span>
                <input
                  className="textinput"
                  placeholder="(可选)签名公钥"
                  value={draft.update_pubkey}
                  onChange={(e) => set("update_pubkey", e.target.value)}
                />
              </div>
              <div className="setrow">
                <span className="setrow-label">{t("settings.autoUpdate")}</span>
                <div className="setrow-actions">
                  <button className="chipbtn" onClick={() => runUpdateCheck(draft)}>
                    {t("settings.checkNow")}
                  </button>
                  <button className="chipbtn" onClick={() => runUpdateInstall(draft)}>
                    {t("settings.installNow")}
                  </button>
                </div>
              </div>
              <div className="hint">{t("settings.updateHint")}</div>
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
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconPalette size={15} /></span>
                  {t("settings.theme")}
                </span>
                <div className="segmented">
                  {(["light", "dark", "system"] as const).map((th) => (
                    <button
                      key={th}
                      className={draft.theme === th ? "active" : ""}
                      onClick={() => set("theme", th)}
                      type="button"
                    >
                      {t(`settings.theme_${th}`)}
                    </button>
                  ))}
                </div>
              </div>
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
              {draft.hotkey_clean_enable && (
                <div className="setrow">
                  <span className="setrow-label">{t("settings.hotkeyCombo")}</span>
                  <HotkeyRecorder value={draft.hotkey_clean} onChange={(v) => set("hotkey_clean", v)} />
                </div>
              )}
            </>
          )}
        </div>
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

// ---- Hotkey recorder ----
const MOD_ALT = 1;
const MOD_CTRL = 2;
const MOD_SHIFT = 4;
const MOD_WIN = 8;

function hotkeyLabel(value: number): string {
  const mods = (value >> 16) & 0xffff;
  const vk = value & 0xffff;
  const parts: string[] = [];
  if (mods & MOD_CTRL) parts.push("Ctrl");
  if (mods & MOD_ALT) parts.push("Alt");
  if (mods & MOD_SHIFT) parts.push("Shift");
  if (mods & MOD_WIN) parts.push("Win");
  // Virtual-key → readable name for common keys.
  if (vk >= 112 && vk <= 123) parts.push(`F${vk - 111}`);
  else if (vk >= 65 && vk <= 90) parts.push(String.fromCharCode(vk));
  else if (vk >= 48 && vk <= 57) parts.push(String.fromCharCode(vk));
  else if (vk === 32) parts.push("Space");
  else if (vk === 13) parts.push("Enter");
  else if (vk === 9) parts.push("Tab");
  else if (vk === 27) parts.push("Esc");
  else if (vk === 8) parts.push("Backspace");
  else if (vk === 46) parts.push("Delete");
  else if (vk === 37) parts.push("Left");
  else if (vk === 38) parts.push("Up");
  else if (vk === 39) parts.push("Right");
  else if (vk === 40) parts.push("Down");
  else parts.push(`VK${vk}`);
  return parts.join(" + ");
}

function HotkeyRecorder({
  value,
  onChange,
}: {
  value: number;
  onChange: (v: number) => void;
}) {
  const [recording, setRecording] = useState(false);

  const start = () => setRecording(true);

  useEffect(() => {
    if (!recording) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      let mods = 0;
      if (e.ctrlKey) mods |= MOD_CTRL;
      if (e.altKey) mods |= MOD_ALT;
      if (e.shiftKey) mods |= MOD_SHIFT;
      if (e.metaKey) mods |= MOD_WIN;
      const vk = e.keyCode || 0;
      // Only record real keys (ignore pure modifier presses).
      if (vk && vk !== 16 && vk !== 17 && vk !== 18 && vk !== 91 && vk !== 92) {
        onChange(((mods & 0xffff) << 16) | (vk & 0xffff));
      }
      setRecording(false);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [recording, onChange]);

  return (
    <button
      className={`hotkey-btn ${recording ? "recording" : ""}`}
      onClick={start}
      type="button"
    >
      {recording ? "按下组合键…" : hotkeyLabel(value)}
    </button>
  );
}

