import { memo, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import i18n from "./i18n";
import {
  applyTrayLabels,
  checkForUpdate,
  cleanMemory,
  downloadAndInstall,
  getAutostart,
  getConfig,
  getConfigLocation,
  getMemoryInfo,
  getOsInfo,
  getUpdaterInfo,
  getVersion,
  isElevated,
  notify,
  openExternal,
  saveConfig,
  setAutostart,
  type CleanDonePayload,
  type Config,
  type MemoryInfo,
  type UpdateInfo,
  type UpdaterInfo,
} from "./api";
import {
  MASK_ALL,
  MASK_DEFAULT,
  REGIONS,
  isRegionSupported,
  supportedMask,
  type OsCapabilities,
} from "./regions";
import { SUPPORTED_LANGUAGES, normalizeLanguage } from "./i18n";
import { ACCENTS, accentByKey } from "./accents";
import {
  IconBell,
  IconBolt,
  IconCache,
  IconChip,
  IconDownload,
  IconDrive,
  IconGauge,
  IconInfo,
  IconKeyboard,
  IconPalette,
  IconSettings,
  IconShield,
  IconSparkles,
  IconTray,
} from "./icons";

type Tab = "main" | "settings";

type ToastKind = "info" | "success" | "progress";

/**
 * Id base for the download-progress toast, kept clear of ordinary toast ids.
 *
 * (The "update available" prompt is no longer a toast at all — see
 * `availableUpdate` in the component: a sticky overlay sat on top of the header
 * and made the tab switcher unreachable.)
 */
const TOAST_PROGRESS_ID_BASE = 500000000;

interface Toast {
  id: number;
  title: string;
  body: string;
  kind: ToastKind;
  /** download progress in bytes downloaded (or 0..1) */
  progress?: number;
  /** total bytes for the progress bar */
  progressTotal?: number;
  /** action for the "update" kind button */
  action?: () => void;
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

/** Colour of the gauge ring / pressure chip, using the user's own thresholds. */
function pressureColor(percent: number, warnLevel: number, dangerLevel: number): string {
  if (percent >= dangerLevel) return "var(--danger)";
  if (percent >= warnLevel) return "var(--warning)";
  return "var(--accent)";
}

/** Default thresholds, matching `Config::default()` on the Rust side. */
const DEFAULT_WARN_LEVEL = 70;
const DEFAULT_DANGER_LEVEL = 90;

/** Result of the most recent update check, shown on the About page. */
interface CheckOutcome {
  ok: boolean;
  /** Version announced when `ok` and an update exists ("" when up to date). */
  version: string;
  /** Human-readable detail: the failure reason, or the version we are on. */
  detail: string;
  /** Whether a newer version was found. */
  available: boolean;
  at: string;
}

/**
 * First bullet of a release-notes body, for the one-line update prompt.
 *
 * The prompt is a single-line pill, so the full notes cannot be shown there;
 * picking the first bullet at least tells the user *what* the update is about
 * instead of only its number.
 */
function notesFirstLine(notes: string): string {
  const line = notes
    .split("\n")
    .map((l) => l.trim())
    .find((l) => l.startsWith("-") || l.startsWith("*"));
  if (!line) return "";
  return line.replace(/^[-*]\s*/, "").trim();
}

/** Respect the OS "reduce motion" accessibility setting. */
function usePrefersReducedMotion(): boolean {
  const [reduce, setReduce] = useState(false);
  useEffect(() => {
    const mq = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    if (!mq) return;
    const onChange = (e: MediaQueryListEvent) => setReduce(e.matches);
    setReduce(mq.matches);
    mq.addEventListener?.("change", onChange);
    return () => mq.removeEventListener?.("change", onChange);
  }, []);
  return reduce;
}

/**
 * Smoothly move a percentage towards `value` by writing to the DOM directly.
 *
 * Deliberately *not* React state: the sample arrives once per second and a
 * state-driven tween would re-render the whole tree ~60×/s, which is the wrong
 * trade for a memory tool. Attach the returned ref to an element rendered empty
 * — the effect fills in the text (including the first paint).
 */
function usePercentTween(value: number, reduceMotion: boolean) {
  const ref = useRef<HTMLDivElement | null>(null);
  const shown = useRef(value);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const write = (v: number) => {
      el.textContent = `${v}%`;
    };
    const from = shown.current;
    if (reduceMotion || from === value) {
      shown.current = value;
      write(value);
      return;
    }
    let raf = 0;
    const started = performance.now();
    const tick = (now: number) => {
      // Clamp: an rAF timestamp is the *frame's* start time and can be earlier
      // than the `performance.now()` captured here, which would make `t` negative
      // and send the eased value off in the opposite direction — the number ran
      // away to values like -23830% before this guard.
      const t = Math.min(1, Math.max(0, (now - started) / 600));
      const eased = 1 - Math.pow(1 - t, 3);
      const v = Math.round(from + (value - from) * eased);
      shown.current = v;
      write(v);
      if (t < 1) {
        raf = requestAnimationFrame(tick);
      } else {
        shown.current = value;
        write(value);
      }
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [value, reduceMotion]);

  return ref;
}

/**
 * Cursor-following highlight on cards — a port of React Bits' `SpotlightCard`,
 * which is itself nothing but two CSS variables plus a `::before` radial
 * gradient. Two deliberate changes:
 *
 * - **Delegated**: one `pointermove` listener on the app root instead of a React
 *   handler per card, so the cards stay dumb and no extra listeners are created.
 * - **No React state**: the position is written straight to a CSS variable. A
 *   state-driven version would re-render the tree on every mouse move.
 *
 * Only elements carrying `data-spot` react, and the write is throttled to one
 * per frame (a `getBoundingClientRect` per move would force layout).
 */
function useCardSpotlight<T extends HTMLElement>() {
  const ref = useRef<T | null>(null);
  useEffect(() => {
    const root = ref.current;
    if (!root) return;
    let raf = 0;
    let pending: { el: HTMLElement; x: number; y: number } | null = null;
    const apply = () => {
      raf = 0;
      const job = pending;
      pending = null;
      if (!job) return;
      job.el.style.setProperty("--spot-x", `${job.x}px`);
      job.el.style.setProperty("--spot-y", `${job.y}px`);
    };
    const onMove = (e: PointerEvent) => {
      const el = (e.target as Element | null)?.closest?.("[data-spot]");
      if (!(el instanceof HTMLElement)) return;
      const rect = el.getBoundingClientRect();
      pending = { el, x: e.clientX - rect.left, y: e.clientY - rect.top };
      if (!raf) raf = requestAnimationFrame(apply);
    };
    root.addEventListener("pointermove", onMove);
    return () => {
      root.removeEventListener("pointermove", onMove);
      if (raf) cancelAnimationFrame(raf);
    };
  }, []);
  return ref;
}

/**
 * Geometry of the active tab, for the sliding pill behind it.
 *
 * React Bits' `PillNav` measures each item with `getBoundingClientRect` and
 * tweens it with GSAP; this keeps the measuring part (the only part that matters
 * — tab widths change with the language, so a percentage translate would drift)
 * and drops the dependency: the pill is CSS-transitioned instead.
 *
 * `offsetLeft`/`offsetWidth` are used rather than a rect because they do not
 * force a layout flush, and the nav is `position: relative` so they are relative
 * to it. A `ResizeObserver` re-measures when the labels change size.
 */
function useSlidingTab(activeIndex: number) {
  const navRef = useRef<HTMLElement | null>(null);
  const [pill, setPill] = useState<{ x: number; w: number } | null>(null);
  useLayoutEffect(() => {
    const nav = navRef.current;
    if (!nav) return;
    const measure = () => {
      const btn = nav.querySelectorAll("button")[activeIndex] as HTMLElement | undefined;
      if (btn) setPill({ x: btn.offsetLeft, w: btn.offsetWidth });
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(nav);
    return () => ro.disconnect();
  }, [activeIndex]);
  return { navRef, pill };
}

/**
 * Spark burst from the click point — a dependency-free take on React Bits'
 * `ClickSpark`.
 *
 * Fires only for the primary actions matching `selector`, not on every click:
 * spraying particles whenever a checkbox is toggled would be noise in a system
 * utility. Rendered as a fixed, non-interactive canvas so it never intercepts
 * input.
 */
function ClickSpark({
  selector,
  color,
  enabled,
}: {
  selector: string;
  color: string;
  enabled: boolean;
}) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const sizeToWindow = () => {
      canvas.width = Math.floor(window.innerWidth * dpr);
      canvas.height = Math.floor(window.innerHeight * dpr);
      canvas.style.width = `${window.innerWidth}px`;
      canvas.style.height = `${window.innerHeight}px`;
    };
    const clear = () => {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, canvas.width, canvas.height);
    };
    sizeToWindow();
    clear();

    if (!enabled) return;

    const LIFE = 420;
    let sparks: { x: number; y: number; angle: number; start: number }[] = [];
    let raf = 0;

    const frame = (now: number) => {
      clear();
      sparks = sparks.filter((s) => now - s.start < LIFE);
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.lineCap = "round";
      for (const s of sparks) {
        const p = (now - s.start) / LIFE;
        const distance = 3 + 20 * p;
        const cx = s.x + Math.cos(s.angle) * distance;
        const cy = s.y + Math.sin(s.angle) * distance;
        const len = 5 + 9 * (1 - p);
        ctx.globalAlpha = 1 - p;
        ctx.beginPath();
        ctx.moveTo(cx, cy);
        ctx.lineTo(cx + Math.cos(s.angle) * len, cy + Math.sin(s.angle) * len);
        ctx.stroke();
      }
      ctx.globalAlpha = 1;
      raf = sparks.length ? requestAnimationFrame(frame) : 0;
    };

    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as Element | null;
      if (!target?.closest?.(selector)) return;
      for (let i = 0; i < 9; i++) {
        sparks.push({
          x: e.clientX,
          y: e.clientY,
          angle: (Math.PI * 2 * i) / 9,
          start: performance.now(),
        });
      }
      if (!raf) raf = requestAnimationFrame(frame);
    };

    window.addEventListener("resize", sizeToWindow);
    window.addEventListener("pointerdown", onPointerDown);
    return () => {
      window.removeEventListener("resize", sizeToWindow);
      window.removeEventListener("pointerdown", onPointerDown);
      if (raf) cancelAnimationFrame(raf);
      clear();
    };
  }, [selector, color, enabled]);

  return <canvas ref={canvasRef} className="click-spark" aria-hidden="true" />;
}

