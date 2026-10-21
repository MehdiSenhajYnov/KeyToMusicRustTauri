import { isKeyDetectionSuspended } from "../utils/keyDetectionSuspension";
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
import { selectLoopSound } from "../utils/loopPlayback";
import { withTrackPlayback } from "../utils/playbackRequests";
import type { MomentumModifier } from "../types";

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

export function useKeyDetection() {
  const setLastKeyPressed = useAudioStore((s) => s.setLastKeyPressed);
  const setLoopContext = useAudioStore((s) => s.setLoopContext);
  const toggleKeyDetection = useSettingsStore((s) => s.toggleKeyDetection);
  const toggleAutoMomentum = useSettingsStore((s) => s.toggleAutoMomentum);
  const browserKeyFallback = useRuntimeStore((s) => s.inputRuntime.browserKeyFallback);
  const lastTriggerTime = useRef(0);
  const playbackPendingRef = useRef(false);
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
      if (!config.keyDetectionEnabled || isKeyDetectionSuspended() || useProfileStore.getState().isLoading) return;

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

      // Reserve the trigger before IPC: decoding may outlast the cooldown.
      // Concurrent requests can otherwise advance the sequence and start out of order.
      const now = Date.now();
      if (playbackPendingRef.current || now - lastTriggerTime.current < config.keyCooldown) {
        return;
      }
      playbackPendingRef.current = true;
      const previousTriggerTime = lastTriggerTime.current;
      lastTriggerTime.current = now;

      const soundMap = new Map(currentProfile.sounds.map((s) => [s.id, s]));

      // Prepare all playback commands, then fire them concurrently for simultaneous multi-track playback
      const playTasks = bindings.map((binding) => withTrackPlayback(binding.trackId, async () => {
        const { sound, nextIndex } = selectLoopSound(
          binding.soundIds,
          soundMap,
          binding.loopMode,
          binding.currentIndex
        );
        if (!sound) return false;

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

          // A failed sound must remain next in the sequence for a retry.
          // Do not overwrite a profile or binding edited while decoding.
          const latestProfile = useProfileStore.getState().currentProfile;
          const latestBinding = latestProfile?.keyBindings.find(
            (kb) => kb.keyCode === binding.keyCode && kb.trackId === binding.trackId
          );
          if (latestProfile?.id !== currentProfile.id || latestBinding !== binding) {
            return true;
          }
          useProfileStore.getState().updateKeyBinding(binding.keyCode, binding.trackId, {
            currentIndex: nextIndex,
          });

          setLoopContext(
            binding.trackId,
            binding.loopMode === "off"
              ? null
              : {
                  keyCode: binding.keyCode,
                  trackId: binding.trackId,
                  loopMode: binding.loopMode,
                  momentumBehavior: "config_only",
                  persistSingleMomentum: binding.loopMode === "single" && startPosition > 0,
                }
          );
          return true;
        } catch (e) {
          const errMsg = String(e);
          if (!errMsg.includes("not found") && !errMsg.includes("Playback cancelled")) {
            useToastStore.getState().addToast(formatErrorMessage(errMsg), "error");
          }
          return false;
        }
      }));

      try {
        const results = await Promise.allSettled(playTasks);
        const anyPlayed = results.some((r) => r.status === "fulfilled" && r.value === true);
        if (!anyPlayed) {
          lastTriggerTime.current = previousTriggerTime;
        }
      } finally {
        playbackPendingRef.current = false;
      }
    },
    [findBindingsForCandidates, setLastKeyPressed, setLoopContext]
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
      if (e.defaultPrevented || isKeyDetectionSuspended() || isTextInput(e.target)) {
        return;
      }

      const resolvedCode = getKeyCode(e);
      const isRepeat = pressedKeysRef.current.has(resolvedCode);
      pressedKeysRef.current.add(resolvedCode);
      recordKeyLayout(resolvedCode, e.key, e);

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
