import { create } from "zustand";
import type { AppConfig, MomentumModifier } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";

interface SettingsState {
  config: AppConfig;
  setConfig: (config: AppConfig) => void;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setMasterVolume: (volume: number) => Promise<void>;
  toggleAutoMomentum: () => Promise<void>;
  toggleKeyDetection: () => Promise<void>;
  setMasterStopShortcut: (keys: string[]) => Promise<void>;
  setAutoMomentumShortcut: (keys: string[]) => Promise<void>;
  setKeyDetectionShortcut: (keys: string[]) => Promise<void>;
  setCrossfadeDuration: (duration: number) => Promise<void>;
  setKeyCooldown: (cooldown: number) => Promise<void>;
  setAudioDevice: (device: string | null) => Promise<void>;
  setChordWindowMs: (ms: number) => Promise<void>;
  setMomentumModifier: (modifier: MomentumModifier) => Promise<void>;
  setPlaylistImportEnabled: (enabled: boolean) => Promise<void>;
  loadConfig: () => Promise<void>;
}

const defaultConfig: AppConfig = {
  masterVolume: 0.8,
  autoMomentum: false,
  keyDetectionEnabled: true,
  masterStopShortcut: ["ControlLeft", "ShiftLeft", "KeyS"],
  autoMomentumShortcut: [],
  keyDetectionShortcut: [],
  crossfadeDuration: 500,
  keyCooldown: 200,
  currentProfileId: null,
  audioDevice: null,
  chordWindowMs: 30,
  momentumModifier: "Shift",
  playlistImportEnabled: false,
};

