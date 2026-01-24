import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useAudioStore } from "../stores/audioStore";

interface SoundPayload {
  trackId: string;
  soundId: string;
}

interface ProgressPayload {
  trackId: string;
  position: number;
}

export function useAudioEvents() {
  const { setSoundStarted, setSoundEnded, updateProgress } = useAudioStore();

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

    return () => {
      unlisteners.forEach((p) => p.then((f) => f()));
    };
  }, [setSoundStarted, setSoundEnded, updateProgress]);
}
