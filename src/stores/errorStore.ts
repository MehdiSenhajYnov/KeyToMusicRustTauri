import { create } from "zustand";

export interface SoundNotFoundEntry {
  soundId: string;
  soundName: string;
  path: string;
  trackId: string;
  sourceType: "local" | "youtube";
  youtubeUrl?: string;
}

interface ErrorState {
  missingQueue: SoundNotFoundEntry[];
  addMissing: (entry: SoundNotFoundEntry) => void;
  dismissCurrent: () => void;
  clearAll: () => void;
}

export const useErrorStore = create<ErrorState>((set) => ({
  missingQueue: [],

  addMissing: (entry) => {
    set((state) => {
      // Avoid duplicate entries for the same sound
      if (state.missingQueue.some((e) => e.soundId === entry.soundId)) {
        return state;
      }
      return { missingQueue: [...state.missingQueue, entry] };
    });
  },

  dismissCurrent: () => {
    set((state) => ({
      missingQueue: state.missingQueue.slice(1),
    }));
  },

  clearAll: () => {
    set({ missingQueue: [] });
  },
}));
