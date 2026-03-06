import { create } from "zustand";
import type { Profile } from "../types";

/**
 * Represents a single undoable action.
 */
export interface HistoryEntry {
  timestamp: number;
  action: string; // Human-readable action description
  // Snapshots of the affected parts of the profile
  previousState: Partial<Pick<Profile, "sounds" | "keyBindings" | "tracks">>;
  newState: Partial<Pick<Profile, "sounds" | "keyBindings" | "tracks">>;
}

interface HistoryState {
  past: HistoryEntry[];
  future: HistoryEntry[];
  maxEntries: number;

  /**
   * Push a new action to history. Clears the future stack.
   */
  pushState: (action: string, previousState: HistoryEntry["previousState"], newState: HistoryEntry["newState"]) => void;

  /**
   * Undo the last action. Returns the previous state to apply, or null if nothing to undo.
   */
  undo: () => HistoryEntry["previousState"] | null;

  /**
   * Redo the last undone action. Returns the new state to apply, or null if nothing to redo.
   */
  redo: () => HistoryEntry["newState"] | null;

  /**
   * Clear all history (e.g., when switching profiles).
   */
  clear: () => void;

  /**
   * Check if undo is available.
   */
  canUndo: () => boolean;

  /**
   * Check if redo is available.
   */
  canRedo: () => boolean;

  /**
   * Get the name of the action that would be undone.
   */
  getUndoActionName: () => string | null;

  /**
   * Get the name of the action that would be redone.
   */
  getRedoActionName: () => string | null;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  past: [],
  future: [],
  maxEntries: 50,

  pushState: (action, previousState, newState) => {
    set((state) => {
      const entry: HistoryEntry = {
        timestamp: Date.now(),
        action,
        previousState,
        newState,
      };

      // Add to past, clear future (new action invalidates redo stack)
      let newPast = [...state.past, entry];

      // Limit history size
      if (newPast.length > state.maxEntries) {
        newPast = newPast.slice(-state.maxEntries);
      }

      return {
        past: newPast,
        future: [], // Clear future on new action
      };
    });
  },

  undo: () => {
    const { past, future } = get();
    if (past.length === 0) return null;

    const entry = past[past.length - 1];

    set({
      past: past.slice(0, -1),
      future: [entry, ...future],
    });

    return entry.previousState;
  },

  redo: () => {
    const { past, future } = get();
    if (future.length === 0) return null;

    const entry = future[0];

    set({
      past: [...past, entry],
      future: future.slice(1),
    });

    return entry.newState;
  },

  clear: () => {
    set({ past: [], future: [] });
  },

  canUndo: () => get().past.length > 0,

  canRedo: () => get().future.length > 0,

  getUndoActionName: () => {
    const { past } = get();
    if (past.length === 0) return null;
    return past[past.length - 1].action;
  },

  getRedoActionName: () => {
    const { future } = get();
    if (future.length === 0) return null;
    return future[0].action;
  },
}));

// ─── Action Helpers ─────────────────────────────────────────────────────────

/**
 * Create a snapshot of the current profile state for history.
 */
export function captureProfileState(profile: Profile): HistoryEntry["previousState"] {
  return {
    sounds: [...profile.sounds],
    keyBindings: [...profile.keyBindings],
    tracks: [...profile.tracks],
  };
}

/**
 * Apply a history state snapshot to a profile.
 */
export function applyHistoryState(
  profile: Profile,
  state: HistoryEntry["previousState"]
): Profile {
  return {
    ...profile,
    sounds: state.sounds ?? profile.sounds,
    keyBindings: state.keyBindings ?? profile.keyBindings,
    tracks: state.tracks ?? profile.tracks,
  };
}
