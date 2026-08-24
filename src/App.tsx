import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
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
  const [tab, setTab] = useState<Tab>("main");
  const [info, setInfo] = useState<MemoryInfo | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [osInfo, setOsInfo] = useState<OsInfo | null>(null);
  const [configLocation, setConfigLocation] = useState<string>("");
  const [selectedMask, setSelectedMask] = useState<number>(MASK_DEFAULT);
  const [cleaning, setCleaning] = useState(false);
  const [lastResult, setLastResult] = useState<CleanResult | null>(null);

  useEffect(() => {
    getMemoryInfo().then(setInfo);
    getOsInfo().then(setOsInfo);
    getConfigLocation().then(setConfigLocation);
    getConfig().then((c) => {
      setConfig(c);
      setSelectedMask(c.reduct_mask);
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

  const handleClean = async () => {
    if (cleaning) return;
    // If admin required but not present, backend throws; backend already
    // handles elevation/notifying. We optimistically clean.
    setCleaning(true);
    setLastResult(null);
    try {
      const res = await cleanMemory(selectedMask, "manual");
      setLastResult(res);
    } catch (e) {
      console.error("clean failed", e);
    } finally {
      setCleaning(false);
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
          <span className="brand-name">Mem Reduct</span>
        </div>
        <nav className="tabs">
          <button
            className={tab === "main" ? "active" : ""}
            onClick={() => setTab("main")}
          >
            Main
          </button>
          <button
            className={tab === "settings" ? "active" : ""}
            onClick={() => setTab("settings")}
          >
            Settings
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
                  <div className="ring-label">Memory used</div>
                </div>
              </div>
              <div className="hero-cards">
                <MetricCard
                  title="Physical"
                  obj={info?.physical_memory}
                />
                <MetricCard title="Page file" obj={info?.page_file} />
                <MetricCard title="System cache" obj={info?.system_cache} />
              </div>
            </section>

            <section className="panel">
              <div className="panel-title">Clean regions</div>
              <div className="region-grid">
                {REGIONS.map((r) => (
                  <label key={r.key} className="region">
                    <input
                      type="checkbox"
                      checked={(selectedMask & r.bit) !== 0}
                      onChange={() => toggleRegion(r.bit)}
                    />
                    <span className="region-body">
                      <span>{r.label}</span>
                      {r.note && <span className="region-note">{r.note}</span>}
                    </span>
                  </label>
                ))}
              </div>
              <div className="region-actions">
                <button
                  className="ghost"
                  onClick={() => setSelectedMask(MASK_ALL)}
                >
                  All
                </button>
                <button
                  className="ghost"
                  onClick={() => setSelectedMask(MASK_DEFAULT)}
                >
                  Default
                </button>
              </div>
            </section>

            <button
              className="clean-btn"
              onClick={handleClean}
              disabled={cleaning}
            >
              {cleaning ? "Cleaning..." : "Clean Memory"}
            </button>

            {lastResult && (
              <div className="result">
                Freed <strong>{formatBytes(lastResult.freed_bytes)}</strong> —{" "}
                {lastResult.regions.join(", ") || "nothing"}
              </div>
            )}

            <footer className="footnote">
              Config:{" "}
              {configLocation === "portable" ? "portable" : "appdata"}
              {osInfo && ` · Win ${osInfo.major}.${osInfo.minor}`} · v3.5.3
            </footer>
          </>
        ) : (
          <SettingsPanel
            config={config}
            onSave={saveConfigAndReload}
          />
        )}
      </main>
    </div>
  );
}

function MetricCard({
  title,
  obj,
}: {
  title: string;
  obj?: { total_bytes: number; free_bytes: number; used_bytes: number; percent: number };
}) {
  if (!obj) return <div className="metric">—</div>;
  return (
    <div className="metric">
      <div className="metric-title">{title}</div>
      <div className="metric-value">{formatBytes(obj.used_bytes)}</div>
      <div className="metric-sub">
        of {formatBytes(obj.total_bytes)} ({obj.percent}%)
      </div>
      <div className="bar">
        <div className="bar-fill" style={{ width: `${obj.percent}%` }} />
      </div>
    </div>
  );
}

function SettingsPanel({
  config,
  onSave,
}: {
  config: Config | null;
  onSave: (c: Config) => void;
}) {
  if (!config) return null;

  const [draft, setDraft] = useState(config);
  const [section, setSection] = useState<
    "general" | "memory" | "appearance" | "tray" | "advanced"
  >("general");

  useEffect(() => {
    setDraft(config);
  }, [config]);

  const set = <K extends keyof Config>(k: K, v: Config[K]) => {
    setDraft((d) => ({ ...d, [k]: v }));
  };

  return (
    <div className="settings">
      <div className="settings-nav">
        {(["general", "memory", "appearance", "tray", "advanced"] as const).map(
          (s) => (
            <button
              key={s}
              className={section === s ? "active" : ""}
              onClick={() => setSection(s)}
            >
              {s[0].toUpperCase() + s.slice(1)}
            </button>
          )
        )}
      </div>

      <div className="settings-body">
        {section === "general" && (
          <>
            <Toggle
              label="Always on top"
              checked={draft.always_on_top}
              onChange={(v) => set("always_on_top", v)}
            />
            <Toggle
              label="Start minimized"
              checked={draft.start_minimized}
              onChange={(v) => set("start_minimized", v)}
            />
            <Toggle
              label="Show clean confirmation"
              checked={draft.show_reduct_confirmation}
              onChange={(v) => set("show_reduct_confirmation", v)}
            />
            <Toggle
              label="Check updates"
              checked={draft.check_updates}
              onChange={(v) => set("check_updates", v)}
            />
            <Toggle
              label="Dark theme"
              checked={draft.use_dark_theme}
              onChange={(v) => set("use_dark_theme", v)}
            />
          </>
        )}

        {section === "memory" && (
          <>
            <Toggle
              label="Auto-reduct by threshold"
              checked={draft.autoreduct_enable}
              onChange={(v) => set("autoreduct_enable", v)}
            />
            <Slider
              label="Auto-reduct threshold %"
              value={draft.autoreduct_value}
              min={0}
              max={100}
              onChange={(v) => set("autoreduct_value", v)}
            />
            <Toggle
              label="Auto-reduct by interval"
              checked={draft.autoreduct_interval_enable}
              onChange={(v) => set("autoreduct_interval_enable", v)}
            />
            <Slider
              label="Interval (minutes)"
              value={draft.autoreduct_interval_value}
              min={1}
              max={1440}
              onChange={(v) => set("autoreduct_interval_value", v)}
            />
            <Toggle
              label="Allow standby list cleanup in auto"
              checked={draft.allow_standby_list_cleanup}
              onChange={(v) => set("allow_standby_list_cleanup", v)}
            />
            <div className="hint">
              Standby/modified lists can cause brief freezes; disabled in auto
              by default.
            </div>
          </>
        )}

        {section === "appearance" && (
          <>
            <ColorRow
              label="Text color"
              value={configHex(draft.tray_color_text)}
              onChange={(v) => set("tray_color_text", parseHex(v))}
            />
            <ColorRow
              label="Background color"
              value={configHex(draft.tray_color_bg)}
              onChange={(v) => set("tray_color_bg", parseHex(v))}
            />
            <ColorRow
              label="Warning color"
              value={configHex(draft.tray_color_warning)}
              onChange={(v) => set("tray_color_warning", parseHex(v))}
            />
            <ColorRow
              label="Danger color"
              value={configHex(draft.tray_color_danger)}
              onChange={(v) => set("tray_color_danger", parseHex(v))}
            />
            <Toggle
              label="Tray shows border"
              checked={draft.tray_show_border}
              onChange={(v) => set("tray_show_border", v)}
            />
            <Toggle
              label="Tray round corners"
              checked={draft.tray_round_corners}
              onChange={(v) => set("tray_round_corners", v)}
            />
            <Toggle
              label="Tray transparency"
              checked={draft.tray_use_transparency}
              onChange={(v) => set("tray_use_transparency", v)}
            />
            <Toggle
              label="Tray change bg on warning/danger"
              checked={draft.tray_change_bg}
              onChange={(v) => set("tray_change_bg", v)}
            />
          </>
        )}

        {section === "tray" && (
          <>
            <Select
              label="Double-click action"
              value={draft.tray_action_dc}
              onChange={(v) => set("tray_action_dc", v)}
              options={[
                [0, "Show window"],
                [1, "Clean memory"],
              ]}
            />
            <Select
              label="Middle-click action"
              value={draft.tray_action_mc}
              onChange={(v) => set("tray_action_mc", v)}
              options={[
                [0, "Show window"],
                [1, "Clean memory"],
              ]}
            />
            <Slider
              label="Warning level %"
              value={draft.tray_level_warning}
              min={0}
              max={100}
              onChange={(v) => set("tray_level_warning", v)}
            />
            <Slider
              label="Danger level %"
              value={draft.tray_level_danger}
              min={0}
              max={100}
              onChange={(v) => set("tray_level_danger", v)}
            />
          </>
        )}

        {section === "advanced" && (
          <>
            <Toggle
              label="Notification sound"
              checked={draft.notifications_sound}
              onChange={(v) => set("notifications_sound", v)}
            />
            <Toggle
              label="Show clean result balloon"
              checked={draft.balloon_clean_results}
              onChange={(v) => set("balloon_clean_results", v)}
            />
            <Toggle
              label="Log clean results"
              checked={draft.log_clean_results}
              onChange={(v) => set("log_clean_results", v)}
            />
            <Toggle
              label="Global hotkey to clean"
              checked={draft.hotkey_clean_enable}
              onChange={(v) => set("hotkey_clean_enable", v)}
            />
            <div className="hint">Default hotkey: Ctrl+F1 (update via app)</div>
          </>
        )}
      </div>

      <div className="settings-footer">
        <button className="primary" onClick={() => onSave(draft)}>
          Save
        </button>
        <button className="ghost" onClick={() => setDraft(config)}>
          Reset
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
