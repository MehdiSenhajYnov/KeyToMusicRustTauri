import { create } from "zustand";
import type { Profile, KeyBinding, Sound, Track } from "../types";
import * as commands from "../utils/tauriCommands";
import type { ProfileSummary } from "../utils/tauriCommands";
import { useErrorStore } from "./errorStore";
import { useHistoryStore, captureProfileState, applyHistoryState } from "./historyStore";
import { useWaveformStore } from "./waveformStore";
import { getSoundFilePath } from "../utils/soundHelpers";

/** Generation counter to cancel stale profile-load operations.
 *  Incremented at the start of each loadProfile(). Fire-and-forget callbacks
 *  check this before doing work — if it changed, the load is stale. */
let _loadGen = 0;

/** Batch preload waveforms for all sounds that have a known duration. */
async function preloadWaveforms(sounds: Sound[]): Promise<void> {
  const entries = sounds
    .filter((s) => s.duration > 0)
    .map((s) => ({ path: getSoundFilePath(s), numPoints: 100 }));
  if (entries.length === 0) return;

  const wfStore = useWaveformStore.getState();
  // Only request waveforms not already cached
  const needed = entries.filter((e) => !wfStore.waveforms.has(e.path));
  if (needed.length === 0) return;

  wfStore.setLoading(true);
  try {
    const result = await commands.getWaveformsBatch(needed);
    useWaveformStore.getState().setBatch(result);
  } catch {
    // Non-critical — waveforms are optional visual data
    useWaveformStore.getState().setLoading(false);
  }
}

/** Compute missing durations for all sounds in a profile (background).
 *  Returns a map of soundId -> duration, or null if nothing needed. */
async function computeProfileDurations(
  profile: Profile,
): Promise<Record<string, number> | null> {
  const entries = profile.sounds
    .filter((sound) => sound.duration === 0)
    .map((sound) => ({
      soundId: sound.id,
      filePath: getSoundFilePath(sound),
      needsDuration: true,
    }));

  if (entries.length === 0) return null;

  try {
    const durations = await commands.preloadProfileSounds(entries);
    return Object.keys(durations).length > 0 ? durations : null;
  } catch (e) {
    console.error("Failed to compute durations:", e);
    return null;
  }
}

interface ProfileState {
  profiles: ProfileSummary[];
  currentProfile: Profile | null;
  isLoading: boolean;

  loadProfiles: () => Promise<void>;
  createProfile: (name: string) => Promise<Profile | null>;
  loadProfile: (id: string) => Promise<void>;
  saveCurrentProfile: () => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  renameProfile: (id: string, newName: string) => Promise<void>;
  duplicateProfile: (id: string, newName?: string) => Promise<Profile | null>;

  // Sounds
  addSound: (sound: Sound) => void;
  removeSound: (soundId: string) => void;
  updateSound: (soundId: string, updates: Partial<Sound>) => void;

  // Tracks
  addTrack: (track: Track) => void;
  removeTrack: (trackId: string) => void;
  updateTrack: (trackId: string, updates: Partial<Track>) => void;

  // Key Bindings
  addKeyBinding: (binding: KeyBinding) => void;
  updateKeyBinding: (keyCode: string, trackId: string, updates: Partial<KeyBinding>) => void;
  removeKeyBinding: (keyCode: string, trackId?: string) => void;

  // Undo/Redo
  undo: () => boolean;
  redo: () => boolean;
}

