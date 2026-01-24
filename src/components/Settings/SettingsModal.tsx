import { useState, useEffect, useRef, useCallback } from "react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useProfileStore } from "../../stores/profileStore";
import { useExportStore } from "../../stores/exportStore";
import { formatShortcut, recordKeyLayout, charToKeyCode } from "../../utils/keyMapping";
import * as commands from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";

interface SettingsModalProps {
  onClose: () => void;
}

type ShortcutTarget = "masterStop" | "autoMomentum" | "keyDetection" | null;

export function SettingsModal({ onClose }: SettingsModalProps) {
  const {
    config,
    setCrossfadeDuration,
    setKeyCooldown,
    setMasterStopShortcut,
    setAutoMomentumShortcut,
    setKeyDetectionShortcut,
    setAudioDevice,
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

  const saveShortcut = useCallback(
    (keys: string[]) => {
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
    [capturingTarget, setMasterStopShortcut, setAutoMomentumShortcut, setKeyDetectionShortcut]
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
      const code = charToKeyCode(e.key) || e.code;
      pressedRef.current.add(code);
      recordKeyLayout(code, e.key);
      setCapturedKeys(Array.from(pressedRef.current));
    };

    const handleUp = (e: KeyboardEvent) => {
      e.preventDefault();
      if (pressedRef.current.size >= 2) {
        saveShortcut(Array.from(pressedRef.current));
      }
      const code = charToKeyCode(e.key) || e.code;
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
      <div className="bg-bg-secondary border border-border-color rounded-lg w-[420px] p-5 space-y-5">
        <div className="flex items-center justify-between">
          <h2 className="text-text-primary font-semibold">Settings</h2>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary"
          >
            x
          </button>
        </div>

        {/* Master Stop Shortcut */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm font-medium">
            Master Stop Shortcut
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
          </div>
        </div>

        {/* Auto-Momentum Shortcut */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm font-medium">
            Toggle Auto-Momentum Shortcut
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
          </div>
        </div>

        {/* Key Detection Shortcut */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm font-medium">
            Toggle Key Detection Shortcut
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
          </div>
        </div>

        {capturingTarget && (
          <p className="text-text-muted text-xs">
            Hold 2+ keys together, then release to save. Press Escape to cancel.
          </p>
        )}

        {/* Crossfade Duration */}
        <div className="space-y-1">
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

        {/* Key Cooldown */}
        <div className="space-y-1">
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

        {/* Import/Export */}
        <div className="space-y-2">
          <label className="text-text-secondary text-sm font-medium">
            Import / Export
          </label>
          <div className="flex items-center gap-2">
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
          </div>
          {importStatus && (
            <p className={`text-xs ${importStatus.startsWith("Error") ? "text-accent-error" : "text-accent-success"}`}>
              {importStatus}
            </p>
          )}
        </div>

        {/* About */}
        <div className="border-t border-border-color pt-3">
          <p className="text-text-muted text-xs">
            KeyToMusic v1.0.0 - Soundboard for manga reading
          </p>
        </div>

        <div className="flex justify-end">
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
