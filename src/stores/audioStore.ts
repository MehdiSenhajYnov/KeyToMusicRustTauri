import { create } from "zustand";
import type { TrackId, SoundId } from "../types";

interface PlayingTrack {
  trackId: TrackId;
  soundId: SoundId;
  position: number;
}

interface AudioState {
  playingTracks: Map<TrackId, PlayingTrack>;
  lastKeyPressed: string | null;

  setSoundStarted: (trackId: TrackId, soundId: SoundId) => void;
  setSoundEnded: (trackId: TrackId) => void;
  updateProgress: (trackId: TrackId, position: number) => void;
  setLastKeyPressed: (keyCode: string | null) => void;
  clearAll: () => void;
}

export const useAudioStore = create<AudioState>((set) => ({
  playingTracks: new Map(),
  lastKeyPressed: null,

  setSoundStarted: (trackId, soundId) => {
    set((state) => {
      const newMap = new Map(state.playingTracks);
      newMap.set(trackId, { trackId, soundId, position: 0 });
      return { playingTracks: newMap };
    });
  },

  setSoundEnded: (trackId) => {
    set((state) => {
      const newMap = new Map(state.playingTracks);
      newMap.delete(trackId);
      return { playingTracks: newMap };
    });
  },

  updateProgress: (trackId, position) => {
    set((state) => {
      const existing = state.playingTracks.get(trackId);
      if (!existing) return state;
      const newMap = new Map(state.playingTracks);
      newMap.set(trackId, { ...existing, position });
      return { playingTracks: newMap };
    });
  },

  setLastKeyPressed: (keyCode) => set({ lastKeyPressed: keyCode }),

  clearAll: () => set({ playingTracks: new Map(), lastKeyPressed: null }),
}));