export const useSettingsStore = create<SettingsState>((set, get) => ({
  config: defaultConfig,

  setConfig: (config) => set({ config }),

  loadConfig: async () => {
    try {
      const config = await commands.getConfig();
      set({ config });
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  },

  updateConfig: async (updates) => {
    const prev = get().config;
    set({ config: { ...prev, ...updates } });
    try {
      await commands.updateConfig(updates);
    } catch (e) {
      set({ config: prev });
      useToastStore.getState().addToast("Failed to save settings", "error");
    }
  },

  setMasterVolume: async (volume) => {
    const prev = get().config.masterVolume;
    set((state) => ({ config: { ...state.config, masterVolume: volume } }));
    try {
      await commands.setMasterVolume(volume);
    } catch (e) {
      set((state) => ({ config: { ...state.config, masterVolume: prev } }));
      useToastStore.getState().addToast("Failed to set master volume", "error");
    }
  },

  toggleAutoMomentum: async () => {
    const prev = get().config.autoMomentum;
    const newValue = !prev;
    set((state) => ({
      config: { ...state.config, autoMomentum: newValue },
    }));
    try {
      await commands.updateConfig({ autoMomentum: newValue });
    } catch (e) {
      set((state) => ({ config: { ...state.config, autoMomentum: prev } }));
      useToastStore.getState().addToast("Failed to toggle auto momentum", "error");
    }
  },

  toggleKeyDetection: async () => {
    const prev = get().config.keyDetectionEnabled;
    const newValue = !prev;
    set((state) => ({
      config: { ...state.config, keyDetectionEnabled: newValue },
    }));
    try {
      await commands.setKeyDetection(newValue);
    } catch (e) {
      set((state) => ({ config: { ...state.config, keyDetectionEnabled: prev } }));
      useToastStore.getState().addToast("Failed to toggle key detection", "error");
    }
  },

  setMasterStopShortcut: async (keys) => {
    const prev = get().config.masterStopShortcut;
    set((state) => ({
      config: { ...state.config, masterStopShortcut: keys },
    }));
    try {
      await commands.setMasterStopShortcut(keys);
    } catch (e) {
      set((state) => ({ config: { ...state.config, masterStopShortcut: prev } }));
      useToastStore.getState().addToast("Failed to set master stop shortcut", "error");
    }
  },

  setAutoMomentumShortcut: async (keys) => {
    const prev = get().config.autoMomentumShortcut;
    set((state) => ({
      config: { ...state.config, autoMomentumShortcut: keys },
    }));
    try {
      await commands.updateConfig({ autoMomentumShortcut: keys });
    } catch (e) {
      set((state) => ({ config: { ...state.config, autoMomentumShortcut: prev } }));
      useToastStore.getState().addToast("Failed to set auto momentum shortcut", "error");
    }
  },

  setKeyDetectionShortcut: async (keys) => {
    const prev = get().config.keyDetectionShortcut;
    set((state) => ({
      config: { ...state.config, keyDetectionShortcut: keys },
    }));
    try {
      await commands.updateConfig({ keyDetectionShortcut: keys });
    } catch (e) {
      set((state) => ({ config: { ...state.config, keyDetectionShortcut: prev } }));
      useToastStore.getState().addToast("Failed to set key detection shortcut", "error");
    }
  },

  setCrossfadeDuration: async (duration) => {
    const prev = get().config.crossfadeDuration;
    set((state) => ({
      config: { ...state.config, crossfadeDuration: duration },
    }));
    try {
      await commands.updateConfig({ crossfadeDuration: duration });
    } catch (e) {
      set((state) => ({ config: { ...state.config, crossfadeDuration: prev } }));
      useToastStore.getState().addToast("Failed to set crossfade duration", "error");
    }
  },

  setKeyCooldown: async (cooldown) => {
    const prev = get().config.keyCooldown;
    set((state) => ({
      config: { ...state.config, keyCooldown: cooldown },
    }));
    try {
      await commands.setKeyCooldown(cooldown);
    } catch (e) {
      set((state) => ({ config: { ...state.config, keyCooldown: prev } }));
      useToastStore.getState().addToast("Failed to set key cooldown", "error");
    }
  },

  setAudioDevice: async (device) => {
    const prev = get().config.audioDevice;
    set((state) => ({
      config: { ...state.config, audioDevice: device },
    }));
    try {
      await commands.setAudioDevice(device);
    } catch (e) {
      set((state) => ({ config: { ...state.config, audioDevice: prev } }));
      useToastStore.getState().addToast("Failed to set audio device", "error");
    }
  },

  setChordWindowMs: async (ms) => {
    const prev = get().config.chordWindowMs;
    set((state) => ({
      config: { ...state.config, chordWindowMs: ms },
    }));
    try {
      await commands.updateConfig({ chordWindowMs: ms });
    } catch (e) {
      set((state) => ({ config: { ...state.config, chordWindowMs: prev } }));
      useToastStore.getState().addToast("Failed to set chord window", "error");
    }
  },

  setMomentumModifier: async (modifier) => {
    const prev = get().config.momentumModifier;
    set((state) => ({
      config: { ...state.config, momentumModifier: modifier },
    }));
    try {
      await commands.updateConfig({ momentumModifier: modifier });
    } catch (e) {
      set((state) => ({ config: { ...state.config, momentumModifier: prev } }));
      useToastStore.getState().addToast("Failed to set momentum modifier", "error");
    }
  },

  setPlaylistImportEnabled: async (enabled) => {
    const prev = get().config.playlistImportEnabled;
    set((state) => ({
      config: { ...state.config, playlistImportEnabled: enabled },
    }));
    try {
      await commands.updateConfig({ playlistImportEnabled: enabled });
    } catch (e) {
      set((state) => ({ config: { ...state.config, playlistImportEnabled: prev } }));
    }
  },
}));

export const useMasterVolume = () => useSettingsStore(s => s.config.masterVolume);
export const useCrossfadeDuration = () => useSettingsStore(s => s.config.crossfadeDuration);
export const useKeyDetectionEnabled = () => useSettingsStore(s => s.config.keyDetectionEnabled);
export const useAutoMomentum = () => useSettingsStore(s => s.config.autoMomentum);
export const useKeyCooldown = () => useSettingsStore(s => s.config.keyCooldown);
export const useAudioDevice = () => useSettingsStore(s => s.config.audioDevice);
export const useChordWindowMs = () => useSettingsStore(s => s.config.chordWindowMs);
export const useMomentumModifier = () => useSettingsStore(s => s.config.momentumModifier);
