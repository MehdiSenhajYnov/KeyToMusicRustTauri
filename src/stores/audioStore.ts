import { create } from "zustand";
import type { TrackId, SoundId } from "../types";

interface PlayingTrack {
  trackId: TrackId;
  soundId: SoundId;
}

// Mutable position record — NOT part of Zustand reactive state.
// Mutated directly by updateProgress() to avoid re-renders on every 250ms tick.
const _positions: Record<string, number> = {};

interface AudioState {
  playingTracks: Map<TrackId, PlayingTrack>;
  lastKeyPressed: string | null;

  setSoundStarted: (trackId: TrackId, soundId: SoundId) => void;
  setSoundEnded: (trackId: TrackId) => void;
  updateProgress: (trackId: TrackId, position: number) => void;
  getPosition: (trackId: TrackId) => number;
  setLastKeyPressed: (keyCode: string | null) => void;
  clearAll: () => void;
}

export const useAudioStore = create<AudioState>((set) => ({
  playingTracks: new Map(),
  lastKeyPressed: null,

  setSoundStarted: (trackId, soundId) => {
    _positions[trackId] = 0;
    set((state) => {
      const newMap = new Map(state.playingTracks);
      newMap.set(trackId, { trackId, soundId });
      return { playingTracks: newMap };
    });
  },

  setSoundEnded: (trackId) => {
    delete _positions[trackId];
    set((state) => {
      const newMap = new Map(state.playingTracks);
      newMap.delete(trackId);
      return { playingTracks: newMap };
    });
  },

  // Mutate _positions directly — no set() call, no re-renders
  updateProgress: (trackId, position) => {
    _positions[trackId] = position;
  },

  getPosition: (trackId) => {
    return _positions[trackId] ?? 0;
  },

  setLastKeyPressed: (keyCode) => {
    set({ lastKeyPressed: keyCode });
    if (keyCode !== null) {
      setTimeout(() => {
        set((s) =>
          s.lastKeyPressed === keyCode ? { lastKeyPressed: null } : s
        );
      }, 100);
    }
  },

  clearAll: () => {
    // Clear all positions
    for (const key of Object.keys(_positions)) {
      delete _positions[key];
    }
    set({ playingTracks: new Map(), lastKeyPressed: null });
  },
}));
