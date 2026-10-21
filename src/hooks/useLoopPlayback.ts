import { useCallback, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAudioStore } from "../stores/audioStore";
import { useProfileStore } from "../stores/profileStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useToastStore } from "../stores/toastStore";
import * as commands from "../utils/tauriCommands";
import { formatErrorMessage } from "../utils/errorMessages";
import { getSoundFilePath } from "../utils/soundHelpers";
import { withTrackPlayback } from "../utils/playbackRequests";
import {
  getLoopReplayStartPosition,
  selectLoopSound,
  shouldAutoReplayLoopMode,
} from "../utils/loopPlayback";

interface SoundEndedPayload {
  trackId: string;
  soundId: string;
  reason?: "finished" | "stopped";
}

export function useLoopPlayback() {
  const clearLoopContext = useAudioStore((s) => s.clearLoopContext);
  const clearLoopContexts = useAudioStore((s) => s.clearLoopContexts);
  const addToast = useToastStore((s) => s.addToast);

  const handleSoundEnded = useCallback(
    async (payload: SoundEndedPayload) => {
      if (payload.reason === "stopped") {
        clearLoopContext(payload.trackId);
        return;
      }

      if (payload.reason !== "finished") {
        return;
      }

      const context = useAudioStore.getState().loopContexts.get(payload.trackId);
      if (!context) {
        return;
      }

      const currentProfile = useProfileStore.getState().currentProfile;
      if (!currentProfile) {
        clearLoopContext(payload.trackId);
        return;
      }

      const binding = currentProfile.keyBindings.find(
        (kb) => kb.keyCode === context.keyCode && kb.trackId === context.trackId
      );

      if (!binding || !shouldAutoReplayLoopMode(binding.loopMode)) {
        clearLoopContext(payload.trackId);
        return;
      }

      const soundMap = new Map(currentProfile.sounds.map((sound) => [sound.id, sound]));
      const { sound, nextIndex } = selectLoopSound(
        binding.soundIds,
        soundMap,
        binding.loopMode,
        binding.currentIndex
      );

      if (!sound) {
        clearLoopContext(payload.trackId);
        return;
      }

      const autoMomentum = useSettingsStore.getState().config.autoMomentum;
      const startPosition = getLoopReplayStartPosition(
        sound,
        binding.loopMode,
        autoMomentum,
        context
      );

      try {
        await commands.playSound(
          binding.trackId,
          sound.id,
          getSoundFilePath(sound),
          startPosition,
          sound.volume
        );

        const latestProfile = useProfileStore.getState().currentProfile;
        const latestBinding = latestProfile?.keyBindings.find(
          (kb) => kb.keyCode === binding.keyCode && kb.trackId === binding.trackId
        );
        if (latestProfile?.id !== currentProfile.id || latestBinding !== binding) {
          return;
        }
        useProfileStore.getState().updateKeyBinding(binding.keyCode, binding.trackId, {
          currentIndex: nextIndex,
        });

        useAudioStore.getState().setLoopContext(binding.trackId, {
          ...context,
          loopMode: binding.loopMode,
          persistSingleMomentum:
            binding.loopMode === "single" && (context.persistSingleMomentum || startPosition > 0),
        });
      } catch (error) {
        clearLoopContext(payload.trackId);

        const message = String(error);
        if (!message.includes("not found") && !message.includes("Playback cancelled")) {
          addToast(formatErrorMessage(message), "error");
        }
      }
    },
    [addToast, clearLoopContext]
  );

  useEffect(() => {
    const unlistenEnded = listen<SoundEndedPayload>("sound_ended", (event) => {
      if (event.payload.reason === "finished") {
        void withTrackPlayback(event.payload.trackId, () => handleSoundEnded(event.payload));
      } else {
        void handleSoundEnded(event.payload);
      }
    });

    const unlistenStopAll = listen("stop_all_triggered", () => {
      clearLoopContexts();
    });

    return () => {
      unlistenEnded.then((unlisten) => unlisten());
      unlistenStopAll.then((unlisten) => unlisten());
    };
  }, [clearLoopContexts, handleSoundEnded]);
}
