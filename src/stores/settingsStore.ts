import { create } from "zustand";
import type { AppConfig, MomentumModifier } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";
import { useMoodStore } from "./moodStore";

interface SettingsState {
  config: AppConfig;
  isInitialized: boolean;
  setConfig: (config: AppConfig) => void;
  updateConfig: (updates: Partial<AppConfig>) => Promise<void>;
  setMasterVolume: (volume: number) => Promise<void>;
  toggleAutoMomentum: () => Promise<void>;
  toggleKeyDetection: () => Promise<void>;
  setStopAllShortcut: (keys: string[]) => Promise<void>;
  setAutoMomentumShortcut: (keys: string[]) => Promise<void>;
  setKeyDetectionShortcut: (keys: string[]) => Promise<void>;
  setCrossfadeDuration: (duration: number) => Promise<void>;
  setKeyCooldown: (cooldown: number) => Promise<void>;
  setAudioDevice: (device: string | null) => Promise<void>;
  setChordWindowMs: (ms: number) => Promise<void>;
  setMomentumModifier: (modifier: MomentumModifier) => Promise<void>;
  setPlaylistImportEnabled: (enabled: boolean) => Promise<void>;
  setMoodAiEnabled: (enabled: boolean) => Promise<void>;
  setMoodApiPort: (port: number) => Promise<void>;
  loadConfig: () => Promise<void>;
}

const defaultConfig: AppConfig = {
  masterVolume: 0.8,
  autoMomentum: false,
  keyDetectionEnabled: true,
  stopAllShortcut: ["ControlLeft", "ShiftLeft", "KeyS"],
  autoMomentumShortcut: [],
  keyDetectionShortcut: [],
  crossfadeDuration: 500,
  keyCooldown: 200,
  currentProfileId: null,
  audioDevice: null,
  chordWindowMs: 30,
  momentumModifier: "Shift",
  playlistImportEnabled: false,
  moodAiEnabled: false,
  moodApiPort: 8765,
};

let confirmedConfig = { ...defaultConfig };
let writeQueue = Promise.resolve();
const revisions = new Map<keyof AppConfig, number>();

function changeSettings(updates: Partial<AppConfig>, write: () => Promise<void>): Promise<void> {
  const keys = Object.keys(updates) as (keyof AppConfig)[];
  const requestRevisions = new Map(keys.map((key) => {
    const revision = (revisions.get(key) ?? 0) + 1;
    revisions.set(key, revision);
    return [key, revision];
  }));
  useSettingsStore.setState((state) => ({ config: { ...state.config, ...updates } }));
  const result = writeQueue.then(async () => {
    try {
      await write();
      confirmedConfig = { ...confirmedConfig, ...updates };
    } catch (error) {
      // Revert only fields still owned by this request; keep subsequent edits visible.
      const rollback = Object.fromEntries(keys.filter((key) => revisions.get(key) === requestRevisions.get(key))
        .map((key) => [key, confirmedConfig[key]]));
      useSettingsStore.setState((state) => ({ config: { ...state.config, ...rollback } }));
      useToastStore.getState().addToast(`Could not save settings: ${error}`, "error");
    }
  });
  writeQueue = result;
  return result;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  config: defaultConfig,
  isInitialized: false,
  setConfig: (config) => {
    confirmedConfig = { ...config };
    set({ config, isInitialized: true });
  },
  loadConfig: async () => {
    try { get().setConfig(await commands.getConfig()); }
    catch (error) { useToastStore.getState().addToast(`Could not load settings: ${error}`, "error"); }
  },
  updateConfig: (updates) => changeSettings(updates, () => commands.updateConfig(updates)),
  setMasterVolume: (volume) => changeSettings({ masterVolume: volume }, () => commands.setMasterVolume(volume)),
  toggleAutoMomentum: () => get().updateConfig({ autoMomentum: !get().config.autoMomentum }),
  toggleKeyDetection: () => {
    const enabled = !get().config.keyDetectionEnabled;
    return changeSettings({ keyDetectionEnabled: enabled }, () => commands.setKeyDetection(enabled));
  },
  setStopAllShortcut: (keys) => changeSettings({ stopAllShortcut: keys }, () => commands.setStopAllShortcut(keys)),
  setAutoMomentumShortcut: (keys) => get().updateConfig({ autoMomentumShortcut: keys }),
  setKeyDetectionShortcut: (keys) => get().updateConfig({ keyDetectionShortcut: keys }),
  setCrossfadeDuration: (duration) => get().updateConfig({ crossfadeDuration: duration }),
  setKeyCooldown: (cooldown) => changeSettings({ keyCooldown: cooldown }, () => commands.setKeyCooldown(cooldown)),
  setAudioDevice: (device) => changeSettings({ audioDevice: device }, () => commands.setAudioDevice(device)),
  setChordWindowMs: (ms) => get().updateConfig({ chordWindowMs: ms }),
  setMomentumModifier: (modifier) => get().updateConfig({ momentumModifier: modifier }),
  setPlaylistImportEnabled: (enabled) => get().updateConfig({ playlistImportEnabled: enabled }),
  setMoodAiEnabled: async (enabled) => {
    await get().updateConfig({ moodAiEnabled: enabled });
    await useMoodStore.getState().refreshServiceStatus();
  },
  setMoodApiPort: async (port) => {
    if (!Number.isInteger(port) || port < 1024 || port > 65535) {
      useToastStore.getState().addToast("API port must be between 1024 and 65535", "error");
      return;
    }
    await get().updateConfig({ moodApiPort: port });
    await useMoodStore.getState().refreshServiceStatus();
  },
}));

export function flushSettings(): Promise<void> {
  return writeQueue;
}

export const useMasterVolume = () => useSettingsStore(s => s.config.masterVolume);
export const useCrossfadeDuration = () => useSettingsStore(s => s.config.crossfadeDuration);
export const useKeyDetectionEnabled = () => useSettingsStore(s => s.config.keyDetectionEnabled);
export const useAutoMomentum = () => useSettingsStore(s => s.config.autoMomentum);
export const useKeyCooldown = () => useSettingsStore(s => s.config.keyCooldown);
export const useAudioDevice = () => useSettingsStore(s => s.config.audioDevice);
export const useChordWindowMs = () => useSettingsStore(s => s.config.chordWindowMs);
export const useMomentumModifier = () => useSettingsStore(s => s.config.momentumModifier);
