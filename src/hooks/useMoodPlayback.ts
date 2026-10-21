import { useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../stores/profileStore";
import { useAudioStore } from "../stores/audioStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import { useMoodStore } from "../stores/moodStore";
import * as commands from "../utils/tauriCommands";
import { getSoundFilePath } from "../utils/soundHelpers";
import { MOOD_DISPLAY, INTENSITY_DISPLAY, selectMoodBindings } from "../utils/moodHelpers";
import { selectLoopSound } from "../utils/loopPlayback";
import { withTrackPlayback } from "../utils/playbackRequests";
import type { BaseMood, MoodIntensity } from "../types";

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
  const setLoopContext = useAudioStore((s) => s.setLoopContext);
  // Track the currently active committed mood+intensity to avoid re-triggering
  const activeMoodRef = useRef<BaseMood | null>(null);
  const activeIntensityRef = useRef<MoodIntensity | null>(null);

  const handleMoodCommitted = useCallback(async (payload: MoodCommittedPayload) => {
    const config = useSettingsStore.getState().config;
    if (!config.moodAiEnabled) return;

    if (!(payload.mood in MOOD_DISPLAY) || useProfileStore.getState().isLoading) return;
    const mood = payload.mood as BaseMood;
    const intensity = Math.max(1, Math.min(3, Math.trunc(payload.intensity ?? 2))) as MoodIntensity;

    // Skip if same mood AND same intensity already active
    if (activeMoodRef.current === mood && activeIntensityRef.current === intensity) {
      return;
    }

    const currentProfile = useProfileStore.getState().currentProfile;
    if (!currentProfile) return;

    // Find all bindings tagged with this mood, respecting intensity threshold
    const matchingBindings = selectMoodBindings(currentProfile.keyBindings, mood, intensity);

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

    // Trigger all matching bindings in parallel
    const playPromises = matchingBindings.map((binding) => withTrackPlayback(binding.trackId, async () => {
      const { sound, nextIndex } = selectLoopSound(
        binding.soundIds,
        soundMap,
        binding.loopMode,
        binding.currentIndex
      );

      if (!sound) return false;

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
                momentumBehavior: "prefer_sound_momentum",
                persistSingleMomentum: binding.loopMode === "single" && startPosition > 0,
              }
        );
        return true;
      } catch {
        return false;
      }
    }));

    const results = await Promise.allSettled(playPromises);
    if (results.some((result) => result.status === "fulfilled" && result.value === true)) {
      useMoodStore.getState().setCommittedMood(mood, intensity);
    // Toast notification
    useToastStore.getState().addToast(
      previousMood
        ? `Mood: ${MOOD_DISPLAY[previousMood] ?? previousMood} → ${MOOD_DISPLAY[mood] ?? mood} (${INTENSITY_DISPLAY[intensity]})`
        : `Mood: ${MOOD_DISPLAY[mood] ?? mood} (${INTENSITY_DISPLAY[intensity]}) — triggering ${matchingBindings.length} key(s)`,
      "success"
    );

    }

    if (!results.some((result) => result.status === "fulfilled" && result.value === true)
        && activeMoodRef.current === mood && activeIntensityRef.current === intensity) {
      activeMoodRef.current = null;
      activeIntensityRef.current = null;
    }
  }, [setLoopContext]);

  useEffect(() => {
    // mood_detected → UI only (raw mood display in Sidebar)
    const unlistenDetected = listen<MoodDetectedPayload>("mood_detected", (event) => {
      if (!(event.payload.mood in MOOD_DISPLAY)) return;
      useMoodStore.getState().setLastDetectedMood(
        event.payload.mood as BaseMood,
        (event.payload.intensity ?? 2) as MoodIntensity
      );
    });

    // mood_committed → PLAYBACK (final mood emitted by the backend pipeline)
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
