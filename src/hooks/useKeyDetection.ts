import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAudioStore } from "../stores/audioStore";
import { useProfileStore } from "../stores/profileStore";
import { useRuntimeStore } from "../stores/runtimeStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import * as commands from "../utils/tauriCommands";
import {
  buildComboFromPressedKeys,
  getBindingCodeCandidates,
  getKeyCode,
  normalizeCombo,
  recordKeyLayout,
} from "../utils/keyMapping";
import { formatErrorMessage } from "../utils/errorMessages";
import { getSoundFilePath } from "../utils/soundHelpers";
import { isTextInput } from "../utils/inputHelpers";
import type { LoopMode, Sound, MomentumModifier } from "../types";

/** Check if the momentum modifier is present in the key combo */
function hasMomentumModifier(keyCode: string, modifier: MomentumModifier): boolean {
  if (modifier === "None") return false;
  const parts = keyCode.split("+");
  switch (modifier) {
    case "Shift":
      return parts.includes("Shift");
    case "Ctrl":
      return parts.includes("Ctrl");
    case "Alt":
      return parts.includes("Alt");
    default:
      return false;
  }
}

function isPrefix(prefix: string[], full: string[]): boolean {
  return prefix.length <= full.length && prefix.every((part, index) => full[index] === part);
}

function findBestComboMatch(currentParts: string[], bindingParts: string[][], combos: string[]): string | null {
  let bestMatch: string | null = null;
  let bestLength = -1;

  bindingParts.forEach((parts, index) => {
    if (parts.length <= currentParts.length && isPrefix(parts, currentParts) && parts.length > bestLength) {
      bestMatch = combos[index];
      bestLength = parts.length;
    }
  });

  return bestMatch;
}

function hasComboExtensions(currentParts: string[], bindingParts: string[][]): boolean {
  return bindingParts.some((parts) => parts.length > currentParts.length && isPrefix(currentParts, parts));
}

interface KeyPressedPayload {
  keyCode: string;
  withShift: boolean;
}

function selectSound(
  soundIds: string[],
  soundMap: Map<string, Sound>,
  loopMode: LoopMode,
  currentIndex: number
): { sound: Sound | null; nextIndex: number } {
  const available = soundIds
    .map((id) => soundMap.get(id))
    .filter((s): s is Sound => s !== undefined);

  if (available.length === 0) return { sound: null, nextIndex: 0 };

  switch (loopMode) {
    case "off": {
      if (available.length === 1) return { sound: available[0], nextIndex: 0 };
      let idx: number;
      do {
        idx = Math.floor(Math.random() * available.length);
      } while (idx === currentIndex && available.length > 1);
      return { sound: available[idx], nextIndex: idx };
    }
    case "single":
      return {
        sound: available[Math.min(currentIndex, available.length - 1)],
        nextIndex: currentIndex,
      };
    case "sequential": {
      const idx = currentIndex % available.length;
      return { sound: available[idx], nextIndex: idx + 1 };
    }
    case "random": {
      if (available.length === 1) return { sound: available[0], nextIndex: 0 };
      let idx: number;
      do {
        idx = Math.floor(Math.random() * available.length);
      } while (idx === currentIndex && available.length > 1);
      return { sound: available[idx], nextIndex: idx };
    }
  }
}