export const useProfileStore = create<ProfileState>((set, get) => ({
  profiles: [],
  currentProfile: null,
  isLoading: true,

  loadProfiles: async () => {
    try {
      const profiles = await commands.listProfiles();
      set({ profiles });
    } catch (e) {
      console.error("Failed to load profiles:", e);
    }
  },

  createProfile: async (name) => {
    try {
      const profile = await commands.createProfile(name);
      await get().loadProfiles();
      return profile;
    } catch (e) {
      console.error("Failed to create profile:", e);
      return null;
    }
  },

  loadProfile: async (id) => {
    // Bump generation — any in-flight fire-and-forget from a previous load becomes stale
    const gen = ++_loadGen;
    set({ isLoading: true });

    try {
      // Stop any playing sounds before switching
      await commands.stopAllSounds().catch(() => {});
      // Clear undo history when switching profiles
      useHistoryStore.getState().clear();
      const profile = await commands.loadProfile(id);

      // Stale check: another loadProfile was called while we awaited
      if (gen !== _loadGen) return;

      // Clean up orphaned sounds (not referenced by any key binding)
      const referencedIds = new Set(
        profile.keyBindings.flatMap((kb) => kb.soundIds)
      );
      const cleanedSounds = profile.sounds.filter((s) => referencedIds.has(s.id));
      if (cleanedSounds.length < profile.sounds.length) {
        profile.sounds = cleanedSounds;
        // Save the cleaned profile
        commands.saveProfile(profile).catch(() => {});
      }
      set({ currentProfile: profile, isLoading: false });
      // Verify sound files exist
      try {
        const missing = await commands.verifyProfileSounds(profile);
        if (gen !== _loadGen) return;
        const { addMissing } = useErrorStore.getState();
        for (const entry of missing) {
          const sound = profile.sounds.find((s) => s.id === entry.soundId);
          addMissing({
            soundId: entry.soundId,
            soundName: entry.soundName,
            path: entry.filePath,
            trackId: "",
            sourceType: entry.sourceType as "local" | "youtube",
            youtubeUrl: sound?.source.type === "youtube" ? sound.source.url : undefined,
          });
        }
      } catch (e) {
        console.error("Failed to verify profile sounds:", e);
      }

      // Note: we do NOT clear the waveform store here — backend cache persists
      // across profile switches, and the frontend store serves as a superset cache.
      // Clearing it would force a redundant getWaveformsBatch call.

      // Compute durations in background — single batched update, then preload waveforms
      computeProfileDurations(profile).then((durations) => {
        // Stale check: profile switched since this was fired
        if (gen !== _loadGen) return;

        if (!durations) {
          // No durations to update, but still preload waveforms for sounds that already have durations
          preloadWaveforms(profile.sounds);
          return;
        }
        set((state) => {
          if (!state.currentProfile) return state;
          const updatedSounds = state.currentProfile.sounds.map((s) =>
            durations[s.id] != null ? { ...s, duration: durations[s.id] } : s
          );
          // Preload waveforms after durations are known
          preloadWaveforms(updatedSounds);
          return {
            currentProfile: {
              ...state.currentProfile,
              sounds: updatedSounds,
            },
          };
        });
      });
    } catch (e) {
      console.error("Failed to load profile:", e);
      set({ isLoading: false });
    }
  },

  saveCurrentProfile: async () => {
    const { currentProfile } = get();
    if (!currentProfile) return;
    try {
      await commands.saveProfile(currentProfile);
    } catch (e) {
      console.error("Failed to save profile:", e);
    }
  },

  deleteProfile: async (id) => {
    try {
      await commands.deleteProfile(id);
      const { currentProfile } = get();
      if (currentProfile?.id === id) {
        set({ currentProfile: null });
      }
      await get().loadProfiles();
    } catch (e) {
      console.error("Failed to delete profile:", e);
    }
  },

  renameProfile: async (id, newName) => {
    try {
      const { currentProfile } = get();
      if (currentProfile?.id === id) {
        // Rename the currently loaded profile
        const updated = { ...currentProfile, name: newName };
        set({ currentProfile: updated });
        await commands.saveProfile(updated);
      } else {
        // Load, rename, save a non-active profile
        const profile = await commands.loadProfile(id);
        profile.name = newName;
        await commands.saveProfile(profile);
      }
      await get().loadProfiles();
    } catch (e) {
      console.error("Failed to rename profile:", e);
    }
  },

  duplicateProfile: async (id, newName) => {
    try {
      const profile = await commands.duplicateProfile(id, newName);
      await get().loadProfiles();
      return profile;
    } catch (e) {
      console.error("Failed to duplicate profile:", e);
      return null;
    }
  },

  addSound: (sound) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          sounds: [...state.currentProfile.sounds, sound],
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState("Add sound", previousState, newState);
  },

  removeSound: (soundId) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const sound = currentProfile.sounds.find((s) => s.id === soundId);
    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          sounds: state.currentProfile.sounds.filter((s) => s.id !== soundId),
          keyBindings: state.currentProfile.keyBindings.map((kb) => ({
            ...kb,
            soundIds: kb.soundIds.filter((id) => id !== soundId),
          })),
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState(
      `Remove sound "${sound?.name || soundId}"`,
      previousState,
      newState
    );
  },

  updateSound: (soundId, updates) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    // Only track meaningful changes (not duration updates from preload)
    const trackableUpdates = ["name", "momentum", "volume"];
    const hasTrackableChange = Object.keys(updates).some((k) =>
      trackableUpdates.includes(k)
    );

    const previousState = hasTrackableChange
      ? captureProfileState(currentProfile)
      : null;

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          sounds: state.currentProfile.sounds.map((s) =>
            s.id === soundId ? { ...s, ...updates } : s
          ),
        },
      };
    });

    if (previousState) {
      const newState = captureProfileState(get().currentProfile!);
      useHistoryStore.getState().pushState("Update sound", previousState, newState);
    }
  },

  addTrack: (track) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          tracks: [...state.currentProfile.tracks, track],
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState(`Add track "${track.name}"`, previousState, newState);
  },

  removeTrack: (trackId) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const track = currentProfile.tracks.find((t) => t.id === trackId);
    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      const remainingBindings = state.currentProfile.keyBindings.filter(
        (kb) => kb.trackId !== trackId
      );
      // Collect all sound IDs still referenced by remaining bindings
      const referencedSoundIds = new Set(
        remainingBindings.flatMap((kb) => kb.soundIds)
      );
      // Find sounds from removed bindings that are no longer referenced
      const removedBindings = state.currentProfile.keyBindings.filter(
        (kb) => kb.trackId === trackId
      );
      const orphanedIds = removedBindings
        .flatMap((kb) => kb.soundIds)
        .filter((id) => !referencedSoundIds.has(id));
      return {
        currentProfile: {
          ...state.currentProfile,
          tracks: state.currentProfile.tracks.filter((t) => t.id !== trackId),
          keyBindings: remainingBindings,
          sounds: state.currentProfile.sounds.filter(
            (s) => !orphanedIds.includes(s.id)
          ),
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState(
      `Remove track "${track?.name || trackId}"`,
      previousState,
      newState
    );
  },

  updateTrack: (trackId, updates) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    // Only track name changes, not playback state updates
    const hasNameChange = "name" in updates;
    const previousState = hasNameChange ? captureProfileState(currentProfile) : null;

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          tracks: state.currentProfile.tracks.map((t) =>
            t.id === trackId ? { ...t, ...updates } : t
          ),
        },
      };
    });

    if (previousState) {
      const newState = captureProfileState(get().currentProfile!);
      useHistoryStore.getState().pushState("Rename track", previousState, newState);
    }
  },

  addKeyBinding: (binding) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      // Replace existing binding for same key + track (multi-track: keep other tracks)
      const existing = state.currentProfile.keyBindings.filter(
        (kb) => !(kb.keyCode === binding.keyCode && kb.trackId === binding.trackId)
      );
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: [...existing, binding],
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState("Add key binding", previousState, newState);
  },

  updateKeyBinding: (keyCode, trackId, updates) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    // Only track meaningful changes (not currentIndex updates from playback)
    const trackableUpdates = ["loopMode", "name", "soundIds", "trackId"];
    const hasTrackableChange = Object.keys(updates).some((k) =>
      trackableUpdates.includes(k)
    );

    const previousState = hasTrackableChange
      ? captureProfileState(currentProfile)
      : null;

    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: state.currentProfile.keyBindings.map((kb) =>
            kb.keyCode === keyCode && kb.trackId === trackId ? { ...kb, ...updates } : kb
          ),
        },
      };
    });

    if (previousState) {
      const newState = captureProfileState(get().currentProfile!);
      useHistoryStore.getState().pushState("Update key binding", previousState, newState);
    }
  },

  removeKeyBinding: (keyCode, trackId?) => {
    const { currentProfile } = get();
    if (!currentProfile) return;

    const previousState = captureProfileState(currentProfile);

    set((state) => {
      if (!state.currentProfile) return state;
      // If trackId provided, remove only that specific binding; otherwise remove all for this keyCode
      const removedBindings = state.currentProfile.keyBindings.filter(
        (kb) => trackId
          ? kb.keyCode === keyCode && kb.trackId === trackId
          : kb.keyCode === keyCode
      );
      const remainingBindings = state.currentProfile.keyBindings.filter(
        (kb) => trackId
          ? !(kb.keyCode === keyCode && kb.trackId === trackId)
          : kb.keyCode !== keyCode
      );
      // Collect all sound IDs still referenced by remaining bindings
      const referencedSoundIds = new Set(
        remainingBindings.flatMap((kb) => kb.soundIds)
      );
      // Remove sounds that are no longer referenced by any binding
      const orphanedIds = new Set(
        removedBindings
          .flatMap((kb) => kb.soundIds)
          .filter((id) => !referencedSoundIds.has(id))
      );
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: remainingBindings,
          sounds: state.currentProfile.sounds.filter(
            (s) => !orphanedIds.has(s.id)
          ),
        },
      };
    });

    const newState = captureProfileState(get().currentProfile!);
    useHistoryStore.getState().pushState("Remove key binding", previousState, newState);
  },

  undo: () => {
    const { currentProfile } = get();
    if (!currentProfile) return false;

    const state = useHistoryStore.getState().undo();
    if (!state) return false;

    const restoredProfile = applyHistoryState(currentProfile, state);
    set({ currentProfile: restoredProfile });
    return true;
  },

  redo: () => {
    const { currentProfile } = get();
    if (!currentProfile) return false;

    const state = useHistoryStore.getState().redo();
    if (!state) return false;

    const restoredProfile = applyHistoryState(currentProfile, state);
    set({ currentProfile: restoredProfile });
    return true;
  },
}));
