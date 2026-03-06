import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAudioStore } from "../stores/audioStore";
import { useErrorStore } from "../stores/errorStore";
import { useProfileStore } from "../stores/profileStore";
import { useToastStore } from "../stores/toastStore";
import { formatErrorMessage } from "../utils/errorMessages";

interface SoundPayload {
  trackId: string;
  soundId: string;
}

interface ProgressPayload {
  trackId: string;
  position: number;
}

interface SoundNotFoundPayload {
  soundId: string;
  path: string;
  trackId: string;
}

interface AudioErrorPayload {
  message: string;
}

export function useAudioEvents() {
  const { setSoundStarted, setSoundEnded, updateProgress } = useAudioStore();
  const addMissing = useErrorStore((s) => s.addMissing);
  const addToast = useToastStore((s) => s.addToast);

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen<SoundPayload>("sound_started", (event) => {
        setSoundStarted(event.payload.trackId, event.payload.soundId);
      })
    );

    unlisteners.push(
      listen<SoundPayload>("sound_ended", (event) => {
        setSoundEnded(event.payload.trackId);
      })
    );

    unlisteners.push(
      listen<ProgressPayload>("playback_progress", (event) => {
        updateProgress(event.payload.trackId, event.payload.position);
      })
    );

    unlisteners.push(
      listen<SoundNotFoundPayload>("sound_not_found", (event) => {
        const { soundId, path, trackId } = event.payload;
        const profile = useProfileStore.getState().currentProfile;
        if (!profile) return;

        const sound = profile.sounds.find((s) => s.id === soundId);
        if (!sound) return;

        const sourceType = sound.source.type === "youtube" ? "youtube" : "local";
        const youtubeUrl = sound.source.type === "youtube" ? sound.source.url : undefined;

        addMissing({
          soundId,
          soundName: sound.name,
          path,
          trackId,
          sourceType,
          youtubeUrl,
        });
      })
    );

    unlisteners.push(
      listen<AudioErrorPayload>("audio_error", (event) => {
        const message = formatErrorMessage(event.payload.message);
        addToast(message, "error");
      })
    );

    return () => {
      unlisteners.forEach((p) => p.then((f) => f()));
    };
  }, [setSoundStarted, setSoundEnded, updateProgress, addMissing, addToast]);
}
