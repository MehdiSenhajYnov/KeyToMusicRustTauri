import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { open } from "@tauri-apps/plugin-shell";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProfileStore } from "../../stores/profileStore";
import { useExportStore } from "../../stores/exportStore";
import {
  formatShortcut,
  recordKeyLayout,
  getKeyCode,
  findMomentumConflicts,
  keyCodeToDisplay,
  type MomentumModifierType,
} from "../../utils/keyMapping";
import * as commands from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { WarningTooltip } from "../common/WarningTooltip";
import type { MomentumModifier } from "../../types";

interface SettingsModalProps {
  onClose: () => void;
}

type ShortcutTarget = "masterStop" | "autoMomentum" | "keyDetection" | null;

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-text-secondary text-xs font-semibold uppercase tracking-wide border-b border-border-color pb-1 mb-3">
      {children}
    </h3>
  );
}

export function SettingsModal({ onClose }: SettingsModalProps) {
  const {
    config,
    setCrossfadeDuration,
    setKeyCooldown,
    setChordWindowMs,
    setMasterStopShortcut,
    setAutoMomentumShortcut,
    setKeyDetectionShortcut,
    setAudioDevice,
    setMomentumModifier,
  } = useSettingsStore();
  const { currentProfile, loadProfiles, loadProfile } = useProfileStore();
  const { isExporting, startExport } = useExportStore();
  const addToast = useToastStore((s) => s.addToast);
  const [capturingTarget, setCapturingTarget] = useState<ShortcutTarget>(null);
  const [capturedKeys, setCapturedKeys] = useState<string[]>([]);
  const pressedRef = useRef(new Set<string>());
  const [importStatus, setImportStatus] = useState<string | null>(null);
  const [audioDevices, setAudioDevices] = useState<string[]>([]);

  useEffect(() => {
    commands.listAudioDevices().then(setAudioDevices).catch(() => {});
  }, []);

  // Build shortcuts array for conflict detection
  const shortcuts = useMemo(() => [
    { name: "Master Stop", keys: config.masterStopShortcut },
    { name: "Auto-Momentum", keys: config.autoMomentumShortcut },
    { name: "Key Detection", keys: config.keyDetectionShortcut },
  ], [config.masterStopShortcut, config.autoMomentumShortcut, config.keyDetectionShortcut]);

  // Detect conflicts between momentum modifier and shortcuts
  const momentumConflicts = useMemo(() => {
    if (!currentProfile) return [];
    return findMomentumConflicts(
      config.momentumModifier as MomentumModifierType,
      shortcuts,
      currentProfile.keyBindings
    );
  }, [config.momentumModifier, shortcuts, currentProfile]);

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
        case "masterStop":
          setMasterStopShortcut(keys);
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
    [capturingTarget, config.momentumModifier, currentProfile, addToast, setMasterStopShortcut, setAutoMomentumShortcut, setKeyDetectionShortcut]
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

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape" && !capturingTarget) onClose();
      if (e.key === "Escape" && capturingTarget) {
        setCapturingTarget(null);
        setCapturedKeys([]);
        pressedRef.current.clear();
      }
    };
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [onClose, capturingTarget]);

  useEffect(() => {
    if (!capturingTarget) return;

    pressedRef.current.clear();

    const handleDown = (e: KeyboardEvent) => {
      e.preventDefault();
      if (e.key === "Escape") return;
      const code = getKeyCode(e);
      pressedRef.current.add(code);
      recordKeyLayout(code, e.key);
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
      <div className="bg-bg-secondary border border-border-color rounded-lg w-[480px] max-h-[85vh] flex flex-col">
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

            {/* Master Stop Shortcut */}
            <div className="space-y-1 mb-3">
              <label className="text-text-secondary text-sm font-medium">
                Master Stop
              </label>
              <div className="flex items-center gap-2">
                <span className="text-text-primary text-sm font-mono bg-bg-tertiary px-2 py-1 rounded min-w-[80px]">
                  {capturingTarget === "masterStop"
                    ? capturedKeys.length > 0
                      ? formatShortcut(capturedKeys)
                      : "Press keys..."
                    : formatShortcut(config.masterStopShortcut)}
                </span>
                <button
                  onClick={() => {
                    setCapturingTarget(capturingTarget === "masterStop" ? null : "masterStop");
                    setCapturedKeys([]);
                  }}
                  className={`text-xs px-2 py-1 rounded ${
                    capturingTarget === "masterStop"
                      ? "bg-accent-warning/20 text-accent-warning"
                      : "bg-bg-hover text-text-secondary hover:text-text-primary"
                  }`}
                >
                  {capturingTarget === "masterStop" ? "Cancel" : "Change"}
                </button>
                {getShortcutConflict("Master Stop") && (
                  <WarningTooltip
                    message={`Uses ${config.momentumModifier}+${keyCodeToDisplay(getShortcutConflict("Master Stop")!.boundKey)} which is bound. Momentum won't work on this key.`}
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
                type="range"
                min="0"
                max="2000"
                step="50"
                value={config.keyCooldown}
                onChange={(e) => setKeyCooldown(Number(e.target.value))}
                className="w-full h-1 accent-accent-primary"
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
                type="range"
                min="20"
                max="100"
                step="5"
                value={config.chordWindowMs}
                onChange={(e) => setChordWindowMs(Number(e.target.value))}
                className="w-full h-1 accent-accent-primary"
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
                className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none"
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
                type="range"
                min="100"
                max="2000"
                step="50"
                value={config.crossfadeDuration}
                onChange={(e) => setCrossfadeDuration(Number(e.target.value))}
                className="w-full h-1 accent-accent-primary"
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
                className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none"
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
