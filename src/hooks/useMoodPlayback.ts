import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../stores/profileStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import { useMoodStore } from "../stores/moodStore";
import * as commands from "../utils/tauriCommands";
import { getSoundFilePath } from "../utils/soundHelpers";
import { MOOD_DISPLAY, INTENSITY_DISPLAY } from "../utils/moodHelpers";
import type { BaseMood, MoodIntensity, LoopMode, Sound } from "../types";

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
  intensity?: number;
  source: string;
}

interface MoodCommittedPayload {
  mood: string;
  intensity?: number;
  source: string;
  previous_mood?: string;
  previous_intensity?: number;
  dwell_count?: number;
  mood_changed?: boolean;
  intensity_changed?: boolean;
}

export function useMoodPlayback() {
  // Track the currently active committed mood+intensity to avoid re-triggering
  const activeMoodRef = useRef<BaseMood | null>(null);
  const activeIntensityRef = useRef<MoodIntensity | null>(null);

  const handleMoodCommitted = useCallback(async (payload: MoodCommittedPayload) => {
    const config = useSettingsStore.getState().config;
    if (!config.moodAiEnabled) return;

    const mood = payload.mood as BaseMood;
    const intensity = (payload.intensity ?? 2) as MoodIntensity;
    useMoodStore.getState().setCommittedMood(mood, intensity);

    // Skip if same mood AND same intensity already active
    if (activeMoodRef.current === mood && activeIntensityRef.current === intensity) {
      return;
    }

    const currentProfile = useProfileStore.getState().currentProfile;
    if (!currentProfile) return;

    // Find all bindings tagged with this mood, respecting intensity threshold
    const matchingBindings = currentProfile.keyBindings.filter((kb) => {
      if (kb.mood !== mood) return false;
      // If binding has a minimum intensity, check threshold
      if (kb.moodIntensity && intensity < kb.moodIntensity) return false;
      return true;
    });

    if (matchingBindings.length === 0) {
      activeMoodRef.current = mood;
      activeIntensityRef.current = intensity;
      useToastStore.getState().addToast(
        `Mood committed: ${MOOD_DISPLAY[mood] ?? mood} ${INTENSITY_DISPLAY[intensity]} (no tagged keys)`,
        "info"
      );
      return;
    }

    // Mood changed — update active mood and trigger new sounds
    const previousMood = activeMoodRef.current;
    activeMoodRef.current = mood;
    activeIntensityRef.current = intensity;

    // Build sound lookup map
    const soundMap = new Map(currentProfile.sounds.map((s) => [s.id, s]));

    // Toast notification
    useToastStore.getState().addToast(
      previousMood
        ? `Mood: ${MOOD_DISPLAY[previousMood] ?? previousMood} → ${MOOD_DISPLAY[mood] ?? mood} (${INTENSITY_DISPLAY[intensity]})`
        : `Mood: ${MOOD_DISPLAY[mood] ?? mood} (${INTENSITY_DISPLAY[intensity]}) — triggering ${matchingBindings.length} key(s)`,
      "success"
    );

    // Trigger all matching bindings in parallel
    const playPromises = matchingBindings.map(async (binding) => {
      const { sound, nextIndex } = selectSoundForMood(
        binding.soundIds,
        soundMap,
        binding.loopMode,
        binding.currentIndex
      );

      if (!sound) return;

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
    // mood_detected → UI only (raw mood display in Sidebar)
    const unlistenDetected = listen<MoodDetectedPayload>("mood_detected", (event) => {
      useMoodStore.getState().setLastDetectedMood(
        event.payload.mood as BaseMood,
        (event.payload.intensity ?? 2) as MoodIntensity
      );
    });

    // mood_committed → PLAYBACK (filtered by MoodDirector)
    const unlistenCommitted = listen<MoodCommittedPayload>("mood_committed", (event) => {
      handleMoodCommitted(event.payload);
    });

    // Reset active mood when all sounds are stopped
    const unlistenStopAll = listen("stop_all_triggered", () => {
      activeMoodRef.current = null;
      activeIntensityRef.current = null;
    });

    return () => {
      unlistenDetected.then((u) => u());
      unlistenCommitted.then((u) => u());
      unlistenStopAll.then((u) => u());
    };
  }, [handleMoodCommitted]);

  // Reset active mood on profile switch
  useEffect(() => {
    let prevProfileId = useProfileStore.getState().currentProfile?.id;
    const unsub = useProfileStore.subscribe((state) => {
      const newId = state.currentProfile?.id;
      if (newId !== prevProfileId) {
        activeMoodRef.current = null;
        activeIntensityRef.current = null;
        useMoodStore.getState().setCommittedMood(null);
        prevProfileId = newId;
      }
    });
    return unsub;
  }, []);
}
