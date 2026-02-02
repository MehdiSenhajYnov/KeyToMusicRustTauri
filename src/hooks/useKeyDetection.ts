import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAudioStore } from "../stores/audioStore";
import { useProfileStore } from "../stores/profileStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import * as commands from "../utils/tauriCommands";
import { getKeyCode, recordKeyLayout } from "../utils/keyMapping";
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
  const lastTriggerTime = useRef(0);

  // Read currentProfile and config via getState() inside the handler
  // to avoid re-creating the callback when they change.
  const handleKeyPress = useCallback(
    async (payload: KeyPressedPayload) => {
      const config = useSettingsStore.getState().config;
      if (!config.keyDetectionEnabled) return;

      setLastKeyPressed(payload.keyCode);

      const currentProfile = useProfileStore.getState().currentProfile;
      if (!currentProfile) return;

      // Try to find binding with the exact combined key code first
      let binding = currentProfile.keyBindings.find(
        (kb) => kb.keyCode === payload.keyCode
      );

      // If not found and keyCode has modifiers, try the base key
      // (this allows [Modifier]+A to trigger "KeyA" binding with momentum)
      let useModifierForMomentum = false;
      if (!binding && payload.keyCode.includes("+")) {
        const parts = payload.keyCode.split("+");
        const baseKey = parts[parts.length - 1];
        binding = currentProfile.keyBindings.find((kb) => kb.keyCode === baseKey);
        // If we found a base key binding and the configured momentum modifier was pressed, use momentum
        if (binding && hasMomentumModifier(payload.keyCode, config.momentumModifier)) {
          useModifierForMomentum = true;
        }
      }

      if (!binding) return;

      // Cooldown: only block if a sound was recently triggered
      const now = Date.now();
      if (now - lastTriggerTime.current < config.keyCooldown) {
        return;
      }

      const soundMap = new Map(currentProfile.sounds.map(s => [s.id, s]));
      const { sound, nextIndex } = selectSound(
        binding.soundIds,
        soundMap,
        binding.loopMode,
        binding.currentIndex
      );
      if (!sound) return;

      // Update currentIndex for off/sequential/random
      if (binding.loopMode !== "single") {
        useProfileStore.getState().updateKeyBinding(binding.keyCode, { currentIndex: nextIndex });
      }

      // Use momentum if autoMomentum is on, or if Shift was used with a non-Shift binding
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
        // Cooldown starts only on successful play
        lastTriggerTime.current = Date.now();
      } catch (e) {
        // File-not-found errors are handled by the sound_not_found event listener
        const errMsg = String(e);
        if (!errMsg.includes("not found")) {
          useToastStore.getState().addToast(formatErrorMessage(errMsg), "error");
        }
      }
    },
    [setLastKeyPressed]
  );

  // Listen for Tauri events from rdev (background key detection)
  // Runs once on mount — handleKeyPress is stable because it reads state via getState()
  useEffect(() => {
    const unlistenKey = listen<KeyPressedPayload>(
      "key_pressed",
      (event) => {
        handleKeyPress(event.payload);
      }
    );

    const unlistenStop = listen("master_stop_triggered", () => {
      setLastKeyPressed(null);
    });

    const unlistenToggleKd = listen("toggle_key_detection", () => {
      toggleKeyDetection();
    });

    const unlistenToggleAm = listen("toggle_auto_momentum", () => {
      toggleAutoMomentum();
    });

    return () => {
      unlistenKey.then((f) => f());
      unlistenStop.then((f) => f());
      unlistenToggleKd.then((f) => f());
      unlistenToggleAm.then((f) => f());
    };
  }, [handleKeyPress, setLastKeyPressed, toggleKeyDetection, toggleAutoMomentum]);

  // Browser keyboard events - only for shortcuts and preventDefault
  // Sound triggering is handled by the backend chord detector (rdev global hook)
  const pressedKeysRef = useRef<Set<string>>(new Set());
  const bindingPartsRef = useRef<{ parts: Set<string>[] }>({ parts: [] });

  useEffect(() => {
    const currentProfile = useProfileStore.getState().currentProfile;
    bindingPartsRef.current.parts = (currentProfile?.keyBindings ?? []).map(
      (kb) => new Set(kb.keyCode.split("+"))
    );
  }, []);

  // Also subscribe to profile changes
  useEffect(() => {
    const unsub = useProfileStore.subscribe((state) => {
      bindingPartsRef.current.parts = (state.currentProfile?.keyBindings ?? []).map(
        (kb) => new Set(kb.keyCode.split("+"))
      );
    });
    return unsub;
  }, []);

  useEffect(() => {
    const handleBrowserKeyDown = (e: KeyboardEvent) => {
      // Skip text inputs (but not sliders/checkboxes) to allow normal typing
      if (isTextInput(e.target)) {
        return;
      }

      // Track pressed keys for shortcut detection (use physical code to match rdev)
      const resolvedCode = getKeyCode(e);
      pressedKeysRef.current.add(resolvedCode);
      recordKeyLayout(resolvedCode, e.key);

      // Read config via getState() to avoid dependency on config changes
      const config = useSettingsStore.getState().config;

      // Check key detection toggle shortcut (works even when detection is off)
      if (
        config.keyDetectionShortcut.length > 0 &&
        config.keyDetectionShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        toggleKeyDetection();
        return;
      }

      // Check master stop shortcut
      if (
        config.masterStopShortcut.length > 0 &&
        config.masterStopShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        commands.stopAllSounds().catch(console.error);
        setLastKeyPressed(null);
        return;
      }

      // Check auto momentum toggle shortcut
      if (
        config.autoMomentumShortcut.length > 0 &&
        config.autoMomentumShortcut.every((k) => pressedKeysRef.current.has(k))
      ) {
        e.preventDefault();
        toggleAutoMomentum();
        return;
      }

      // Prevent default for keys that might have bindings (to avoid typing in UI)
      // The actual sound triggering is handled by the backend chord detector
      const baseKeyCode = getKeyCode(e);

      // Use pre-built Sets for efficient lookup
      const hasRelatedBinding = bindingPartsRef.current.parts.some((partsSet) =>
        partsSet.has(baseKeyCode) ||
        (partsSet.has("Ctrl") && e.ctrlKey) ||
        (partsSet.has("Shift") && e.shiftKey) ||
        (partsSet.has("Alt") && e.altKey)
      );

      if (hasRelatedBinding) {
        e.preventDefault();
      }
    };

    const handleBrowserKeyUp = (e: KeyboardEvent) => {
      const resolvedCode = getKeyCode(e);
      pressedKeysRef.current.delete(resolvedCode);
    };

    const handleWindowBlur = () => {
      pressedKeysRef.current.clear();
    };

    window.addEventListener("keydown", handleBrowserKeyDown);
    window.addEventListener("keyup", handleBrowserKeyUp);
    window.addEventListener("blur", handleWindowBlur);
    return () => {
      window.removeEventListener("keydown", handleBrowserKeyDown);
      window.removeEventListener("keyup", handleBrowserKeyUp);
      window.removeEventListener("blur", handleWindowBlur);
    };
  }, [setLastKeyPressed, toggleKeyDetection, toggleAutoMomentum]);
}
