import { listen } from "@tauri-apps/api/event";
import { useDialog } from "../../hooks/useDialog";
import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWheelSlider } from "../../hooks/useWheelSlider";
import { useProfileStore } from "../../stores/profileStore";
import { useRuntimeStore } from "../../stores/runtimeStore";
import { useExportStore } from "../../stores/exportStore";
import {
  formatShortcut,
  recordKeyLayout,
  getKeyCode,
  findMomentumConflicts,
  keyCodeToDisplay,
  buildShortcutsList,
  type MomentumModifierType,
} from "../../utils/keyMapping";
import * as commands from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { WarningTooltip } from "../common/WarningTooltip";
import { DislikedVideosPanel } from "./DislikedVideosPanel";
import { useMoodStore } from "../../stores/moodStore";
import type { LinuxInputAccessStatus, MomentumModifier } from "../../types";

interface SettingsModalProps {
  onClose: () => void;
}

type ShortcutTarget = "stopAll" | "autoMomentum" | "keyDetection" | null;

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-text-secondary text-xs font-semibold uppercase tracking-wide border-b border-border-color pb-1 mb-3">
      {children}
    </h3>
  );
}

function MoodAiSection({ config }: { config: import("../../types").AppConfig }) {
  const {
    status, presets, models, isTesting, isSigningIn, refreshServiceStatus, loadPresets, saveApiKey,
    testConnection, signIn, signOut, onAccount, exportExtension, clearCache,
  } = useMoodStore();
  const [keyDraft, setKeyDraft] = useState("");
  const [urlDraft, setUrlDraft] = useState(config.moodBaseUrl);
  const [exportedTo, setExportedTo] = useState<string | null>(null);
  const settings = useSettingsStore.getState();

  useEffect(() => { refreshServiceStatus(); loadPresets(); }, [refreshServiceStatus, loadPresets]);
  useEffect(() => setUrlDraft(config.moodBaseUrl), [config.moodBaseUrl]);

  useEffect(() => {
    const cleanups: (() => void)[] = [];
    let disposed = false;
    const register = (unlisten: () => void) => { if (disposed) unlisten(); else cleanups.push(unlisten); };
    for (const name of ["mood_api_status", "mood_progress"]) {
      listen(name, () => { refreshServiceStatus(); }).then(register);
    }
    listen<{ planEnabled: boolean; error: string | null }>("mood_account", (e) => { onAccount(e.payload); }).then(register);
    return () => { disposed = true; cleanups.forEach((u) => u()); };
  }, [refreshServiceStatus, onAccount]);

  const plan = config.moodAuth === "chatgpt";
  const account = status?.account ?? null;
  const canListModels = plan ? Boolean(account) : Boolean(status && (status.local || status.apiKey));
  // Load the account's models once it can answer.
  useEffect(() => {
    if (canListModels && models.length === 0) testConnection(true);
  }, [canListModels, config.moodAuth, config.moodBaseUrl, account?.email]); // eslint-disable-line react-hooks/exhaustive-deps
  const modelChoices = models.some((m) => m.id === config.moodModel)
    ? models
    : [{ id: config.moodModel, name: config.moodModel }, ...models];

  const preset = presets.find((p) => p.baseUrl === config.moodBaseUrl);
  const needsKey = !plan && (status ? !status.local : true);
  const chapter = status?.chapter ?? null;
  const inputClass = "bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary focus:border-border-focus outline-none";
  const buttonClass = "px-2 py-1 text-xs bg-accent-primary/20 text-accent-primary rounded hover:bg-accent-primary/30 disabled:opacity-50";

  return (
    <section>
      <SectionHeader>Manga Mood AI</SectionHeader>
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div>
            <label className="text-text-secondary text-sm font-medium">Enable Mood AI</label>
            <p className="text-text-muted text-xs">
              A vision model reads the chapter open in your browser, splits it into scenes and plays the keys tagged with each scene's mood.
            </p>
          </div>
          <button
            onClick={() => settings.setMoodAiEnabled(!config.moodAiEnabled)}
            className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors ${
              config.moodAiEnabled ? "bg-accent-primary" : "bg-bg-tertiary"
            }`}
          >
            <span className={`inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform ${
              config.moodAiEnabled ? "translate-x-[18px]" : "translate-x-[2px]"
            }`} />
          </button>
        </div>

        {/* Provider */}
        <div className="space-y-2 rounded border border-border-color bg-bg-tertiary/40 p-3">
          <p className="text-text-secondary text-sm font-medium">Model</p>
          <div className="flex items-center gap-2">
            <label className="text-text-muted text-xs w-20">Pay with</label>
            <select
              value={config.moodAuth}
              onChange={(e) => { useMoodStore.setState({ models: [] }); settings.setMoodProvider({ moodAuth: e.target.value as "chatgpt" | "api_key" }); }}
              className={`${inputClass} flex-1`}
            >
              <option value="chatgpt">My ChatGPT plan (Plus / Pro)</option>
              <option value="api_key">An API key or a local server</option>
            </select>
          </div>
          {plan && (
            <div className="flex items-center gap-2">
              <label className="text-text-muted text-xs w-20">Account</label>
              <span className="text-text-secondary text-sm flex-1 truncate">
                {account
                  ? account.planEnabled
                    ? `${account.email ?? "ChatGPT"} · Using ChatGPT plan`
                    : `${account.email ?? "ChatGPT"} · plan usage not allowed`
                  : isSigningIn
                    ? "Finish signing in in your browser…"
                    : "Not signed in"}
              </span>
              {!account?.planEnabled && (
                <button disabled={isSigningIn} onClick={signIn} className={buttonClass}>Continue with ChatGPT</button>
              )}
              {account && <button onClick={signOut} className="text-xs text-text-muted hover:underline">Sign out</button>}
              <button onClick={() => open("https://chatgpt.com/settings/usage")} className="text-xs text-text-muted hover:underline">
                Manage usage
              </button>
            </div>
          )}
          {!plan && (
          <div className="flex items-center gap-2">
            <label className="text-text-muted text-xs w-20">Provider</label>
            <select
              value={preset ? preset.baseUrl : "custom"}
              onChange={(e) => {
                if (e.target.value !== "custom") settings.setMoodProvider({ moodBaseUrl: e.target.value });
                else setUrlDraft("");
              }}
              className={`${inputClass} flex-1`}
            >
              {presets.map((p) => <option key={p.baseUrl} value={p.baseUrl}>{p.name}</option>)}
              <option value="custom">Other (OpenAI-compatible)</option>
            </select>
          </div>
          )}
          {!plan && !preset && (
            <div className="flex items-center gap-2">
              <label className="text-text-muted text-xs w-20">URL</label>
              <input
                value={urlDraft}
                placeholder="https://host/v1"
                onChange={(e) => setUrlDraft(e.target.value)}
                onBlur={() => urlDraft.trim() && urlDraft !== config.moodBaseUrl && settings.setMoodProvider({ moodBaseUrl: urlDraft.trim() })}
                className={`${inputClass} flex-1`}
              />
            </div>
          )}
          {needsKey && (
            <div className="flex items-center gap-2">
              <label className="text-text-muted text-xs w-20">API key</label>
              <input
                type="password"
                value={keyDraft}
                placeholder={status?.apiKey ? `Saved (${status.apiKey})` : "sk-…"}
                onChange={(e) => setKeyDraft(e.target.value)}
                className={`${inputClass} flex-1`}
              />
              <button
                disabled={!keyDraft.trim()}
                onClick={async () => { await saveApiKey(keyDraft.trim()); setKeyDraft(""); }}
                className={buttonClass}
              >
                Save
              </button>
              {status?.apiKey && (
                <button onClick={() => saveApiKey(null)} className="text-xs text-text-muted hover:underline">Remove</button>
              )}
            </div>
          )}
          <div className="flex items-center gap-2">
            <label className="text-text-muted text-xs w-20">Model</label>
            <select
              value={config.moodModel}
              onChange={(e) => settings.setMoodProvider({ moodModel: e.target.value })}
              className={`${inputClass} flex-1`}
            >
              {modelChoices.map((m) => <option key={m.id} value={m.id}>{m.name}</option>)}
            </select>
            <button disabled={isTesting || !canListModels} onClick={() => testConnection()} className={buttonClass}>
              {isTesting ? "Loading…" : "Refresh"}
            </button>
          </div>
          <div className="flex items-center gap-2">
            <label className="text-text-muted text-xs w-20">Pages</label>
            <select
              value={config.moodImageDetail}
              onChange={(e) => settings.setMoodProvider({ moodImageDetail: e.target.value as "low" | "high" })}
              className={`${inputClass} flex-1`}
            >
              <option value="high">Detailed — the model reads the dialogue</option>
              <option value="low">Economy — small thumbnails, cheaper</option>
            </select>
          </div>
          {status?.error && <p className="text-accent-error text-xs">{status.error}</p>}
        </div>

        {/* Extension */}
        <div className="space-y-2 rounded border border-border-color bg-bg-tertiary/40 p-3">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="text-text-secondary text-sm font-medium">Browser extension</p>
              <p className="text-text-muted text-xs">
                {status?.api === "running"
                  ? `Listening on 127.0.0.1:${status.port}.`
                  : config.moodAiEnabled
                    ? "The local API is not running."
                    : "Enable Mood AI to let the extension connect."}
              </p>
            </div>
            <button
              onClick={async () => setExportedTo(await exportExtension())}
              className={`${buttonClass} shrink-0`}
            >
              Install extension
            </button>
          </div>
          {exportedTo && (
            <p className="text-text-muted text-xs">
              Written to <span className="text-text-secondary">{exportedTo}</span>. Chrome/Edge/Brave: chrome://extensions → Developer mode → Load unpacked.
              Firefox: about:debugging → This Firefox → Load Temporary Add-on → manifest.json.
            </p>
          )}
          <div className="flex items-center gap-2">
            <label className="text-text-muted text-xs w-20">API port</label>
            <input
              type="number"
              min="1024"
              max="65535"
              value={config.moodApiPort}
              onChange={(e) => {
                const port = Number(e.target.value);
                if (port >= 1024 && port <= 65535) settings.setMoodApiPort(port);
              }}
              className={`${inputClass} w-24`}
            />
          </div>
        </div>

        {chapter && (
          <div className="space-y-1 rounded border border-border-color bg-bg-tertiary/40 p-3">
            <div className="flex items-center justify-between">
              <p className="text-text-secondary text-sm font-medium truncate">{chapter.title || "Current chapter"}</p>
              <button onClick={clearCache} className="text-xs text-text-muted hover:underline shrink-0">Forget analyzed chapters</button>
            </div>
            <p className="text-text-muted text-xs">
              {chapter.analyzed}/{chapter.totalPages} pages analyzed · page {chapter.focus + 1}
              {chapter.inFlight > 0 ? ` · ${chapter.inFlight} in progress` : ""}
            </p>
            {chapter.current && (
              <p className="text-text-muted text-xs">Scene: {chapter.current.mood} {chapter.current.intensity} — {chapter.current.summary}</p>
            )}
          </div>
        )}
      </div>
    </section>
  );
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const {
    config,
    setCrossfadeDuration,
    setKeyCooldown,
    setChordWindowMs,
    setStopAllShortcut,
    setAutoMomentumShortcut,
    setKeyDetectionShortcut,
    setAudioDevice,
    setMomentumModifier,
  } = useSettingsStore();
  const { currentProfile, loadProfiles, loadProfile } = useProfileStore();
  const inputRuntime = useRuntimeStore((s) => s.inputRuntime);
  const { isExporting, startExport } = useExportStore();
  const addToast = useToastStore((s) => s.addToast);
  const [capturingTarget, setCapturingTarget] = useState<ShortcutTarget>(null);
  const [capturedKeys, setCapturedKeys] = useState<string[]>([]);
  const pressedRef = useRef(new Set<string>());
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);
  const [linuxInputStatus, setLinuxInputStatus] = useState<LinuxInputAccessStatus | null>(null);
  const [linuxInputBusy, setLinuxInputBusy] = useState(false);

  useEffect(() => {
    commands.listAudioDevices().then(setAudioDevices).catch(() => {});
  }, []);

  const refreshLinuxInputStatus = useCallback(() => {
    if (!inputRuntime.isLinux) return;
    commands.getLinuxInputAccessStatus().then(setLinuxInputStatus).catch(() => {});
  }, [inputRuntime.isLinux]);

  useEffect(() => {
    refreshLinuxInputStatus();
  }, [refreshLinuxInputStatus]);

  const enableLinuxBackgroundDetection = useCallback(async () => {
    setLinuxInputBusy(true);
    try {
      const result = await commands.enableLinuxBackgroundDetection();
      setLinuxInputStatus(result.status);
      addToast(result.message, result.status.backgroundDetectionAvailable ? "success" : "info");

      window.setTimeout(() => {
        refreshLinuxInputStatus();
      }, 3000);
    } catch (error) {
      addToast(String(error), "error");
    } finally {
      setLinuxInputBusy(false);
    }
  }, [addToast, refreshLinuxInputStatus]);

  // Build shortcuts array for conflict detection
  const shortcuts = useMemo(() => buildShortcutsList(config),
    [config.stopAllShortcut, config.autoMomentumShortcut, config.keyDetectionShortcut]);

  // Detect conflicts between momentum modifier and shortcuts
  const momentumConflicts = useMemo(() => {
    if (!currentProfile) return [];
    return findMomentumConflicts(
      config.momentumModifier as MomentumModifierType,
      shortcuts,
      currentProfile.keyBindings
    );
  }, [config.momentumModifier, shortcuts, currentProfile]);

  const { ref: cooldownWheelRef, isWheelActive: cooldownWheelActive } = useWheelSlider({
    value: config.keyCooldown, min: 0, max: 2000, step: 50,
    onChange: setKeyCooldown,
  });
  const { ref: chordWheelRef, isWheelActive: chordWheelActive } = useWheelSlider({
    value: config.chordWindowMs, min: 20, max: 100, step: 5,
    onChange: setChordWindowMs,
  });
  const { ref: crossfadeWheelRef, isWheelActive: crossfadeWheelActive } = useWheelSlider({
    value: config.crossfadeDuration, min: 100, max: 2000, step: 50,
    onChange: setCrossfadeDuration,
  });

  // Get conflict for a specific shortcut by name
  const getShortcutConflict = (shortcutName: string) => {
    return momentumConflicts.find((c) => c.shortcutName === shortcutName);
  };

  const saveShortcut = useCallback(
    (keys: string[]) => {
      // Check if shortcut conflicts with momentum modifier + bound keys
      const newConflicts = currentProfile
        ? findMomentumConflicts(
            config.momentumModifier as MomentumModifierType,
            [{ name: "New shortcut", keys }],
            currentProfile.keyBindings
          )
        : [];

      if (newConflicts.length > 0) {
        const conflict = newConflicts[0];
        addToast(
          `Warning: Uses ${config.momentumModifier}+${keyCodeToDisplay(conflict.boundKey)}. Momentum won't work on this key.`,
          "warning"
        );
      }

      switch (capturingTarget) {
        case "stopAll":
          setStopAllShortcut(keys);
          break;
        case "autoMomentum":
          setAutoMomentumShortcut(keys);
          break;
        case "keyDetection":
          setKeyDetectionShortcut(keys);
          break;
      }
      setCapturingTarget(null);
      setCapturedKeys([]);
      pressedRef.current.clear();
    },
    [capturingTarget, config.momentumModifier, currentProfile, addToast, setStopAllShortcut, setAutoMomentumShortcut, setKeyDetectionShortcut]
  );

  const clearShortcut = useCallback(
    (target: ShortcutTarget) => {
      switch (target) {
        case "autoMomentum":
          setAutoMomentumShortcut([]);
          break;
        case "keyDetection":
          setKeyDetectionShortcut([]);
          break;
      }
    },
    [setAutoMomentumShortcut, setKeyDetectionShortcut]
  );

  const dialogRef = useDialog(() => {
    if (capturingTarget) {
      setCapturingTarget(null);
      setCapturedKeys([]);
      pressedRef.current.clear();
    } else onClose();
  });

  useEffect(() => {
    if (!capturingTarget) return;

    pressedRef.current.clear();

    const handleDown = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") return;
      const code = getKeyCode(e);
      pressedRef.current.add(code);
      recordKeyLayout(code, e.key, e);
      setCapturedKeys(Array.from(pressedRef.current));
    };

    const handleUp = (e: KeyboardEvent) => {
      e.preventDefault();
      if (pressedRef.current.size >= 2) {
        saveShortcut(Array.from(pressedRef.current));
      }
      const code = getKeyCode(e);
      pressedRef.current.delete(code);
    };

    window.addEventListener("keydown", handleDown);
    window.addEventListener("keyup", handleUp);
    return () => {
      window.removeEventListener("keydown", handleDown);
      window.removeEventListener("keyup", handleUp);
    };
  }, [capturingTarget, saveShortcut]);

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
      <div ref={dialogRef} role="dialog" aria-modal="true" aria-label="Settings" tabIndex={-1} className="bg-bg-secondary border border-border-color rounded-lg w-[480px] max-w-[95vw] max-h-[85vh] flex flex-col">
        {/* Header - fixed */}
        <div className="flex items-center justify-between p-4 border-b border-border-color shrink-0">
          <h2 className="text-text-primary font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            x
          </button>
        </div>

        {/* Scrollable content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-5">
          {/* ===== SHORTCUTS SECTION ===== */}
          <section>
            <SectionHeader>Shortcuts</SectionHeader>

            {/* Stop All Shortcut */}
            <div className="space-y-1 mb-3">
              <label className="text-text-secondary text-sm font-medium">
                Stop All
              </label>
              <div className="flex items-center gap-2">
                <span className="text-text-primary text-sm font-mono bg-bg-tertiary px-2 py-1 rounded min-w-[80px]">
                  {capturingTarget === "stopAll"
                    ? capturedKeys.length > 0
                      ? formatShortcut(capturedKeys)
                      : "Press keys..."
                    : formatShortcut(config.stopAllShortcut)}
                </span>
                <button
                  onClick={() => {
                    setCapturingTarget(capturingTarget === "stopAll" ? null : "stopAll");
                    setCapturedKeys([]);
                  }}
                  className={`text-xs px-2 py-1 rounded ${
                    capturingTarget === "stopAll"
                      ? "bg-accent-warning/20 text-accent-warning"
                      : "bg-bg-hover text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {capturingTarget === "stopAll" ? "Cancel" : "Change"}
                </button>
                {getShortcutConflict("Stop All") && (
                  <WarningTooltip
                    message={`Uses ${config.momentumModifier}+${keyCodeToDisplay(getShortcutConflict("Stop All")!.boundKey)} which is bound. Momentum won't work on this key.`}
                  />
                )}
              </div>
            </div>

            {/* Auto-Momentum Shortcut */}
            <div className="space-y-1 mb-3">
              <label className="text-text-secondary text-sm font-medium">
                Toggle Auto-Momentum
              </label>
              <div className="flex items-center gap-2">
                <span className="text-text-primary text-sm font-mono bg-bg-tertiary px-2 py-1 rounded min-w-[80px]">
                  {capturingTarget === "autoMomentum"
                    ? capturedKeys.length > 0
                      ? formatShortcut(capturedKeys)
                      : "Press keys..."
                    : config.autoMomentumShortcut.length > 0
                      ? formatShortcut(config.autoMomentumShortcut)
                      : "None"}
                </span>
                <button
                  onClick={() => {
                    setCapturingTarget(capturingTarget === "autoMomentum" ? null : "autoMomentum");
                    setCapturedKeys([]);
                  }}
                  className={`text-xs px-2 py-1 rounded ${
                    capturingTarget === "autoMomentum"
                      ? "bg-accent-warning/20 text-accent-warning"
                      : "bg-bg-hover text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {capturingTarget === "autoMomentum" ? "Cancel" : "Change"}
                </button>
                {config.autoMomentumShortcut.length > 0 && (
                  <button
                    onClick={() => clearShortcut("autoMomentum")}
                    className="text-xs text-text-muted hover:text-accent-error"
                  >
                    Clear
                  </button>
                )}
                {getShortcutConflict("Auto-Momentum") && (
                  <WarningTooltip
                    message={`Uses ${config.momentumModifier}+${keyCodeToDisplay(getShortcutConflict("Auto-Momentum")!.boundKey)} which is bound. Momentum won't work on this key.`}
                  />
                )}
              </div>
            </div>

            {/* Key Detection Shortcut */}
            <div className="space-y-1">
              <label className="text-text-secondary text-sm font-medium">
                Toggle Key Detection
              </label>
              <div className="flex items-center gap-2">
                <span className="text-text-primary text-sm font-mono bg-bg-tertiary px-2 py-1 rounded min-w-[80px]">
                  {capturingTarget === "keyDetection"
                    ? capturedKeys.length > 0
                      ? formatShortcut(capturedKeys)
                      : "Press keys..."
                    : config.keyDetectionShortcut.length > 0
                      ? formatShortcut(config.keyDetectionShortcut)
                      : "None"}
                </span>
                <button
                  onClick={() => {
                    setCapturingTarget(capturingTarget === "keyDetection" ? null : "keyDetection");
                    setCapturedKeys([]);
                  }}
                  className={`text-xs px-2 py-1 rounded ${
                    capturingTarget === "keyDetection"
                      ? "bg-accent-warning/20 text-accent-warning"
                      : "bg-bg-hover text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {capturingTarget === "keyDetection" ? "Cancel" : "Change"}
                </button>
                {config.keyDetectionShortcut.length > 0 && (
                  <button
                    onClick={() => clearShortcut("keyDetection")}
                    className="text-xs text-text-muted hover:text-accent-error"
                  >
                    Clear
                  </button>
                )}
                {getShortcutConflict("Key Detection") && (
                  <WarningTooltip
                    message={`Uses ${config.momentumModifier}+${keyCodeToDisplay(getShortcutConflict("Key Detection")!.boundKey)} which is bound. Momentum won't work on this key.`}
                  />
                )}
              </div>
            </div>

            {capturingTarget && (
              <p className="text-text-muted text-xs mt-2">
                Hold 2+ keys together, then release to save. Press Escape to cancel.
              </p>
            )}
          </section>

          {/* ===== KEY DETECTION SECTION ===== */}
          <section>
            <SectionHeader>Key Detection</SectionHeader>

            {inputRuntime.isLinux && (
              <div className="space-y-2 mb-4 p-3 rounded border border-border-color bg-bg-tertiary/40">
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <label className="text-text-secondary text-sm font-medium">
                      Background Detection
                    </label>
                    <p className="text-text-muted text-xs">
                      {!linuxInputStatus
                        ? "Checking current session..."
                        : linuxInputStatus.sessionType === "x11"
                        ? "Ready through the X11 backend."
                        : linuxInputStatus.backgroundDetectionAvailable
                          ? "Ready in the current session."
                          : "Needs system access on Wayland to work while the app is in background."}
                    </p>
                  </div>
                  <span
                    className={`text-xs px-2 py-1 rounded ${
                      !linuxInputStatus
                        ? "bg-bg-hover text-text-muted"
                        : linuxInputStatus.backgroundDetectionAvailable
                        ? "bg-accent-success/15 text-accent-success"
                        : "bg-accent-warning/15 text-accent-warning"
                    }`}
                  >
                    {!linuxInputStatus
                      ? "Checking"
                      : linuxInputStatus.backgroundDetectionAvailable
                        ? "Ready"
                        : "Action Needed"}
                  </span>
                </div>

                {linuxInputStatus?.message && (
                  <p className="text-text-muted text-xs">
                    {linuxInputStatus.message}
                  </p>
                )}

                {linuxInputStatus?.reloginRecommended && !linuxInputStatus.backgroundDetectionAvailable && (
                  <p className="text-text-muted text-xs">
                    The app will retry automatically after the fix. On some distros, one sign-out/sign-in may still be needed.
                  </p>
                )}

                <div className="flex items-center gap-2">
                  {linuxInputStatus?.canAutoFix && !linuxInputStatus.backgroundDetectionAvailable && (
                    <button
                      onClick={() => { void enableLinuxBackgroundDetection(); }}
                      disabled={linuxInputBusy}
                      className={`text-xs px-3 py-1.5 rounded ${
                        linuxInputBusy
                          ? "bg-bg-hover text-text-muted cursor-wait"
                          : "bg-accent-primary/20 text-accent-primary hover:bg-accent-primary/30"
                      }`}
                    >
                      {linuxInputBusy ? "Applying..." : "Enable Automatically"}
                    </button>
                  )}
                  <button
                    onClick={refreshLinuxInputStatus}
                    disabled={linuxInputBusy}
                    className="text-xs px-3 py-1.5 rounded bg-bg-hover text-text-secondary hover:text-text-primary"
                  >
                    Recheck
                  </button>
                </div>
              </div>
            )}

            {/* Key Cooldown */}
            <div className="space-y-1 mb-3">
              <div className="flex justify-between">
                <label className="text-text-secondary text-sm font-medium">
                  Key Cooldown
                </label>
                <span className="text-text-muted text-xs">
                  {config.keyCooldown}ms
                </span>
              </div>
              <input
                ref={cooldownWheelRef}
                type="range"
                min="0"
                max="2000"
                step="50"
                value={config.keyCooldown}
                onChange={(e) => setKeyCooldown(Number(e.target.value))}
                className={`w-full h-1 accent-accent-primary transition-all duration-200 ${
                  cooldownWheelActive ? "scale-105 shadow-[0_0_8px_rgba(99,102,241,0.5)]" : ""
                }`}
              />
              <div className="flex justify-between text-text-muted text-xs">
                <span>0ms</span>
                <span>2000ms</span>
              </div>
            </div>

            {/* Chord Window */}
            <div className="space-y-1 mb-3">
              <div className="flex justify-between">
                <label className="text-text-secondary text-sm font-medium">
                  Chord Window
                </label>
                <span className="text-text-muted text-xs">
                  {config.chordWindowMs}ms
                </span>
              </div>
              <input
                ref={chordWheelRef}
                type="range"
                min="20"
                max="100"
                step="5"
                value={config.chordWindowMs}
                onChange={(e) => setChordWindowMs(Number(e.target.value))}
                className={`w-full h-1 accent-accent-primary transition-all duration-200 ${
                  chordWheelActive ? "scale-105 shadow-[0_0_8px_rgba(99,102,241,0.5)]" : ""
                }`}
              />
              <div className="flex justify-between text-text-muted text-xs">
                <span>20ms (fast)</span>
                <span>100ms (lenient)</span>
              </div>
              <p className="text-text-muted text-xs">
                Time window for detecting multi-key combos (A+Z)
              </p>
            </div>

            {/* Momentum Modifier */}
            <div className="space-y-1">
              <div className="flex items-center gap-2">
                <label className="text-text-secondary text-sm font-medium">
                  Momentum Modifier
                </label>
                {momentumConflicts.length > 0 && (
                  <WarningTooltip
                    message={`${momentumConflicts.length} shortcut(s) use ${config.momentumModifier} + bound keys: ${momentumConflicts.map(c => c.shortcutName).join(", ")}`}
                  />
                )}
              </div>
              <select
                value={config.momentumModifier}
                onChange={(e) => {
                  const newModifier = e.target.value as MomentumModifier;

                  // Check for conflicts with existing shortcuts
                  if (newModifier !== "None" && currentProfile) {
                    const newConflicts = findMomentumConflicts(
                      newModifier as MomentumModifierType,
                      shortcuts,
                      currentProfile.keyBindings
                    );

                    if (newConflicts.length > 0) {
                      addToast(
                        `Warning: ${newConflicts.map(c => c.shortcutName).join(", ")} use ${newModifier} + bound keys. Momentum won't work on these keys.`,
                        "warning"
                      );
                    }
                  }

                  setMomentumModifier(newModifier);
                }}
                className="w-full app-select text-sm"
              >
                <option value="Shift">Shift (default)</option>
                <option value="Ctrl">Ctrl</option>
                <option value="Alt">Alt</option>
                <option value="None">Disabled</option>
              </select>
              <p className="text-text-muted text-xs">
                Hold this key while pressing a bound key to start from momentum position.
              </p>
            </div>
          </section>

          {/* ===== AUDIO SECTION ===== */}
          <section>
            <SectionHeader>Audio</SectionHeader>

            {/* Crossfade Duration */}
            <div className="space-y-1 mb-3">
              <div className="flex justify-between">
                <label className="text-text-secondary text-sm font-medium">
                  Crossfade Duration
                </label>
                <span className="text-text-muted text-xs">
                  {config.crossfadeDuration}ms
                </span>
              </div>
              <input
                ref={crossfadeWheelRef}
                type="range"
                min="100"
                max="2000"
                step="50"
                value={config.crossfadeDuration}
                onChange={(e) => setCrossfadeDuration(Number(e.target.value))}
                className={`w-full h-1 accent-accent-primary transition-all duration-200 ${
                  crossfadeWheelActive ? "scale-105 shadow-[0_0_8px_rgba(99,102,241,0.5)]" : ""
                }`}
              />
              <div className="flex justify-between text-text-muted text-xs">
                <span>100ms</span>
                <span>2000ms</span>
              </div>
            </div>

            {/* Audio Device */}
            <div className="space-y-1">
              <label className="text-text-secondary text-sm font-medium">
                Audio Output
              </label>
              <select
                value={config.audioDevice ?? ""}
                onChange={(e) => {
                  const value = e.target.value === "" ? null : e.target.value;
                  setAudioDevice(value);
                  addToast(
                    value ? `Audio output: ${value}` : "Audio output: System Default",
                    "info"
                  );
                }}
                className="w-full app-select text-sm"
              >
                <option value="">System Default (follow changes)</option>
                {audioDevices.map((device) => (
                  <option key={device} value={device}>
                    {device}
                  </option>
                ))}
              </select>
              <p className="text-text-muted text-xs">
                "System Default" follows your OS audio output changes automatically.
              </p>
            </div>
          </section>

          {/* ===== MANGA MOOD AI SECTION ===== */}
          <MoodAiSection config={config} />

          {/* ===== DATA SECTION ===== */}
          <section>
            <SectionHeader>Data</SectionHeader>
            <div className="flex items-center gap-2 flex-wrap">
              <button
                onClick={() => {
                  if (currentProfile) {
                    startExport(currentProfile.id, currentProfile.name);
                  }
                }}
                disabled={!currentProfile || isExporting}
                className="px-3 py-1.5 text-sm bg-accent-primary/20 text-accent-primary rounded hover:bg-accent-primary/30 disabled:opacity-40 disabled:cursor-not-allowed"
              >
                {isExporting ? "Exporting..." : "Export Profile"}
              </button>
              <button
                onClick={async () => {
                  setImportStatus("Choosing file...");
                  try {
                    const path = await commands.pickKtmFile();
                    if (!path) {
                      setImportStatus(null);
                      return;
                    }
                    setImportStatus("Importing...");
                    const newId = await commands.importProfile(path);
                    await loadProfiles();
                    await loadProfile(newId);
                    setImportStatus("Imported successfully!");
                    setTimeout(() => setImportStatus(null), 3000);
                  } catch (e) {
                    setImportStatus(`Error: ${e}`);
                    setTimeout(() => setImportStatus(null), 5000);
                  }
                }}
                className="px-3 py-1.5 text-sm bg-accent-primary/20 text-accent-primary rounded hover:bg-accent-primary/30"
              >
                Import Profile
              </button>
              <button
                onClick={async () => {
                  setImportStatus("Choosing legacy file...");
                  try {
                    const path = await commands.pickLegacyFile();
                    if (!path) {
                      setImportStatus(null);
                      return;
                    }
                    setImportStatus("Converting legacy save...");
                    const profile = await commands.importLegacySave(path);
                    await loadProfiles();
                    await loadProfile(profile.id);
                    setImportStatus(`Imported "${profile.name}" successfully!`);
                    setTimeout(() => setImportStatus(null), 3000);
                  } catch (e) {
                    setImportStatus(`Error: ${e}`);
                    setTimeout(() => setImportStatus(null), 5000);
                  }
                }}
                className="px-3 py-1.5 text-sm bg-yellow-500/20 text-yellow-400 rounded hover:bg-yellow-500/30"
              >
                Import Legacy Save
              </button>
            </div>
            {importStatus && (
              <p className={`text-xs mt-2 ${importStatus.startsWith("Error") ? "text-accent-error" : "text-accent-success"}`}>
                {importStatus}
              </p>
            )}
          </section>

          {/* ===== DISCOVERY DISLIKES SECTION ===== */}
          <section>
            <SectionHeader>Discovery Dislikes</SectionHeader>
            <DislikedVideosPanel />
          </section>

          {/* ===== ABOUT SECTION ===== */}
          <section>
            <SectionHeader>About</SectionHeader>
            <p className="text-text-muted text-xs mb-2">
              KeyToMusic v1.0.0 - Soundboard for manga reading
            </p>
            <div className="flex gap-3">
              <button
                onClick={async () => {
                  try {
                    const folder = await commands.getDataFolder();
                    await commands.openFolder(folder);
                  } catch (e) {
                    addToast("Failed to open data folder", "error");
                  }
                }}
                className="text-xs text-accent-primary hover:text-accent-primary/80 underline"
              >
                Open Data Folder
              </button>
              <button
                onClick={async () => {
                  try {
                    const folder = await commands.getLogsFolder();
                    await open(folder);
                  } catch (e) {
                    addToast("Failed to open logs folder", "error");
                  }
                }}
                className="text-xs text-accent-primary hover:text-accent-primary/80 underline"
              >
                Open Logs Folder
              </button>
            </div>
          </section>
        </div>

        {/* Footer - fixed */}
        <div className="flex justify-end p-4 border-t border-border-color shrink-0">
          <button
            onClick={onClose}
            className="px-4 py-2 bg-bg-hover text-text-primary text-sm rounded hover:bg-bg-tertiary"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
