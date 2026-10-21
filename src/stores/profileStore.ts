import { mergeKeyBindings, moveKeyBinding } from "../utils/keyBindings";
import { create } from "zustand";
import type { Profile, KeyBinding, Sound, Track } from "../types";
import * as commands from "../utils/tauriCommands";
import type { ProfileSummary } from "../utils/tauriCommands";
import { useErrorStore } from "./errorStore";
import { useHistoryStore, captureProfileState, applyHistoryState, batchHistory } from "./historyStore";
import { useWaveformStore } from "./waveformStore";
import { useToastStore } from "./toastStore";
import { useSettingsStore } from "./settingsStore";
import { useAudioStore } from "./audioStore";
import { createProfilePersistence } from "../utils/profilePersistence";
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
  saveStatus: "pending" | "saving" | "saved" | "error";
  prepareProfile: (profile: Profile) => Promise<void>;

  loadProfiles: () => Promise<void>;
  createProfile: (name: string) => Promise<Profile | null>;
  loadProfile: (id: string) => Promise<boolean>;
  saveCurrentProfile: () => Promise<boolean>;
  deleteProfile: (id: string) => Promise<boolean>;
  renameProfile: (id: string, newName: string) => Promise<boolean>;
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

  reassignKey: (keyCode: string, trackId: string, newKey: string, soundId?: string) => void;
  removeSoundFromBinding: (keyCode: string, trackId: string, soundId: string) => void;

  // Undo/Redo
  undo: () => boolean;
  redo: () => boolean;
}

