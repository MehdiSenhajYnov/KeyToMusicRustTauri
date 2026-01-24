import { create } from "zustand";
import type { AppConfig } from "../types";
import * as commands from "../utils/tauriCommands";

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
    const newConfig = { ...get().config, ...updates };
    set({ config: newConfig });
    try {
      await commands.updateConfig(updates);
    } catch (e) {
      console.error("Failed to update config:", e);
    }
  },

  setMasterVolume: async (volume) => {
    set((state) => ({ config: { ...state.config, masterVolume: volume } }));
    try {
      await commands.setMasterVolume(volume);
    } catch (e) {
      console.error("Failed to set master volume:", e);
    }
  },

  toggleAutoMomentum: async () => {
    const newValue = !get().config.autoMomentum;
    set((state) => ({
      config: { ...state.config, autoMomentum: newValue },
    }));
    try {
      await commands.updateConfig({ autoMomentum: newValue });
    } catch (e) {
      console.error("Failed to toggle auto momentum:", e);
    }
  },

  toggleKeyDetection: async () => {
    const newValue = !get().config.keyDetectionEnabled;
    set((state) => ({
      config: { ...state.config, keyDetectionEnabled: newValue },
    }));
    try {
      await commands.setKeyDetection(newValue);
    } catch (e) {
      console.error("Failed to toggle key detection:", e);
    }
  },

  setMasterStopShortcut: async (keys) => {
    set((state) => ({
      config: { ...state.config, masterStopShortcut: keys },
    }));
    try {
      await commands.setMasterStopShortcut(keys);
    } catch (e) {
      console.error("Failed to set master stop shortcut:", e);
    }
  },

  setAutoMomentumShortcut: async (keys) => {
    set((state) => ({
      config: { ...state.config, autoMomentumShortcut: keys },
    }));
    try {
      await commands.updateConfig({ autoMomentumShortcut: keys });
    } catch (e) {
      console.error("Failed to set auto momentum shortcut:", e);
    }
  },

  setKeyDetectionShortcut: async (keys) => {
    set((state) => ({
      config: { ...state.config, keyDetectionShortcut: keys },
    }));
    try {
      await commands.updateConfig({ keyDetectionShortcut: keys });
    } catch (e) {
      console.error("Failed to set key detection shortcut:", e);
    }
  },

  setCrossfadeDuration: async (duration) => {
    set((state) => ({
      config: { ...state.config, crossfadeDuration: duration },
    }));
    try {
      await commands.updateConfig({ crossfadeDuration: duration });
    } catch (e) {
      console.error("Failed to set crossfade duration:", e);
    }
  },

  setKeyCooldown: async (cooldown) => {
    set((state) => ({
      config: { ...state.config, keyCooldown: cooldown },
    }));
    try {
      await commands.setKeyCooldown(cooldown);
    } catch (e) {
      console.error("Failed to set key cooldown:", e);
    }
  },

  setAudioDevice: async (device) => {
    set((state) => ({
      config: { ...state.config, audioDevice: device },
    }));
    try {
      await commands.setAudioDevice(device);
    } catch (e) {
      console.error("Failed to set audio device:", e);
    }
  },
}));
