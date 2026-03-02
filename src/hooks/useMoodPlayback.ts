import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../stores/profileStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import { useMoodStore } from "../stores/moodStore";
import * as commands from "../utils/tauriCommands";
import { getSoundFilePath } from "../utils/soundHelpers";
import { MOOD_DISPLAY } from "../utils/moodHelpers";
import type { MoodCategory, LoopMode, Sound } from "../types";

function selectSoundForMood(
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
    case "off":
    case "random": {
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
  }
}

interface MoodDetectedPayload {
  mood: string;
  source: string;
}

export function useMoodPlayback() {
  // Track the currently active mood to avoid re-triggering on same mood
  const activeMoodRef = useRef<MoodCategory | null>(null);

  const handleMoodDetected = useCallback(async (payload: MoodDetectedPayload) => {
    const config = useSettingsStore.getState().config;
    if (!config.moodAiEnabled) return;

    const mood = payload.mood as MoodCategory;
    useMoodStore.getState().setLastDetectedMood(mood);

    // Skip if same mood is already active — don't restart sounds
    if (activeMoodRef.current === mood) {
      return;
    }

    const currentProfile = useProfileStore.getState().currentProfile;
    if (!currentProfile) return;

    // Find all bindings tagged with this mood
    const matchingBindings = currentProfile.keyBindings.filter(
      (kb) => kb.mood === mood
    );

    if (matchingBindings.length === 0) {
      // Update active mood even with no bindings (prevents spamming toast on every page)
      activeMoodRef.current = mood;
      useToastStore.getState().addToast(
        `Mood detected: ${MOOD_DISPLAY[mood] ?? mood} (no tagged keys)`,
        "info"
      );
      return;
    }

    // Mood changed — update active mood and trigger new sounds
    const previousMood = activeMoodRef.current;
    activeMoodRef.current = mood;

    // Build sound lookup map
    const soundMap = new Map(currentProfile.sounds.map((s) => [s.id, s]));

    // Toast notification
    useToastStore.getState().addToast(
      previousMood
        ? `Mood: ${MOOD_DISPLAY[previousMood] ?? previousMood} → ${MOOD_DISPLAY[mood] ?? mood}`
        : `Mood: ${MOOD_DISPLAY[mood] ?? mood} — triggering ${matchingBindings.length} key(s)`,
      "success"
    );

    // Trigger all matching bindings in parallel (multi-track, same as key detection)
    // The audio engine handles crossfade automatically when a new sound plays on an occupied track
    const playPromises = matchingBindings.map(async (binding) => {
      const { sound, nextIndex } = selectSoundForMood(
        binding.soundIds,
        soundMap,
        binding.loopMode,
        binding.currentIndex
      );

      if (!sound) return;

      // Update currentIndex for next trigger
      useProfileStore.getState().updateKeyBinding(
        binding.keyCode,
        binding.trackId,
        { currentIndex: nextIndex }
      );

      const filePath = getSoundFilePath(sound);
      const useMomentum = config.autoMomentum || sound.momentum > 0;
      const startPosition = useMomentum ? sound.momentum : 0;

      try {
        await commands.playSound(
          binding.trackId,
          sound.id,
          filePath,
          startPosition,
          sound.volume
        );
      } catch (e) {
        console.error(`Failed to play mood-triggered sound: ${e}`);
      }
    });

    await Promise.allSettled(playPromises);
  }, []);

  useEffect(() => {
    const unlistenMood = listen<MoodDetectedPayload>("mood_detected", (event) => {
      handleMoodDetected(event.payload);
    });

    // Reset active mood when all sounds are stopped (so next detection re-triggers)
    const unlistenStopAll = listen("stop_all_triggered", () => {
      activeMoodRef.current = null;
    });

    return () => {
      unlistenMood.then((u) => u());
      unlistenStopAll.then((u) => u());
    };
  }, [handleMoodDetected]);

  // Reset active mood on profile switch
  useEffect(() => {
    let prevProfileId = useProfileStore.getState().currentProfile?.id;
    const unsub = useProfileStore.subscribe((state) => {
      const newId = state.currentProfile?.id;
      if (newId !== prevProfileId) {
        activeMoodRef.current = null;
        prevProfileId = newId;
      }
    });
    return unsub;
  }, []);
}