export const useProfileStore = create<ProfileState>((set, get) => ({
  profiles: [],
  currentProfile: null,
  isLoading: true,
  saveStatus: "saved",

  loadProfiles: async () => {
    try {
      const profiles = await commands.listProfiles();
      set({ profiles });
    } catch (e) {
      useToastStore.getState().addToast(`Could not list profiles: ${e}`, "error");
    }
  },

  createProfile: async (name) => {
    try {
      const profile = await commands.createProfile(name);
      await get().loadProfiles();
      return profile;
    } catch (e) {
      useToastStore.getState().addToast(`Could not create profile: ${e}`, "error");
      return null;
    }
  },

  loadProfile: async (id) => {
    if (get().currentProfile?.id === id && !get().isLoading) return true;
    const gen = ++_loadGen;
    set({ isLoading: true });
    try {
      if (!await persistence.flush()) throw new Error("Save your changes before switching profiles.");
      if (gen !== _loadGen) return false;
      const profile = await commands.loadProfile(id);
      if (gen !== _loadGen) return false;
      await commands.stopAllSounds().catch(() => {});
      if (gen !== _loadGen) return false;
      useAudioStore.getState().clearAll();
      useErrorStore.getState().clearAll();
      useHistoryStore.getState().clear();
      set({ currentProfile: profile, isLoading: false });
      await useSettingsStore.getState().updateConfig({ currentProfileId: id });
      if (gen !== _loadGen) return false;
      void get().prepareProfile(profile);
      return true;
    } catch (error) {
      if (gen !== _loadGen) return false;
      set({ isLoading: false });
      useToastStore.getState().addToast(`Could not open profile: ${error}`, "error");
      return false;
    }
  },

  prepareProfile: async (profile) => {
    const gen = _loadGen;
    const isCurrent = () => gen === _loadGen && get().currentProfile?.id === profile.id;
    try {
      const missing = await commands.verifyProfileSounds(profile);
      if (!isCurrent()) return;
      for (const entry of missing) {
        const sound = profile.sounds.find((s) => s.id === entry.soundId);
        useErrorStore.getState().addMissing({
          soundId: entry.soundId, soundName: entry.soundName, path: entry.filePath,
          trackId: "", sourceType: entry.sourceType as "local" | "youtube",
          youtubeUrl: sound?.source.type === "youtube" ? sound.source.url : undefined,
        });
      }
    } catch (error) {
      console.error("Failed to verify profile sounds:", error);
    }
    if (!isCurrent()) return;
    const durations = await computeProfileDurations(profile);
    if (!isCurrent()) return;
    if (durations) {
      set((state) => ({ currentProfile: {
        ...state.currentProfile!,
        sounds: state.currentProfile!.sounds.map((sound) => {
          const original = profile.sounds.find((s) => s.id === sound.id);
          return durations[sound.id] != null && original && getSoundFilePath(original) === getSoundFilePath(sound)
            ? { ...sound, duration: durations[sound.id] } : sound;
        }),
      } }));
    }
    await preloadWaveforms(get().currentProfile!.sounds);
  },

  saveCurrentProfile: async () => persistence.flush(),

  deleteProfile: async (id) => {
    try {
      if (!await persistence.flush()) return false;
      await commands.deleteProfile(id);
      if (get().currentProfile?.id === id) {
        ++_loadGen;
        await commands.stopAllSounds().catch(() => {});
        useAudioStore.getState().clearAll();
        useHistoryStore.getState().clear();
        useErrorStore.getState().clearAll();
        set({ currentProfile: null, isLoading: false });
        await useSettingsStore.getState().updateConfig({ currentProfileId: null });
      }
      await get().loadProfiles();
      return true;
    } catch (error) {
      useToastStore.getState().addToast(`Could not delete profile: ${error}`, "error");
      return false;
    }
  },

  renameProfile: async (id, newName) => {
    if (!newName.trim()) return false;
    try {
      if (get().currentProfile?.id === id) {
        set({ currentProfile: { ...get().currentProfile!, name: newName.trim() } });
        if (!await persistence.flush()) return false;
      } else {
        const profile = await commands.loadProfile(id);
        await commands.saveProfile({ ...profile, name: newName.trim(), updatedAt: new Date().toISOString() });
      }
      await get().loadProfiles();
      return true;
    } catch (error) {
      useToastStore.getState().addToast(`Could not rename profile: ${error}`, "error");
      return false;
    }
  },

  duplicateProfile: async (id, newName) => {
    try {
      if (!await persistence.flush()) return null;
      const profile = await commands.duplicateProfile(id, newName);
      await get().loadProfiles();
      return profile;
    } catch (error) {
      useToastStore.getState().addToast(`Could not duplicate profile: ${error}`, "error");
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
      // Remove soundId from all bindings, then drop bindings with no sounds left
      const updatedBindings = state.currentProfile.keyBindings
        .map((kb) => ({
          ...kb,
          soundIds: kb.soundIds.filter((id) => id !== soundId),
        }))
        .filter((kb) => kb.soundIds.length > 0);
      return {
        currentProfile: {
          ...state.currentProfile,
          sounds: state.currentProfile.sounds.filter((s) => s.id !== soundId),
          keyBindings: updatedBindings,
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
    const trackableUpdates = ["loopMode", "name", "soundIds", "trackId", "keyCode", "mood", "moodIntensity"];
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
          keyBindings: mergeKeyBindings(state.currentProfile.keyBindings.map((kb) =>
            kb.keyCode === keyCode && kb.trackId === trackId ? { ...kb, ...updates } : kb
          )),
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

  reassignKey: (keyCode, trackId, newKey, soundId) => {
    const profile = get().currentProfile;
    if (!profile) return;
    const keyBindings = moveKeyBinding(profile.keyBindings, keyCode, trackId, newKey, soundId);
    if (keyBindings === profile.keyBindings) return;
    const updated = { ...profile, keyBindings };
    set({ currentProfile: updated });
    useHistoryStore.getState().pushState("Move key assignment", captureProfileState(profile), captureProfileState(updated));
  },

  removeSoundFromBinding: (keyCode, trackId, soundId) => {
    const profile = get().currentProfile;
    if (!profile) return;
    const keyBindings = profile.keyBindings.map((binding) => binding.keyCode === keyCode && binding.trackId === trackId
      ? { ...binding, currentIndex: 0, soundIds: binding.soundIds.filter((id) => id !== soundId) } : binding)
      .filter((binding) => binding.soundIds.length > 0);
    const used = keyBindings.some((binding) => binding.soundIds.includes(soundId));
    const updated = { ...profile, keyBindings, sounds: used ? profile.sounds : profile.sounds.filter((sound) => sound.id !== soundId) };
    set({ currentProfile: updated });
    useHistoryStore.getState().pushState("Remove sound from key", captureProfileState(profile), captureProfileState(updated));
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

const persistence = createProfilePersistence(commands.saveProfile, (saveStatus, error) => {
  useProfileStore.setState({ saveStatus });
  if (error) useToastStore.getState().addToast(`Changes could not be saved: ${error}. Use Retry in the header.`, "error");
});

// Capture the profile at edit time. Component timers cannot accidentally save another profile.
useProfileStore.subscribe((state, previous) => {
  const profile = state.currentProfile;
  if (profile && previous.currentProfile?.id === profile.id && profile !== previous.currentProfile) {
    persistence.enqueue({ ...profile, updatedAt: new Date().toISOString() });
  }
});

/** One user action (such as adding a batch of sounds) is one undo step. */
export function profileTransaction(action: string, work: () => void) {
  const before = useProfileStore.getState().currentProfile;
  if (!before) return;
  batchHistory(work);
  const after = useProfileStore.getState().currentProfile;
  if (after && after.id === before.id && after !== before) {
    useHistoryStore.getState().pushState(action, captureProfileState(before), captureProfileState(after));
  }
}
