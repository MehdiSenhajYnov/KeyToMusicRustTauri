import { create } from "zustand";
import type { TrackId, SoundId } from "../types";
import type { ActiveLoopContext } from "../utils/loopPlayback";

interface PlayingTrack {
  trackId: TrackId;
  soundId: SoundId;
}

// Mutable position record — NOT part of Zustand reactive state.
// Mutated directly by updateProgress() to avoid re-renders on every 250ms tick.
const _positions: Record<string, number> = {};

interface AudioState {
  playingTracks: Map<TrackId, PlayingTrack>;
  loopContexts: Map<TrackId, ActiveLoopContext>;
  lastKeyPressed: string | null;

  setSoundStarted: (trackId: TrackId, soundId: SoundId) => void;
  setSoundEnded: (trackId: TrackId, soundId?: SoundId) => void;
  updateProgress: (trackId: TrackId, position: number) => void;
  getPosition: (trackId: TrackId) => number;
  setLoopContext: (trackId: TrackId, context: ActiveLoopContext | null) => void;
  clearLoopContext: (trackId: TrackId) => void;
  clearLoopContexts: () => void;
  setLastKeyPressed: (keyCode: string | null) => void;
  clearAll: () => void;
}

export const useAudioStore = create<AudioState>((set, get) => ({
  playingTracks: new Map(),
  loopContexts: new Map(),
  lastKeyPressed: null,

  setSoundStarted: (trackId, soundId) => {
    _positions[trackId] = 0;
    set((state) => {
      const newMap = new Map(state.playingTracks);
      newMap.set(trackId, { trackId, soundId });
      return { playingTracks: newMap };
    });
  },

  setSoundEnded: (trackId, soundId) => {
    if (soundId && get().playingTracks.get(trackId)?.soundId !== soundId) return;
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

  setLoopContext: (trackId, context) => {
    set((state) => {
      const next = new Map(state.loopContexts);
      if (context) {
        next.set(trackId, context);
      } else {
        next.delete(trackId);
      }
      return { loopContexts: next };
    });
  },

  clearLoopContext: (trackId) => {
    set((state) => {
      if (!state.loopContexts.has(trackId)) return state;
      const next = new Map(state.loopContexts);
      next.delete(trackId);
      return { loopContexts: next };
    });
  },

  clearLoopContexts: () => {
    set({ loopContexts: new Map() });
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
    set({ playingTracks: new Map(), loopContexts: new Map(), lastKeyPressed: null });
  },
}));
