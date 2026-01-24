import { create } from "zustand";
import type { Profile, KeyBinding, Sound, Track } from "../types";
import * as commands from "../utils/tauriCommands";
import type { ProfileSummary } from "../utils/tauriCommands";
import { useErrorStore } from "./errorStore";

function getSoundFilePath(sound: Sound): string {
  if (sound.source.type === "local") return sound.source.path;
  return sound.source.cachedPath;
}

/** Compute missing durations for all sounds in a profile (background). */
async function computeProfileDurations(
  profile: Profile,
  updateSoundFn: (soundId: string, updates: Partial<Sound>) => void
) {
  const entries = profile.sounds
    .filter((sound) => sound.duration === 0)
    .map((sound) => ({
      soundId: sound.id,
      filePath: getSoundFilePath(sound),
      needsDuration: true,
    }));

  if (entries.length === 0) return;

  try {
    const durations = await commands.preloadProfileSounds(entries);
    for (const [soundId, duration] of Object.entries(durations)) {
      updateSoundFn(soundId, { duration });
    }
  } catch (e) {
    console.error("Failed to compute durations:", e);
  }
}

interface ProfileState {
  profiles: ProfileSummary[];
  currentProfile: Profile | null;

  loadProfiles: () => Promise<void>;
  createProfile: (name: string) => Promise<Profile | null>;
  loadProfile: (id: string) => Promise<void>;
  saveCurrentProfile: () => Promise<void>;
  deleteProfile: (id: string) => Promise<void>;
  renameProfile: (id: string, newName: string) => Promise<void>;

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
  updateKeyBinding: (keyCode: string, updates: Partial<KeyBinding>) => void;
  removeKeyBinding: (keyCode: string) => void;
}

export const useProfileStore = create<ProfileState>((set, get) => ({
  profiles: [],
  currentProfile: null,

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
    try {
      // Stop any playing sounds before switching
      await commands.stopAllSounds().catch(() => {});
      const profile = await commands.loadProfile(id);
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
      set({ currentProfile: profile });
      // Verify sound files exist
      try {
        const missing = await commands.verifyProfileSounds(profile);
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
      // Compute durations in background
      computeProfileDurations(profile, (soundId, updates) => {
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
      });
    } catch (e) {
      console.error("Failed to load profile:", e);
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

  addSound: (sound) => {
    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          sounds: [...state.currentProfile.sounds, sound],
        },
      };
    });
  },

  removeSound: (soundId) => {
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
  },

  updateSound: (soundId, updates) => {
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
  },

  addTrack: (track) => {
    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          tracks: [...state.currentProfile.tracks, track],
        },
      };
    });
  },

  removeTrack: (trackId) => {
    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          tracks: state.currentProfile.tracks.filter((t) => t.id !== trackId),
          keyBindings: state.currentProfile.keyBindings.filter(
            (kb) => kb.trackId !== trackId
          ),
        },
      };
    });
  },

  updateTrack: (trackId, updates) => {
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
  },

  addKeyBinding: (binding) => {
    set((state) => {
      if (!state.currentProfile) return state;
      // Replace existing binding for same key
      const existing = state.currentProfile.keyBindings.filter(
        (kb) => kb.keyCode !== binding.keyCode
      );
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: [...existing, binding],
        },
      };
    });
  },

  updateKeyBinding: (keyCode, updates) => {
    set((state) => {
      if (!state.currentProfile) return state;
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: state.currentProfile.keyBindings.map((kb) =>
            kb.keyCode === keyCode ? { ...kb, ...updates } : kb
          ),
        },
      };
    });
  },

  removeKeyBinding: (keyCode) => {
    set((state) => {
      if (!state.currentProfile) return state;
      const removedBinding = state.currentProfile.keyBindings.find(
        (kb) => kb.keyCode === keyCode
      );
      const remainingBindings = state.currentProfile.keyBindings.filter(
        (kb) => kb.keyCode !== keyCode
      );
      // Collect all sound IDs still referenced by remaining bindings
      const referencedSoundIds = new Set(
        remainingBindings.flatMap((kb) => kb.soundIds)
      );
      // Remove sounds that are no longer referenced by any binding
      const orphanedIds = removedBinding
        ? removedBinding.soundIds.filter((id) => !referencedSoundIds.has(id))
        : [];
      return {
        currentProfile: {
          ...state.currentProfile,
          keyBindings: remainingBindings,
          sounds: state.currentProfile.sounds.filter(
            (s) => !orphanedIds.includes(s.id)
          ),
        },
      };
    });
  },
}));