/**
 * One-line summary of a cleanup result, shared by the manual path and the
 * `clean-done` events so both report identically.
 *
 * Failed regions are called out explicitly: reporting only "freed 0 B" made a
 * refused cleanup look like a successful no-op.
 */
function cleanResultBody(
  result: { freed_bytes: number; regions: string[]; failed: string[] },
  t: (key: string) => string
): string {
  const parts = [`${t("main.released")} ${formatBytes(result.freed_bytes)}`];
  if (result.regions.length > 0) {
    parts.push(`${result.regions.length} ${t("main.regionsCount")}`);
  }
  if (result.failed.length > 0) {
    parts.push(`${result.failed.length} ${t("main.failedCount")}`);
  }
  return parts.join(" · ");
}

export default function App() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("main");
  const [settingsSection, setSettingsSection] = useState<Section>("general");
  const [info, setInfo] = useState<MemoryInfo | null>(null);
  const [config, setConfig] = useState<Config | null>(null);
  const [osInfo, setOsInfo] = useState<OsCapabilities | null>(null);
  const [selectedMask, setSelectedMask] = useState<number>(MASK_DEFAULT);
  const [cleaning, setCleaning] = useState(false);
  const [confirmMask, setConfirmMask] = useState<number | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [version, setVersion] = useState<string>("");
  const [elevated, setElevated] = useState<boolean | null>(null);
  const [configLocation, setConfigLocation] = useState<string>("");
  /** Endpoint and version used by the updater (no network I/O). */
  const [updaterInfo, setUpdaterInfo] = useState<UpdaterInfo | null>(null);
  /** Outcome of the most recent update check, for the About page. */
  const [lastCheck, setLastCheck] = useState<CheckOutcome | null>(null);
  /** An announced update waiting for the user's decision (renders the banner). */
  const [availableUpdate, setAvailableUpdate] = useState<UpdateInfo | null>(null);
  const progressToastId = useRef<number | null>(null);
  /** Monotonic id source: `Date.now()` alone collides when two toasts are
   *  pushed within the same millisecond. */
  const toastIdSeq = useRef(0);
  /** Auto-dismiss timers, cleared on unmount so they can't fire late. */
  const toastTimers = useRef<number[]>([]);
  /**
   * The region mask the user last picked.
   *
   * Kept in a ref (not only in state) so a settings save that was already
   * debounced cannot round-trip an older `reduct_mask` and silently undo the
   * main screen's selection.
   */
  const maskRef = useRef<number>(MASK_DEFAULT);
  /** Live translation function for imperative callbacks created on mount. */
  const tRef = useRef<(key: string) => string>(() => "");
  /** Live `balloon_clean_results`, for the same mount-time listeners. */
  const balloonCleanResultsRef = useRef<boolean>(true);

  // Theme: "light" | "dark" | "system" (the legacy use_dark_theme flag is
  // honoured only for configs saved before the three-state theme existed).
  const [systemDark, setSystemDark] = useState<boolean>(
    () => window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false
  );
  const [resolvedDark, setResolvedDark] = useState<boolean>(false);
  /** All decorative motion is skipped when the OS asks for reduced motion. */
  const reduceMotion = usePrefersReducedMotion();

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

  // Apply the accent color preset. Only the base colour is injected; every
  // derived tone is computed from it in styles.css.
  useEffect(() => {
    const accent = accentByKey(config?.accent_color ?? "green");
    document.documentElement.style.setProperty("--accent-base", accent.primary);
  }, [config?.accent_color]);

  // Keep the native tray menu labels in sync with the app language.
  const pushTrayLabels = useCallback(() => {
    applyTrayLabels({
      show: i18n.t("tray.show"),
      clean: i18n.t("tray.clean"),
      settings: i18n.t("tray.settings"),
      website: i18n.t("tray.website"),
      about: i18n.t("tray.about"),
      exit: i18n.t("tray.exit"),
    }).catch(() => {});
  }, []);

  useEffect(() => {
    pushTrayLabels();
    i18n.on("languageChanged", pushTrayLabels);
    return () => {
      i18n.off("languageChanged", pushTrayLabels);
    };
  }, [pushTrayLabels]);

  // Every callback that the (memoised) settings panel receives must keep a
  // stable identity, otherwise `React.memo` is defeated and the whole panel —
  // dozens of rows plus its own effects — is reconciled on every 1 Hz memory
  // update even while it is hidden behind `display: none`.
  const dismissToast = useCallback((id: number) => {
    if (progressToastId.current === id) progressToastId.current = null;
    setToasts((ts) => ts.filter((t) => t.id !== id));
  }, []);

  const pushToast = useCallback(
    (
      title: string,
      body: string,
      kind: ToastKind = "info",
      opts?: {
        progress?: number;
        progressTotal?: number;
        action?: () => void;
        stickyId?: number;
      }
    ) => {
      toastIdSeq.current += 1;
      const id = opts?.stickyId ?? toastIdSeq.current;
      if (opts?.stickyId !== undefined) {
        // Upsert: if a toast with this sticky id exists, update it in place;
        // otherwise create it. (A pure "update existing" silently drops the first
        // push, which is why the update prompt never showed on a cold state.)
        setToasts((ts) => {
          if (ts.some((t) => t.id === opts.stickyId)) {
            return ts.map((t) =>
              t.id === opts.stickyId
                ? {
                    ...t,
                    title,
                    body,
                    progress: opts.progress,
                    progressTotal: opts.progressTotal,
                    action: opts.action,
                  }
                : t
            );
          }
          return [
            ...ts,
            {
              id,
              title,
              body,
              kind,
              progress: opts.progress,
              progressTotal: opts.progressTotal,
              action: opts.action,
            },
          ];
        });
        return id;
      }
      setToasts((ts) => [
        ...ts,
        {
          id,
          title,
          body,
          kind,
          progress: opts?.progress,
          progressTotal: opts?.progressTotal,
          action: opts?.action,
        },
      ]);
      if (kind !== "progress") {
        const timer = window.setTimeout(() => {
          toastTimers.current = toastTimers.current.filter((t) => t !== timer);
          setToasts((ts) => ts.filter((t) => t.id !== id));
        }, 4200);
        toastTimers.current.push(timer);
      }
      return id;
    },
    []
  );

  // Download & install with a live progress toast (fixed sticky id).
  const startUpdateInstall = useCallback(async () => {
    // The banner has done its job; the progress toast takes over.
    setAvailableUpdate(null);
    // A dedicated id range keeps progress toasts from colliding with the
    // monotonic counter used by ordinary toasts.
    toastIdSeq.current += 1;
    const stickyId = TOAST_PROGRESS_ID_BASE + toastIdSeq.current;
    progressToastId.current = stickyId;
    // `i18n.t` rather than the hook's `t`: this function is captured by the
    // mount-time update prompt, so a bound `t` would freeze the language at
    // startup.
    pushToast(i18n.t("settings.updateInstalling"), "0%", "progress", {
      stickyId,
      progress: 0,
      progressTotal: 100,
    });
    try {
      await downloadAndInstall();
      // On success the installer launches and the plugin exits the process.
    } catch (e) {
      progressToastId.current = null;
      dismissToast(stickyId);
      pushToast(i18n.t("settings.updateError"), String(e), "info");
    }
  }, [pushToast, dismissToast]);

  /**
   * Show the "new version" banner.
   *
   * Rendered inline above the content instead of as a toast: the prompt is
   * sticky, and a fixed overlay at the top of a 400 px window sat on top of the
   * brand/tab header, leaving the user unable to switch tabs until they dealt
   * with it. Only the first release-notes bullet fits on one line, which is
   * enough to say *what* the update is about.
   */
  const showUpdatePrompt = useCallback((info: UpdateInfo) => {
    setAvailableUpdate(info);
  }, []);

  /** Remember what the last check did, so the About page can report it. */
  const recordCheck = useCallback((outcome: Omit<CheckOutcome, "at">) => {
    setLastCheck({ ...outcome, at: new Date().toLocaleTimeString() });
  }, []);

  useEffect(() => {
    tRef.current = t;
  }, [t]);

  useEffect(() => {
    balloonCleanResultsRef.current = config?.balloon_clean_results ?? true;
  }, [config?.balloon_clean_results]);

  useEffect(() => {
    getMemoryInfo().then(setInfo).catch(() => {});
    getVersion().then(setVersion).catch(() => {});
    isElevated().then(setElevated).catch(() => {});
    getConfigLocation().then(setConfigLocation).catch(() => {});
    // OS capabilities gate the regions this machine can actually clean.
    getOsInfo().then(setOsInfo).catch(() => {});
    // Endpoint + current version for the About page's diagnostics.
    getUpdaterInfo().then(setUpdaterInfo).catch(() => {});
    getConfig().then((c) => {
      setConfig(c);
      maskRef.current = c.reduct_mask;
      setSelectedMask(c.reduct_mask);
      // Apply the persisted language before any translated text is built.
      const lang = normalizeLanguage(c.language);
      if (lang) i18n.changeLanguage(lang);
      // Startup update check (always on): show an interactive pill when a newer
      // version exists.
      //
      // A failure is *recorded* instead of being swallowed: without it the user
      // cannot tell "already up to date" apart from "the check never reached
      // GitHub", which is exactly what a filtering proxy produces. The About
      // page shows the outcome, with the reason (and the endpoint tried) in the
      // failure text itself.
      checkForUpdate()
        .then((r) => {
          recordCheck({
            ok: true,
            available: r.available,
            version: r.available ? r.version : "",
            detail: r.available ? `v${r.version}` : `v${r.current_version}`,
          });
          if (r.available) showUpdatePrompt(r);
        })
        .catch((e) => {
          recordCheck({ ok: false, available: false, version: "", detail: String(e) });
          console.warn("update check failed:", e);
        });
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
      setTab("settings");
      setSettingsSection("about");
    });
    // Cleanups started from the tray / hotkey / auto rule have no command caller
    // to report back to, so the backend announces them here.
    const unlistenClean = listen<CleanDonePayload>("clean-done", (e) => {
      const { source, result } = e.payload;
      if (source === "manual") return; // already reported by runClean
      if (!balloonCleanResultsRef.current) return;
      const body = cleanResultBody(result, tRef.current);
      pushToast(tRef.current("main.cleanMemory"), body, "success");
      notify(tRef.current("app.name"), body).catch(() => {});
    });
    // A hotkey that could not be registered used to fail silently while the
    // toggle kept claiming it was armed.
    const unlistenHotkey = listen<string>("hotkey-error", (e) => {
      pushToast(tRef.current("settings.hotkeyClean"), tRef.current("settings.hotkeyFailed"), "info");
      notify(tRef.current("settings.hotkeyClean"), tRef.current("settings.hotkeyFailed"), true).catch(
        () => {}
      );
      console.warn("hotkey registration failed:", e.payload);
    });
    // Closing the window only hides it; say so the first time.
    const unlistenHidden = listen("hidden-to-tray", () => {
      const body = tRef.current("main.hiddenToTray");
      pushToast(tRef.current("app.name"), body, "info");
      notify(tRef.current("app.name"), body, true).catch(() => {});
    });
    const unlistenProgress = listen<{ chunk: number; total: number | null }>(
      "update-progress",
      (e) => {
        const pid = progressToastId.current;
        if (pid === null) return;
        const total = e.payload.total ?? 0;
        const pct = total > 0 ? Math.round((e.payload.chunk / total) * 100) : 0;
        setToasts((ts) =>
          ts.map((t) =>
            t.id === pid
              ? { ...t, progress: pct, progressTotal: 100, body: `${pct}%` }
              : t
          )
        );
      }
    );

    // No polling: the Rust backend emits "memory-update" once per second.

    return () => {
      unlistenMemory.then((fn) => fn());
      unlistenAuto.then((fn) => fn());
      unlistenSettings.then((fn) => fn());
      unlistenAbout.then((fn) => fn());
      unlistenClean.then((fn) => fn());
      unlistenHotkey.then((fn) => fn());
      unlistenHidden.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      toastTimers.current.forEach((timer) => window.clearTimeout(timer));
      toastTimers.current = [];
    };
  }, []);

  const runClean = async (mask: number) => {
    if (cleaning) return;
    setCleaning(true);
    try {
      const res = await cleanMemory(mask, "manual");
      getMemoryInfo().then(setInfo).catch(() => {});
      // In-app toast + system notification.
      const body = cleanResultBody(res, t);
      if (config?.balloon_clean_results ?? true) {
        // A partially refused cleanup is not a success — say so with a neutral
        // style rather than a green check.
        pushToast(t("main.cleanMemory"), body, res.failed.length > 0 ? "info" : "success");
        notify(t("app.name"), body).catch(() => {});
      }
    } catch (e) {
      pushToast(t("main.cleanMemory"), String(e), "info");
      console.error("clean failed", e);
    } finally {
      setCleaning(false);
    }
  };

  const handleClean = () => {
    if (cleaning) return;
    if (selectedMask === 0) {
      pushToast(t("main.cleanMemory"), t("main.nothing"), "info");
      return;
    }
    if (config?.show_reduct_confirmation) {
      setConfirmMask(selectedMask);
    } else {
      runClean(selectedMask);
    }
  };

  /**
   * Apply a region mask to the UI and (optionally) persist it.
   *
   * Persisting matters: the tray menu, the global hotkey and the automatic rule
   * all clean `reduct_mask` from the config, so a selection that only lived in
   * React state made those entry points disagree with what the user had ticked —
   * and lost the choice on every restart.
   */
  const applyMask = (mask: number, persist: boolean) => {
    const effective = supportedMask(mask, osInfo);
    maskRef.current = effective;
    setSelectedMask(effective);
    if (!persist || !config) return;
    const next: Config = { ...config, reduct_mask: effective };
    setConfig(next);
    saveConfig(next).catch((e) => pushToast(i18n.t("settings.saveFailed"), String(e), "info"));
  };

  const saveConfigAndReload = useCallback(
    async (next: Config) => {
      // The settings panel never edits the region mask, so always carry the
      // freshest one over instead of round-tripping whatever the draft holds.
      const merged: Config = { ...next, reduct_mask: maskRef.current };
      setConfig(merged);
      setSelectedMask(maskRef.current);
      try {
        await saveConfig(merged);
      } catch (e) {
        pushToast(i18n.t("settings.saveFailed"), String(e), "info");
      }
    },
    [pushToast]
  );

  const toggleRegion = (bit: number) => {
    applyMask(selectedMask & bit ? selectedMask & ~bit : selectedMask | bit, true);
  };

  // Escape closes the confirmation dialog (the overlay click alone is not
  // discoverable, and there is no other keyboard route out of it).
  useEffect(() => {
    if (confirmMask === null) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setConfirmMask(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [confirmMask]);

  const phys = info?.physical_memory;
  const physPct = phys?.percent ?? 0;
  // Follow the user's own tray thresholds instead of a hardcoded 70/90 pair, so
  // the main screen and the tray icon agree on what "high" means.
  const warnLevel = config?.tray_level_warning ?? DEFAULT_WARN_LEVEL;
  const dangerLevel = config?.tray_level_danger ?? DEFAULT_DANGER_LEVEL;
  const pressure = physPct >= dangerLevel ? "crit" : physPct >= warnLevel ? "warn" : "ok";
  const selectedCount = REGIONS.filter(
    (r) => selectedMask & r.bit && isRegionSupported(r, osInfo)
  ).length;
  const supportedCount = REGIONS.filter((r) => isRegionSupported(r, osInfo)).length;
  // The ring number is written by a rAF loop instead of React (see the hook).
  const gaugeRef = usePercentTween(physPct, reduceMotion);
  // Delegated cursor spotlight for the cards carrying `data-spot`.
  const appRef = useCardSpotlight<HTMLDivElement>();
  // Sliding pill behind the active tab (measured, so it survives a language
  // change that resizes the labels).
  const { navRef, pill } = useSlidingTab(tab === "main" ? 0 : 1);
  return (
    <div className={`app ${resolvedDark ? "dark" : ""}`} ref={appRef}>
      <header className="topbar">
        <div className="brand">
          <div className="brand-mark">
            <IconBolt size={18} />
          </div>
          <span className="brand-name shiny-text">{t("app.name")}</span>
        </div>
        <nav className="tabs" ref={navRef}>
          {pill && (
            <span
              className="tab-pill"
              aria-hidden="true"
              style={{ transform: `translateX(${pill.x}px)`, width: pill.w }}
            />
          )}
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

      {availableUpdate && (
        <div className="update-banner" role="status">
          <span className="update-banner-icon">
            <IconDownload size={15} />
          </span>
          <div className="update-banner-text">
            <div className="update-banner-title">
              {t("settings.updateAvailable")}
              <span className="update-banner-version">v{availableUpdate.version}</span>
            </div>
            {notesFirstLine(availableUpdate.body) && (
              <div className="update-banner-note" title={availableUpdate.body}>
                {notesFirstLine(availableUpdate.body)}
              </div>
            )}
          </div>
          <button className="update-banner-btn" onClick={() => void startUpdateInstall()}>
            {t("settings.installNow")}
          </button>
          <button
            className="update-banner-close"
            title={t("settings.dismiss")}
            aria-label={t("settings.dismiss")}
            onClick={() => setAvailableUpdate(null)}
          >
            ✕
          </button>
        </div>
      )}

      <main className="content">
        <div style={{ display: tab === "main" ? "flex" : "none" , flexDirection: "column", gap: 14 }}>
          <>
            <div className="statusbar">
              <span className={`statuschip ${pressure}`}>
                <span className="dot" />
                {t(`status.${pressure}`)}
              </span>
              {elevated !== null && (
                <span
                  className={`statuschip ${elevated ? "elevated" : "plain"}`}
                  title={elevated ? t("main.elevatedHint") : t("main.notElevated")}
                >
                  <span className="dot" />
                  {elevated ? t("main.elevated") : t("main.limited")}
                </span>
              )}
            </div>

            <section className="hero glass" data-spot>
              <div className="gauge-wrap">
                <div
                  className="gauge"
                  style={
                    {
                      // The ring is driven by a registered custom property so the
                      // sweep can be transitioned in CSS instead of jumping once
                      // per second.
                      "--gauge-pct": physPct,
                      "--gauge-color": pressureColor(physPct, warnLevel, dangerLevel),
                    } as React.CSSProperties
                  }
                >
                  <div className="gauge-inner">
                    {/* Filled by usePercentTween through the DOM; rendering the
                        value here as well would fight the animation. */}
                    <div className="gauge-value" ref={gaugeRef} />
                    <div className="gauge-label">{t("main.memoryUsed")}</div>
                    <div className="gauge-sub">
                      {info ? formatBytes(info.physical_memory.used_bytes) : "—"}
                    </div>
                  </div>
                </div>
              </div>

              <div className="metrics">
                <MetricCard
                  index={0}
                  icon={<IconChip size={17} />}
                  title={t("main.physical")}
                  obj={info?.physical_memory}
                  t={t}
                  warnLevel={warnLevel}
                  dangerLevel={dangerLevel}
                />
                <MetricCard
                  index={1}
                  icon={<IconDrive size={17} />}
                  title={t("main.pageFile")}
                  obj={info?.page_file}
                  t={t}
                  warnLevel={warnLevel}
                  dangerLevel={dangerLevel}
                />
                <MetricCard
                  index={2}
                  icon={<IconCache size={17} />}
                  title={t("main.systemCache")}
                  obj={info?.system_cache}
                  t={t}
                  warnLevel={warnLevel}
                  dangerLevel={dangerLevel}
                />
              </div>
            </section>

            <section className="panel glass">
              <div className="panel-head">
                <div className="panel-title">
                  <IconSparkles size={15} />
                  {t("main.cleanRegions")}
                </div>
                <span className="panel-count">
                  {selectedCount}/{supportedCount}
                </span>
              </div>
              <div className="region-grid">
                {REGIONS.map((r, i) => {
                  const supported = isRegionSupported(r, osInfo);
                  return (
                    <RegionCard
                      key={r.key}
                      index={i}
                      label={t(`regions.${r.key}`)}
                      note={
                        supported
                          ? r.noteKey
                            ? t(r.noteKey)
                            : ""
                          : t("main.unsupported")
                      }
                      noteIsWarning={supported}
                      on={Boolean(selectedMask & r.bit)}
                      disabled={!supported}
                      onClick={() => toggleRegion(r.bit)}
                    />
                  );
                })}
              </div>
              <div className="region-actions">
                <button className="chipbtn" onClick={() => applyMask(MASK_ALL, true)}>
                  {t("main.all")}
                </button>
                <button className="chipbtn" onClick={() => applyMask(MASK_DEFAULT, true)}>
                  {t("main.default")}
                </button>
                <button className="chipbtn" onClick={() => applyMask(0, true)}>
                  {t("main.none")}
                </button>
              </div>
            </section>

            {/* React Bits' `StarBorder` idea, adapted to a filled button: while
                memory pressure is elevated the Clean button gets a sweep that
                travels around it, tinted with the same colour the tray icon
                would use. Render-time only — transform animation, so it stays on
                the compositor and the CPU cost is negligible. */}
            <button
              className={`clean-btn ${pressure === "ok" ? "" : "attention"}`}
              style={{ "--ring-color": pressureColor(physPct, warnLevel, dangerLevel) } as React.CSSProperties}
              onClick={handleClean}
              disabled={cleaning}
            >
              {pressure !== "ok" && <span className="press-ring" aria-hidden="true" />}
              {cleaning ? null : <IconBolt size={20} />}
              {cleaning ? t("main.cleaning") : t("main.cleanMemory")}
            </button>
          </>
        </div>
        <div style={{ display: tab === "settings" ? "flex" : "none", flex: 1, minHeight: 0 }}>
          {config ? (
            <SettingsPanel config={config} t={t} onSave={saveConfigAndReload} version={version} configLocation={configLocation} section={settingsSection} onSectionChange={setSettingsSection} onToast={pushToast} updaterInfo={updaterInfo} lastCheck={lastCheck} onCheckOutcome={recordCheck} onUpdate={showUpdatePrompt} />
          ) : null}
        </div>
      </main>

      {confirmMask !== null && (
        <div className="modal-overlay" onClick={() => setConfirmMask(null)}>
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-label={t("confirm.title")}
            onClick={(e) => e.stopPropagation()}
          >
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
                autoFocus
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

      {/* Screen readers announce toasts through a polite live region. */}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.kind}`}>
            <span className="toast-title">{toast.title}</span>
            <span className="toast-dot">·</span>
            <span className="toast-body">{toast.body}</span>
            {toast.kind === "progress" && (
              <span className="toast-progress">
                <span
                  className="toast-progress-fill"
                  style={{ width: `${toast.progress ?? 0}%` }}
                />
              </span>
            )}
            <button
              className="toast-close"
              title={t("settings.dismiss")}
              aria-label={t("settings.dismiss")}
              onClick={(e) => {
                e.stopPropagation();
                dismissToast(toast.id);
              }}
            >
              ✕
            </button>
          </div>
        ))}
      </div>

      {/* Decorative spark burst on the primary actions; disabled when the OS asks
          for reduced motion. */}
      <ClickSpark
        selector=".clean-btn, .btn-primary, .update-banner-btn"
        color={accentByKey(config?.accent_color ?? "green").primary}
        enabled={!reduceMotion}
      />
    </div>
  );
}

function MetricCard({
  icon,
  title,
  obj,
  t,
  warnLevel,
  dangerLevel,
  index,
}: {
  icon: React.ReactNode;
  title: string;
  obj?: { total_bytes: number; free_bytes: number; used_bytes: number; percent: number };
  t: (k: string) => string;
  warnLevel: number;
  dangerLevel: number;
  /** Position in the list, fed to the CSS as `--i` for the entrance stagger. */
  index: number;
}) {
  // Render placeholders instead of nothing until the first sample arrives, so
  // the card layout never pops in after the fact.
  const pct = obj?.percent ?? 0;
  const barClass =
    pct >= dangerLevel ? "bar-fill danger" : pct >= warnLevel ? "bar-fill warn" : "bar-fill";
  return (
    <div className="metric" data-spot style={{ "--i": index } as React.CSSProperties}>
      <div className="metric-icon">{icon}</div>
      <div className="metric-body">
        <div className="metric-top">
          <span className="metric-title">{title}</span>
          <span className="metric-value">{obj ? formatBytes(obj.used_bytes) : "—"}</span>
        </div>
        <div className="metric-sub">
          {t("main.of")} {obj ? formatBytes(obj.total_bytes) : "—"} · {pct}%
        </div>
        <div className="bar">
          <div className={barClass} style={{ width: `${pct}%` }} />
        </div>
      </div>
    </div>
  );
}

/**
 * A single clean-region toggle.
 *
 * Rendered as a real `role="checkbox"` button rather than a clickable `<label>`:
 * the previous markup was unreachable by keyboard and invisible to screen
 * readers, so the region selection could not be used without a mouse.
 */
function RegionCard({
  label,
  note,
  noteIsWarning,
  on,
  disabled,
  index,
  onClick,
}: {
  label: string;
  note: string;
  noteIsWarning: boolean;
  on: boolean;
  disabled: boolean;
  /** Position in the grid, fed to the CSS as `--i` for the entrance stagger. */
  index: number;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={on}
      aria-disabled={disabled}
      disabled={disabled}
      className={`region ${on ? "on" : ""} ${disabled ? "unsupported" : ""}`}
      data-spot
      style={{ "--i": index } as React.CSSProperties}
      onClick={disabled ? undefined : onClick}
      title={disabled ? note : undefined}
    >
      <span className="check">
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
          <path d="M20 6L9 17l-5-5" />
        </svg>
      </span>
      <span className="region-body">
        <span>{label}</span>
        {note && (
          <span className={noteIsWarning ? "region-note" : "region-note muted"}>{note}</span>
        )}
      </span>
    </button>
  );
}

type Section = "general" | "memory" | "appearance" | "tray" | "about";

/**
 * Settings panel.
 *
 * Memoised, together with the `useCallback`s the parent passes in: the backend
 * pushes a memory sample every second and the panel is kept mounted behind
 * `display: none`, so without this it would reconcile its (large) tree 60 times
 * per minute for nothing. It still re-renders on language change, because `t`
 * changes identity then.
 */
const SettingsPanel = memo(function SettingsPanel({
  config,
  t,
  onSave,
  version,
  configLocation,
  section,
  onSectionChange,
  onToast,
  updaterInfo,
  lastCheck,
  onCheckOutcome,
  onUpdate,
}: {
  config: Config;
  t: (k: string) => string;
  onSave: (c: Config) => void;
  version: string;
  configLocation: string;
  section: Section;
  onSectionChange: (s: Section) => void;
  onToast: (title: string, body: string, kind?: "info" | "success") => void;
  updaterInfo: UpdaterInfo | null;
  lastCheck: CheckOutcome | null;
  onCheckOutcome: (o: Omit<CheckOutcome, "at">) => void;
  onUpdate: (info: UpdateInfo) => void;
}) {
  const [draft, setDraft] = useState<Config>(config);
  const [autostart, setAutostartState] = useState<boolean>(false);
  const [updatePhase, setUpdatePhase] = useState<"idle" | "checking">("idle");
  /** Debounced persistence: sliders fire `set()` on every pixel of movement,
   *  and each save writes the config file and re-arms the global hotkey. */
  const saveTimer = useRef<number | null>(null);
  const pendingSave = useRef<Config | null>(null);
  // `onSave` is re-created by the parent on every render, so keep it in a ref:
  // depending on it directly would defeat the debounce below.
  const onSaveRef = useRef(onSave);
  useEffect(() => {
    onSaveRef.current = onSave;
  }, [onSave]);

  const flushSave = useCallback(() => {
    if (saveTimer.current !== null) {
      window.clearTimeout(saveTimer.current);
      saveTimer.current = null;
    }
    const next = pendingSave.current;
    if (next) {
      pendingSave.current = null;
      onSaveRef.current(next);
    }
  }, []);

  // Persist whatever is still pending when leaving the settings view.
  useEffect(() => flushSave, [flushSave]);

  useEffect(() => {
    setDraft(config);
  }, [config]);

  // Query the scheduled-task state once; keep it in sync with the switch.
  useEffect(() => {
    getAutostart().then(setAutostartState).catch(() => {});
  }, []);

  // Poll the scheduled-task state until it settles (used after the UAC prompt
  // is shown — the elevated helper writes the task asynchronously).
  const pollAutostart = () => {
    let tries = 0;
    const timer = setInterval(() => {
      getAutostart()
        .then((cur) => {
          setAutostartState(cur);
          tries += 1;
          // Stop as soon as the task exists (enabled) or after ~6s.
          if (cur || tries >= 20) {
            clearInterval(timer);
          }
        })
        .catch(() => {
          tries += 1;
          if (tries >= 20) clearInterval(timer);
        });
    }, 300);
  };

  const toggleAutostart = (enabled: boolean) => {
    // Optimistic UI: flip the switch immediately, then reconcile in background.
    setAutostartState(enabled);
    setAutostart(enabled)
      .then((status) => {
        if (status === "elevation_requested") {
          // UAC prompt was shown; poll until the task state lands.
          pollAutostart();
        } else {
          // Task created/removed synchronously: confirm the final state.
          setAutostartState(status === "installed");
        }
      })
      .catch(() => {
        // Roll back on failure and re-read the truth.
        setAutostartState(!enabled);
        getAutostart().then(setAutostartState).catch(() => {});
      });
  };

  // Update local state immediately, persist (debounced) shortly after.
  const set = <K extends keyof Config>(k: K, v: Config[K]) => {
    setDraft((d) => {
      const next = { ...d, [k]: v };
      pendingSave.current = next;
      if (saveTimer.current !== null) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(() => {
        saveTimer.current = null;
        const pending = pendingSave.current;
        if (pending) {
          pendingSave.current = null;
          onSaveRef.current(pending);
        }
      }, 300);
      return next;
    });
  };

  // Single "Check for updates" flow: check → if a new version exists, download
  // and install it in the background, then the app restarts automatically.
  // The GitHub endpoint is slow/unstable (measured 3–14s+ on this network), so
  // we guard with a hard timeout so the button never spins forever.
  // Single "check for updates" flow: check → if a new version exists, offer to
  // download and install it. The GitHub endpoint can be slow or unreachable
  // (a filtering proxy is the usual cause), so the request is bounded here as
  // well as in the backend, and the *reason* for a failure is reported instead
  // of a bare "check failed" — plus recorded for the About page.
  const runUpdateCheck = async () => {
    if (updatePhase !== "idle") return;
    setUpdatePhase("checking");
    try {
      const r = await Promise.race([
        checkForUpdate(),
        new Promise<never>((_, reject) =>
          setTimeout(() => reject(new Error(t("settings.updateTimeout"))), 35000)
        ),
      ]);
      if (r.available) {
        onCheckOutcome({ ok: true, available: true, version: r.version, detail: `v${r.version}` });
        setUpdatePhase("idle");
        // Delegate to the shared update flow (interactive pill + progress).
        onUpdate(r);
      } else {
        const detail = `${t("settings.version")} ${r.current_version}`;
        onCheckOutcome({ ok: true, available: false, version: "", detail });
        onToast(t("settings.updateNone"), detail, "success");
        notify(t("settings.updateNone"), detail, true).catch(() => {});
        setUpdatePhase("idle");
      }
    } catch (e) {
      onCheckOutcome({ ok: false, available: false, version: "", detail: String(e) });
      onToast(t("settings.updateError"), String(e), "info");
      notify(t("settings.updateError"), String(e), true).catch(() => {});
      setUpdatePhase("idle");
    }
  };

  const sections: { id: Section; icon: React.ReactNode }[] = [
    { id: "general", icon: <IconSettings size={14} /> },
    { id: "memory", icon: <IconBolt size={14} /> },
    { id: "appearance", icon: <IconPalette size={14} /> },
    { id: "tray", icon: <IconTray size={14} /> },
    { id: "about", icon: <IconInfo size={14} /> },
  ];

  return (
    <div className="settings">
      <div className="settings-tabs glass">
        {sections.map((s) => (
          <button
            key={s.id}
            className={section === s.id ? "active" : ""}
            onClick={() => onSectionChange(s.id)}
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
              <Toggle label={t("settings.autostart")} icon={<IconBolt size={15} />} checked={autostart} onChange={toggleAutostart} />
              <div className="hint">{t("settings.autostartHint")}</div>
              <Toggle label={t("settings.showCleanConfirmation")} icon={<IconSparkles size={15} />} checked={draft.show_reduct_confirmation} onChange={(v) => set("show_reduct_confirmation", v)} />
              <Toggle label={t("settings.startMinimized")} icon={<IconTray size={15} />} checked={draft.start_minimized} onChange={(v) => set("start_minimized", v)} />
              <Toggle label={t("settings.hotkeyClean")} icon={<IconKeyboard size={15} />} checked={draft.hotkey_clean_enable} onChange={(v) => set("hotkey_clean_enable", v)} />
              {draft.hotkey_clean_enable && (
                <div className="setrow">
                  <span className="setrow-label">{t("settings.hotkeyCombo")}</span>
                  <HotkeyRecorder value={draft.hotkey_clean} onChange={(v) => set("hotkey_clean", v)} />
                </div>
              )}
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconDrive size={15} /></span>
                  {t("app.language")}
                </span>
                <select
                  value={draft.language}
                  onChange={(e) => {
                    const lang = normalizeLanguage(e.target.value) ?? "zh-CN";
                    set("language", lang);
                    i18n.changeLanguage(lang);
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
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconPalette size={15} /></span>
                  {t("settings.accentColor")}
                </span>
                <div className="accent-swatches">
                  {ACCENTS.map((a) => (
                    <button
                      key={a.key}
                      className={`swatch ${draft.accent_color === a.key ? "active" : ""}`}
                      style={{ background: a.primary }}
                      title={t(a.nameKey)}
                      onClick={() => set("accent_color", a.key)}
                      type="button"
                      aria-label={t(a.nameKey)}
                    />
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
              <Toggle label={t("settings.showCleanResult")} icon={<IconSparkles size={15} />} checked={draft.balloon_clean_results} onChange={(v) => set("balloon_clean_results", v)} />
            </>
          )}

          {section === "about" && (
            <>
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconSparkles size={15} /></span>
                  {t("settings.checkUpdates")}
                </span>
                <button
                  className="chipbtn"
                  onClick={() => runUpdateCheck()}
                  disabled={updatePhase !== "idle"}
                >
                  {updatePhase === "checking" && <span className="spinner" />}
                  {updatePhase === "checking"
                    ? t("settings.checking")
                    : t("settings.checkNow")}
                </button>
              </div>
              <div className="setrow">
                <span className="setrow-label">{t("settings.version")}</span>
                <span className="setrow-value">{version ? `v${version}` : "…"}</span>
              </div>
              {/* Update diagnostics: without these, a failed check is
                  indistinguishable from "already up to date". */}
              <div className="setrow">
                <span className="setrow-label">{t("settings.updateLastCheck")}</span>
                <span className={`setrow-value ${lastCheck && !lastCheck.ok ? "bad" : ""}`}>
                  {lastCheck
                    ? `${lastCheck.at} · ${lastCheck.ok ? t("settings.updateCheckOk") : t("settings.updateCheckFailed")}`
                    : t("settings.updateNever")}
                </span>
              </div>
              {lastCheck && (
                <div className={`hint update-detail ${lastCheck.ok ? "" : "bad"}`}>
                  {lastCheck.available
                    ? `${t("settings.updateFound")} v${lastCheck.version}`
                    : lastCheck.detail}
                </div>
              )}
              {/* The single human-facing entry point. This row *is* "read the
                  release notes" — a second link with that label pointed at the
                  very same URL. The raw manifest URL is gone too: it is machine
                  JSON the user cannot act on, and a failed check already names
                  the endpoint in its message. */}
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconInfo size={15} /></span>
                  {t("settings.updatePage")}
                </span>
                {updaterInfo ? (
                  <a
                    className="link"
                    href={updaterInfo.release_page}
                    onClick={(e) => {
                      e.preventDefault();
                      openExternal(updaterInfo.release_page).catch(() => {});
                    }}
                  >
                    {t("settings.updatePageOpen")}
                  </a>
                ) : (
                  <span className="setrow-value">…</span>
                )}
              </div>
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconSettings size={15} /></span>
                  {t("main.configLocation")}
                </span>
                <span className="setrow-value">
                  {configLocation === "portable"
                    ? t("main.portable")
                    : configLocation === "appdata"
                      ? t("main.appdata")
                      : "…"}
                </span>
              </div>
              <div className="setrow">
                <span className="setrow-label">
                  <span className="icon"><IconInfo size={15} /></span>
                  {t("about.website")}
                </span>
                <a
                  className="link"
                  href="https://github.com/JackPolaris/memreduct-rs"
                  onClick={(e) => { e.preventDefault(); openExternal("https://github.com/JackPolaris/memreduct-rs").catch(() => {}); }}
                >
                  github.com/JackPolaris/memreduct-rs
                </a>
              </div>
              <div className="hint">{t("about.license")}</div>
            </>
          )}
        </div>
      </div>
    </div>
  );
});

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
        aria-label={label}
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
      <select value={value} aria-label={label} onChange={(e) => onChange(Number(e.target.value))}>
        {options.map(([v, l]) => (
          <option key={v} value={v}>
            {l}
          </option>
        ))}
      </select>
    </div>
  );
}

/**
 * Colour picker row.
 *
 * The hex field used to be `readOnly`, so the only way to enter a colour was the
 * OS picker. It now accepts typing — committing on blur/Enter and reverting on
 * anything that is not a valid `#rrggbb` — while staying in sync when the value
 * changes from elsewhere.
 */
function ColorRow({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  const [text, setText] = useState(value);
  useEffect(() => {
    setText(value);
  }, [value]);

  const commit = (raw: string) => {
    const match = raw.trim().match(/^#?([0-9a-fA-F]{6})$/);
    if (match) {
      onChange(`#${match[1].toLowerCase()}`);
    } else {
      setText(value);
    }
  };

  return (
    <div className="setrow">
      <span className="setrow-label">{label}</span>
      <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <input
          className="hexinput"
          value={text}
          aria-label={label}
          spellCheck={false}
          onChange={(e) => setText(e.target.value)}
          onBlur={(e) => commit(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
          }}
        />
        <input
          type="color"
          value={value}
          aria-label={label}
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
  if (vk >= 112 && vk <= 135) parts.push(`F${vk - 111}`);
  else if (vk >= 65 && vk <= 90) parts.push(String.fromCharCode(vk));
  else if (vk >= 48 && vk <= 57) parts.push(String.fromCharCode(vk));
  else if (vk >= 96 && vk <= 105) parts.push(`Num${vk - 96}`);
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

/**
 * Translate a keyboard event into a Windows virtual-key code.
 *
 * Uses `KeyboardEvent.code` (layout independent) instead of the deprecated
 * `keyCode`. Returns `null` for keys that cannot be part of a hotkey (pure
 * modifiers, media keys, …).
 */
function toVirtualKey(e: KeyboardEvent): number | null {
  const code = e.code ?? "";
  if (/^Key[A-Z]$/.test(code)) return code.charCodeAt(3);
  if (/^Digit[0-9]$/.test(code)) return code.charCodeAt(5);
  if (/^Numpad[0-9]$/.test(code)) return 96 + Number(code.slice(6));
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return 112 + Number(code.slice(1)) - 1;
  const special: Record<string, number> = {
    Space: 32,
    Enter: 13,
    NumpadEnter: 13,
    Tab: 9,
    Escape: 27,
    Backspace: 8,
    Delete: 46,
    Insert: 45,
    Home: 36,
    End: 35,
    PageUp: 33,
    PageDown: 34,
    ArrowLeft: 37,
    ArrowUp: 38,
    ArrowRight: 39,
    ArrowDown: 40,
    Minus: 189,
    Equal: 187,
    BracketLeft: 219,
    BracketRight: 221,
    Backslash: 220,
    Semicolon: 186,
    Quote: 222,
    Comma: 188,
    Period: 190,
    Slash: 191,
    Backquote: 192,
  };
  return special[code] ?? null;
}

/** Function keys work standalone; every other key needs a modifier. */
function isFunctionKey(vk: number): boolean {
  return vk >= 112 && vk <= 135;
}

function HotkeyRecorder({
  value,
  onChange,
}: {
  value: number;
  onChange: (v: number) => void;
}) {
  const { t } = useTranslation();
  const [recording, setRecording] = useState(false);
  const [invalid, setInvalid] = useState(false);

  const start = () => {
    setInvalid(false);
    setRecording(true);
  };

  useEffect(() => {
    if (!recording) return;
    const handler = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const vk = toVirtualKey(e);
      // Ignore pure modifier presses while recording.
      if (vk === null) return;
      let mods = 0;
      if (e.ctrlKey) mods |= MOD_CTRL;
      if (e.altKey) mods |= MOD_ALT;
      if (e.shiftKey) mods |= MOD_SHIFT;
      if (e.metaKey) mods |= MOD_WIN;
      // A bare letter/digit would swallow that key globally — require a
      // modifier unless the key is a function key.
      if (mods === 0 && !isFunctionKey(vk)) {
        setInvalid(true);
        return;
      }
      onChange(((mods & 0xffff) << 16) | (vk & 0xffff));
      setRecording(false);
      setInvalid(false);
    };
    window.addEventListener("keydown", handler, true);
    return () => window.removeEventListener("keydown", handler, true);
  }, [recording, onChange]);

  return (
    <>
      <button
        className={`hotkey-btn ${recording ? "recording" : ""}`}
        onClick={start}
        type="button"
      >
        {recording ? t("settings.hotkeyRecording") : hotkeyLabel(value)}
      </button>
      {invalid && <span className="hint">{t("settings.hotkeyNeedModifier")}</span>}
    </>
  );
}

