import { useState, useRef, useEffect, useCallback } from "react";
import { useProfileStore } from "../../stores/profileStore";
import { useToastStore } from "../../stores/toastStore";
import { useConfirmStore } from "../../stores/confirmStore";
import { keyCodeToDisplay, getKeyCode, recordKeyLayout, buildComboFromPressedKeys } from "../../utils/keyMapping";
import { formatDuration } from "../../utils/fileHelpers";
import { AddSoundModal } from "./AddSoundModal";
import * as commands from "../../utils/tauriCommands";
import { getSoundFilePath } from "../../utils/soundHelpers";
import type { LoopMode, Sound } from "../../types";

interface SoundDetailsProps {
  selectedKey: string;
  onClose: () => void;
  onKeyChanged?: (newKey: string) => void;
}

export function SoundDetails({ selectedKey, onClose, onKeyChanged }: SoundDetailsProps) {
  const { currentProfile, updateKeyBinding, removeKeyBinding, addKeyBinding, removeSound, updateSound, saveCurrentProfile } =
    useProfileStore();
  const addToast = useToastStore((s) => s.addToast);
  const showConfirm = useConfirmStore((s) => s.confirm);
  const [showAddModal, setShowAddModal] = useState(false);
  const [previewingSoundId, setPreviewingSoundId] = useState<string | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seekTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // "binding" = reassign entire key, soundId string = move that sound to another key
  const [capturingKeyFor, setCapturingKeyFor] = useState<"binding" | string | null>(null);
  const [capturedDisplay, setCapturedDisplay] = useState("");
  const pressedKeysRef = useRef<Set<string>>(new Set());

  // Clear timers on unmount to prevent leaks
  useEffect(() => {
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      if (seekTimerRef.current) clearTimeout(seekTimerRef.current);
    };
  // volumeDebounceRef is created later (after binding check), cleared in that scope
  }, []);

  const handleCapturedKey = useCallback(async (newKeyCode: string) => {
    if (!currentProfile) return;
    const binding = currentProfile.keyBindings.find((kb) => kb.keyCode === selectedKey);
    if (!binding) return;

    if (newKeyCode === selectedKey) {
      setCapturingKeyFor(null);
      return;
    }

    const existingTarget = currentProfile.keyBindings.find((kb) => kb.keyCode === newKeyCode);

    if (capturingKeyFor === "binding") {
      // Reassign the entire binding to a new key
      if (existingTarget) {
        if (!await showConfirm(`Key [${keyCodeToDisplay(newKeyCode)}] already has sounds assigned. Merge into it?`)) {
          setCapturingKeyFor(null);
          return;
        }
        // Merge: add current binding's sounds into target
        const mergedSoundIds = [...existingTarget.soundIds, ...binding.soundIds.filter((id) => !existingTarget.soundIds.includes(id))];
        updateKeyBinding(newKeyCode, { soundIds: mergedSoundIds });
        removeKeyBinding(selectedKey);
      } else {
        // Move binding to new key - add first to prevent orphaning sounds
        addKeyBinding({ ...binding, keyCode: newKeyCode });
        removeKeyBinding(selectedKey);
      }
      setTimeout(() => saveCurrentProfile(), 100);
      addToast(`Key changed: [${keyCodeToDisplay(selectedKey)}] → [${keyCodeToDisplay(newKeyCode)}]`, "success");
      onKeyChanged?.(newKeyCode);
    } else {
      // Move individual sound to another key
      const soundId = capturingKeyFor!;
      const newSoundIds = binding.soundIds.filter((id) => id !== soundId);

      if (existingTarget) {
        // Add sound to existing target binding
        if (!existingTarget.soundIds.includes(soundId)) {
          updateKeyBinding(newKeyCode, { soundIds: [...existingTarget.soundIds, soundId] });
        }
      } else {
        // Create new binding for the target key
        addKeyBinding({
          keyCode: newKeyCode,
          trackId: binding.trackId,
          soundIds: [soundId],
          loopMode: "off",
          currentIndex: 0,
        });
      }

      // Remove from current binding
      if (newSoundIds.length === 0) {
        removeKeyBinding(selectedKey);
        onClose();
      } else {
        updateKeyBinding(selectedKey, { soundIds: newSoundIds });
      }

      setTimeout(() => saveCurrentProfile(), 100);
      const sound = currentProfile.sounds.find((s) => s.id === soundId);
      addToast(`"${sound?.name}" moved to [${keyCodeToDisplay(newKeyCode)}]`, "success");
    }

    setCapturingKeyFor(null);
  }, [currentProfile, selectedKey, capturingKeyFor, updateKeyBinding, removeKeyBinding, addKeyBinding, saveCurrentProfile, addToast, showConfirm, onKeyChanged, onClose]);

  useEffect(() => {
    if (!capturingKeyFor) {
      pressedKeysRef.current.clear();
      setCapturedDisplay("");
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") {
        setCapturingKeyFor(null);
        return;
      }
      const code = getKeyCode(e);
      recordKeyLayout(code, e.key);
      pressedKeysRef.current.add(code);
      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (combo) {
        setCapturedDisplay(keyCodeToDisplay(combo));
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const code = getKeyCode(e);
      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (combo) {
        handleCapturedKey(combo);
      }
      pressedKeysRef.current.delete(code);
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("keyup", handleKeyUp, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("keyup", handleKeyUp, true);
    };
  }, [capturingKeyFor, handleCapturedKey]);

  if (!currentProfile) return null;

  const binding = currentProfile.keyBindings.find(
    (kb) => kb.keyCode === selectedKey
  );
  if (!binding) return null;

  const boundSounds = binding.soundIds
    .map((id) => currentProfile.sounds.find((s) => s.id === id))
    .filter((s) => s !== undefined);

  const firstSound = boundSounds[0];
  const displayName = binding.name || firstSound?.name || "";

  const handleNameChange = (newName: string) => {
    updateKeyBinding(selectedKey, { name: newName || undefined });
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => saveCurrentProfile(), 500);
  };

  const handleDeleteKey = async () => {
    if (!await showConfirm(`Delete key binding [${keyCodeToDisplay(selectedKey)}]?`)) return;
    removeKeyBinding(selectedKey);
    setTimeout(() => saveCurrentProfile(), 100);
    addToast(`Key [${keyCodeToDisplay(selectedKey)}] removed`, "info");
    onClose();
  };

  const handleLoopModeChange = (mode: LoopMode) => {
    updateKeyBinding(selectedKey, { loopMode: mode, currentIndex: 0 });
    setTimeout(() => saveCurrentProfile(), 100);
  };

  const handleTrackChange = (trackId: string) => {
    updateKeyBinding(selectedKey, { trackId });
    setTimeout(() => saveCurrentProfile(), 100);
  };

  const handleRemoveSound = async (soundId: string, name: string) => {
    if (!await showConfirm(`Remove "${name}" from this key?`)) return;
    removeSound(soundId);
    setTimeout(() => saveCurrentProfile(), 100);
    addToast(`Sound "${name}" removed`, "info");
  };

  const volumeDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const handleVolumeChange = (soundId: string, volume: number) => {
    updateSound(soundId, { volume });
    // Debounce the backend IPC call
    if (volumeDebounceRef.current) clearTimeout(volumeDebounceRef.current);
    volumeDebounceRef.current = setTimeout(() => {
      commands.setSoundVolume(binding.trackId, soundId, volume).catch(() => {});
      saveCurrentProfile();
    }, 100);
  };

  const handleMomentumChange = (soundId: string, momentum: number) => {
    updateSound(soundId, { momentum });
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => saveCurrentProfile(), 500);
    if (previewingSoundId === soundId && binding) {
      if (seekTimerRef.current) clearTimeout(seekTimerRef.current);
      const sound = currentProfile!.sounds.find((s) => s.id === soundId);
      if (sound) {
        seekTimerRef.current = setTimeout(() => {
          commands.playSound(binding.trackId, sound.id, getSoundFilePath(sound), momentum, sound.volume).catch(() => {});
        }, 150);
      }
    }
  };

  const handlePreviewToggle = async (sound: Sound) => {
    if (previewingSoundId === sound.id) {
      // Stop preview
      try {
        await commands.stopSound(binding!.trackId);
      } catch (e) {
        console.error("Failed to stop preview:", e);
      }
      setPreviewingSoundId(null);
    } else {
      // Play preview at momentum position
      const filePath = getSoundFilePath(sound);
      try {
        await commands.playSound(
          binding!.trackId,
          sound.id,
          filePath,
          sound.momentum,
          sound.volume
        );
        setPreviewingSoundId(sound.id);
      } catch (e) {
        console.error("Failed to preview sound:", e);
        addToast("Failed to play preview", "error");
      }
    }
  };

  return (
    <div className="p-4 space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-text-primary text-sm font-semibold">
            Sounds for key [{keyCodeToDisplay(selectedKey)}]
          </h3>
          <button
            onClick={() => setCapturingKeyFor(capturingKeyFor === "binding" ? null : "binding")}
            className={`text-xs px-1.5 py-0.5 rounded ${
              capturingKeyFor === "binding"
                ? "bg-accent-warning/20 text-accent-warning"
                : "bg-bg-hover text-text-muted hover:text-text-primary"
            }`}
            title="Change the key for this binding"
          >
            {capturingKeyFor === "binding"
              ? capturedDisplay || "Press a key..."
              : "Change Key"}
          </button>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleDeleteKey}
            className="text-accent-error hover:text-accent-error/80 text-xs"
            title="Delete this key binding"
          >
            Delete Key
          </button>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-sm"
          >
            Close
          </button>
        </div>
      </div>

      <div className="flex items-center gap-2">
        <span className="text-text-muted text-xs whitespace-nowrap">Nom:</span>
        <input
          type="text"
          value={displayName}
          onChange={(e) => handleNameChange(e.target.value)}
          placeholder="Nom du groupe"
          className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary focus:border-border-focus outline-none"
        />
      </div>

      {capturingKeyFor && (
        <p className="text-accent-warning text-xs">
          {capturedDisplay ? (
            <span>
              <span className="font-mono font-medium">{capturedDisplay}</span>
              <span className="text-text-muted ml-2">Release to confirm</span>
            </span>
          ) : (
            <span className="animate-pulse">Press the target key (Escape to cancel)</span>
          )}
        </p>
      )}

      <div className="space-y-2">
        {boundSounds.map((sound) => (
          <div
            key={sound.id}
            className="bg-bg-secondary rounded p-2 space-y-1"
          >
            <div className="flex items-center justify-between">
              <span className="text-text-primary text-sm font-medium truncate">
                {sound.name}
              </span>
              <div className="flex items-center gap-2 shrink-0">
                <button
                  onClick={() => setCapturingKeyFor(capturingKeyFor === sound.id ? null : sound.id)}
                  className={`text-xs ${
                    capturingKeyFor === sound.id
                      ? "text-accent-warning"
                      : "text-text-muted hover:text-accent-primary"
                  }`}
                  title="Move this sound to another key"
                >
                  {capturingKeyFor === sound.id
                    ? capturedDisplay || "Press key..."
                    : "Move"}
                </button>
                <button
                  onClick={() => handleRemoveSound(sound.id, sound.name)}
                  className="text-text-muted hover:text-accent-error text-xs"
                >
                  Remove
                </button>
              </div>
            </div>
            <div className="flex items-center gap-4 text-xs text-text-muted">
              <span>Duration: {formatDuration(sound.duration)}</span>
              <div className="flex items-center gap-1">
                <span>Vol:</span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  value={Math.round(sound.volume * 100)}
                  onChange={(e) =>
                    handleVolumeChange(sound.id, Number(e.target.value) / 100)
                  }
                  className="w-16 h-1 accent-accent-secondary"
                />
                <span>{Math.round(sound.volume * 100)}%</span>
              </div>
            </div>
            {/* Momentum mini-player */}
            <div className="flex items-center gap-2 text-xs text-text-muted">
              <span className="text-text-secondary whitespace-nowrap">Mom:</span>
              <input
                type="number"
                min="0"
                max={sound.duration || undefined}
                step="0.5"
                value={sound.momentum}
                onChange={(e) => {
                  const val = Math.max(0, Number(e.target.value));
                  handleMomentumChange(sound.id, val);
                }}
                className="w-16 bg-bg-tertiary border border-border-color rounded px-1 py-0.5 text-text-primary text-xs"
              />
              <span>s</span>
              <input
                type="range"
                min="0"
                max={sound.duration > 0 ? sound.duration : 1}
                step="0.1"
                value={sound.momentum}
                disabled={sound.duration === 0}
                onChange={(e) => {
                  const val = Number(e.target.value);
                  handleMomentumChange(sound.id, val);
                }}
                className="flex-1 h-1 accent-accent-primary disabled:opacity-30"
              />
              <span className="text-text-muted whitespace-nowrap">
                {formatDuration(sound.duration || 0)}
              </span>
              <button
                onClick={() => handlePreviewToggle(sound)}
                className={`w-6 h-6 flex items-center justify-center rounded ${
                  previewingSoundId === sound.id
                    ? "bg-accent-error/20 text-accent-error"
                    : "bg-bg-hover text-text-secondary hover:text-text-primary"
                }`}
                title={previewingSoundId === sound.id ? "Stop" : "Play from momentum"}
              >
                {previewingSoundId === sound.id ? "\u25A0" : "\u25B6"}
              </button>
            </div>
          </div>
        ))}
      </div>

      <div className="flex items-center gap-4 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-text-muted text-xs">Track:</span>
          <select
            value={binding.trackId}
            onChange={(e) => handleTrackChange(e.target.value)}
            className="bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary"
          >
            {currentProfile.tracks.map((t) => (
              <option key={t.id} value={t.id}>{t.name}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-text-muted text-xs">Loop:</span>
          <select
            value={binding.loopMode}
            onChange={(e) => handleLoopModeChange(e.target.value as LoopMode)}
            className="bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary"
          >
            <option value="off">Off</option>
            <option value="single">Single</option>
            <option value="random">Random</option>
            <option value="sequential">Sequential</option>
          </select>
        </div>

        <button
          onClick={() => setShowAddModal(true)}
          className="text-accent-primary text-xs hover:underline"
        >
          + Add Sound to Key
        </button>
      </div>

      {showAddModal && (
        <AddSoundModal
          targetKey={selectedKey}
          onClose={() => setShowAddModal(false)}
        />
      )}
    </div>
  );
}