export function useKeyDetection() {
  const setLastKeyPressed = useAudioStore((s) => s.setLastKeyPressed);
  const toggleKeyDetection = useSettingsStore((s) => s.toggleKeyDetection);
  const toggleAutoMomentum = useSettingsStore((s) => s.toggleAutoMomentum);
  const browserKeyFallback = useRuntimeStore((s) => s.inputRuntime.browserKeyFallback);
  const lastTriggerTime = useRef(0);
  const pressedKeysRef = useRef<Set<string>>(new Set());
  const browserPendingComboRef = useRef<string | null>(null);
  const browserChordTimerRef = useRef<number | null>(null);
  const bindingMetaRef = useRef<{
    combos: string[];
    comboParts: string[][];
    parts: Set<string>[];
  }>({
    combos: [],
    comboParts: [],
    parts: [],
  });

  const clearBrowserChordTimer = useCallback(() => {
    if (browserChordTimerRef.current !== null) {
      window.clearTimeout(browserChordTimerRef.current);
      browserChordTimerRef.current = null;
    }
    browserPendingComboRef.current = null;
  }, []);

  const syncBindingMeta = useCallback(() => {
    const currentProfile = useProfileStore.getState().currentProfile;
    const combos = [...new Set((currentProfile?.keyBindings ?? []).map((kb) => normalizeCombo(kb.keyCode)))];
    bindingMetaRef.current = {
      combos,
      comboParts: combos.map((combo) => combo.split("+")),
      parts: combos.map((combo) => new Set(combo.split("+"))),
    };
    clearBrowserChordTimer();
  }, [clearBrowserChordTimer]);

  const findBindingsForCandidates = useCallback((keyCodes: string[]) => {
    const currentProfile = useProfileStore.getState().currentProfile;
    if (!currentProfile) return [];

    for (const candidate of keyCodes) {
      const matches = currentProfile.keyBindings.filter((kb) => kb.keyCode === candidate);
      if (matches.length > 0) {
        return matches;
      }
    }

    return [];
  }, []);

  // Read currentProfile and config via getState() inside the handler
  // to avoid re-creating the callback when they change.
  const handleKeyPress = useCallback(
    async (payload: KeyPressedPayload) => {
      const config = useSettingsStore.getState().config;
      if (!config.keyDetectionEnabled) return;

      setLastKeyPressed(payload.keyCode);

      const currentProfile = useProfileStore.getState().currentProfile;
      if (!currentProfile) return;

      // Try the canonical physical code first, then a legacy layout-aware fallback.
      let bindings = findBindingsForCandidates(getBindingCodeCandidates(payload.keyCode));

      // If not found and keyCode has modifiers, try the base key
      // (this allows [Modifier]+A to trigger "KeyA" bindings with momentum)
      let useModifierForMomentum = false;
      if (bindings.length === 0 && payload.keyCode.includes("+")) {
        const parts = payload.keyCode.split("+");
        const baseKey = parts[parts.length - 1];
        bindings = findBindingsForCandidates(getBindingCodeCandidates(baseKey));
        if (bindings.length > 0 && hasMomentumModifier(payload.keyCode, config.momentumModifier)) {
          useModifierForMomentum = true;
        }
      }

      if (bindings.length === 0) return;

      // Cooldown: global per keyCode — one press triggers all tracks, cooldown prevents double-press
      const now = Date.now();
      if (now - lastTriggerTime.current < config.keyCooldown) {
        return;
      }

      const soundMap = new Map(currentProfile.sounds.map((s) => [s.id, s]));

      // Prepare all playback commands, then fire them concurrently for simultaneous multi-track playback
      const playTasks = bindings.map(async (binding) => {
        const { sound, nextIndex } = selectSound(
          binding.soundIds,
          soundMap,
          binding.loopMode,
          binding.currentIndex
        );
        if (!sound) return false;

        // Update currentIndex for off/sequential/random
        if (binding.loopMode !== "single") {
          useProfileStore.getState().updateKeyBinding(binding.keyCode, binding.trackId, {
            currentIndex: nextIndex,
          });
        }

        const startPosition =
          config.autoMomentum || useModifierForMomentum ? sound.momentum : 0;

        const filePath = getSoundFilePath(sound);

        try {
          await commands.playSound(
            binding.trackId,
            sound.id,
            filePath,
            startPosition,
            sound.volume
          );
          return true;
        } catch (e) {
          const errMsg = String(e);
          if (!errMsg.includes("not found")) {
            useToastStore.getState().addToast(formatErrorMessage(errMsg), "error");
          }
          return false;
        }
      });

      const results = await Promise.allSettled(playTasks);
      const anyPlayed = results.some((r) => r.status === "fulfilled" && r.value === true);

      // Cooldown starts only if at least one sound played
      if (anyPlayed) {
        lastTriggerTime.current = Date.now();
      }
    },
    [findBindingsForCandidates, setLastKeyPressed]
  );

  const triggerBrowserCombo = useCallback(
    (combo: string) => {
      void handleKeyPress({
        keyCode: combo,
        withShift: combo.split("+").includes("Shift"),
      });
    },
    [handleKeyPress]
  );

  const queueBrowserCombo = useCallback(
    (combo: string) => {
      clearBrowserChordTimer();
      browserPendingComboRef.current = combo;

      const chordWindowMs = useSettingsStore.getState().config.chordWindowMs;
      browserChordTimerRef.current = window.setTimeout(() => {
        const pendingCombo = browserPendingComboRef.current;
        browserChordTimerRef.current = null;
        browserPendingComboRef.current = null;

        if (!pendingCombo) return;

        const pendingParts = pendingCombo.split("+");
        const bestMatch = findBestComboMatch(
          pendingParts,
          bindingMetaRef.current.comboParts,
          bindingMetaRef.current.combos
        );

        if (bestMatch) {
          triggerBrowserCombo(bestMatch);
          return;
        }

        triggerBrowserCombo(pendingCombo);
      }, chordWindowMs);
    },
    [clearBrowserChordTimer, triggerBrowserCombo]
  );

  // Listen for Tauri events from rdev (background key detection)
  // Runs once on mount — handleKeyPress is stable because it reads state via getState()
  useEffect(() => {
    const unlistenKey = listen<KeyPressedPayload>(
      "key_pressed",
      (event) => {
        void handleKeyPress(event.payload);
      }
    );

    const unlistenStop = listen("stop_all_triggered", () => {
      setLastKeyPressed(null);
    });

    const unlistenToggleKd = listen("toggle_key_detection", () => {
      void toggleKeyDetection();
    });

    const unlistenToggleAm = listen("toggle_auto_momentum", () => {
      void toggleAutoMomentum();
    });

    const unlistenBackendWarning = listen<{ message: string }>(
      "key_detection_backend_warning",
      (event) => {
        useToastStore.getState().addToast(event.payload.message, "error");
      }
    );

    return () => {
      unlistenKey.then((f) => f());
      unlistenStop.then((f) => f());
      unlistenToggleKd.then((f) => f());
      unlistenToggleAm.then((f) => f());
      unlistenBackendWarning.then((f) => f());
    };
  }, [handleKeyPress, setLastKeyPressed, toggleKeyDetection, toggleAutoMomentum]);

  useEffect(() => {
    syncBindingMeta();
  }, [syncBindingMeta]);

  useEffect(() => {
    const unsub = useProfileStore.subscribe(() => {
      syncBindingMeta();
    });
    return unsub;
  }, [syncBindingMeta]);

  useEffect(() => {
    const handleBrowserKeyDown = (e: KeyboardEvent) => {
      // Skip text inputs (but not sliders/checkboxes) to allow normal typing
      if (isTextInput(e.target)) {
        return;
      }

      const resolvedCode = getKeyCode(e);
      const isRepeat = pressedKeysRef.current.has(resolvedCode);
      pressedKeysRef.current.add(resolvedCode);
      recordKeyLayout(resolvedCode, e.key);

      // Read config via getState() to avoid dependency on config changes
      const config = useSettingsStore.getState().config;

      if (
        config.keyDetectionShortcut.length > 0 &&
        config.keyDetectionShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        if (browserKeyFallback && !isRepeat) {
          void toggleKeyDetection();
        }
        return;
      }

      if (
        config.stopAllShortcut.length > 0 &&
        config.stopAllShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        if (browserKeyFallback && !isRepeat) {
          void commands.stopAllSounds();
          setLastKeyPressed(null);
        }
        return;
      }

      if (
        config.autoMomentumShortcut.length > 0 &&
        config.autoMomentumShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        if (browserKeyFallback && !isRepeat) {
          void toggleAutoMomentum();
        }
        return;
      }

      const hasRelatedBinding = bindingMetaRef.current.parts.some((partsSet) =>
        partsSet.has(resolvedCode) ||
        (partsSet.has("Ctrl") && e.ctrlKey) ||
        (partsSet.has("Shift") && e.shiftKey) ||
        (partsSet.has("Alt") && e.altKey)
      );

      if (hasRelatedBinding) {
        e.preventDefault();
      }

      if (!browserKeyFallback || isRepeat) {
        return;
      }

      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (!combo) {
        return;
      }

      const comboParts = combo.split("+");

      if (hasComboExtensions(comboParts, bindingMetaRef.current.comboParts)) {
        queueBrowserCombo(combo);
        return;
      }

      clearBrowserChordTimer();

      const bestMatch = findBestComboMatch(
        comboParts,
        bindingMetaRef.current.comboParts,
        bindingMetaRef.current.combos
      );

      if (bestMatch) {
        triggerBrowserCombo(bestMatch);
        return;
      }

      triggerBrowserCombo(combo);
    };

    const handleBrowserKeyUp = (e: KeyboardEvent) => {
      const resolvedCode = getKeyCode(e);
      pressedKeysRef.current.delete(resolvedCode);
    };

    const handleWindowBlur = () => {
      pressedKeysRef.current.clear();
      clearBrowserChordTimer();
    };

    window.addEventListener("keydown", handleBrowserKeyDown);
    window.addEventListener("keyup", handleBrowserKeyUp);
    window.addEventListener("blur", handleWindowBlur);
    return () => {
      window.removeEventListener("keydown", handleBrowserKeyDown);
      window.removeEventListener("keyup", handleBrowserKeyUp);
      window.removeEventListener("blur", handleWindowBlur);
      clearBrowserChordTimer();
    };
  }, [
    browserKeyFallback,
    clearBrowserChordTimer,
    queueBrowserCombo,
    setLastKeyPressed,
    toggleAutoMomentum,
    toggleKeyDetection,
    triggerBrowserCombo,
  ]);
}
