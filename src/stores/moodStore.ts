import { create } from "zustand";
import type { BaseMood, MoodIntensity } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";

interface MoodState {
  status: commands.MoodServiceStatus | null;
  presets: commands.MoodPreset[];
  models: string[];
  isTesting: boolean;
  lastDetectedMood: BaseMood | null;
  lastDetectedIntensity: MoodIntensity | null;
  committedMood: BaseMood | null;
  committedIntensity: MoodIntensity | null;
  sceneSummary: string;

  refreshServiceStatus: () => Promise<void>;
  loadPresets: () => Promise<void>;
  saveApiKey: (key: string | null) => Promise<void>;
  testConnection: () => Promise<void>;
  exportExtension: () => Promise<string | null>;
  clearCache: () => Promise<void>;
  setLastDetectedMood: (mood: BaseMood | null, intensity?: MoodIntensity | null) => void;
  setCommittedMood: (mood: BaseMood | null, intensity?: MoodIntensity | null, summary?: string) => void;
}

const toast = (message: string, kind: "success" | "error" | "info") =>
  useToastStore.getState().addToast(message, kind);

export const useMoodStore = create<MoodState>((set, get) => ({
  status: null,
  presets: [],
  models: [],
  isTesting: false,
  lastDetectedMood: null,
  lastDetectedIntensity: null,
  committedMood: null,
  committedIntensity: null,
  sceneSummary: "",

  refreshServiceStatus: async () => {
    try {
      set({ status: await commands.getMoodServiceStatus() });
    } catch (e) {
      console.error("Failed to refresh Mood AI status:", e);
    }
  },

  loadPresets: async () => {
    if (get().presets.length) return;
    try {
      set({ presets: await commands.getMoodPresets() });
    } catch (e) {
      console.error("Failed to load Mood AI presets:", e);
    }
  },

  saveApiKey: async (key) => {
    try {
      await commands.setMoodApiKey(key);
      await get().refreshServiceStatus();
      toast(key ? "API key saved" : "API key removed", "success");
    } catch (e) {
      toast(`Could not save the API key: ${e}`, "error");
    }
  },

  testConnection: async () => {
    if (get().isTesting) return;
    set({ isTesting: true });
    try {
      const models = await commands.testMoodConnection();
      set({ models });
      toast(`Connected — ${models.length} model${models.length === 1 ? "" : "s"} available`, "success");
    } catch (e) {
      toast(`Connection failed: ${e}`, "error");
    } finally {
      set({ isTesting: false });
      await get().refreshServiceStatus();
    }
  },

  exportExtension: async () => {
    try {
      const path = await commands.exportMoodExtension();
      toast(`Extension written to ${path}`, "success");
      return path;
    } catch (e) {
      toast(`Could not export the extension: ${e}`, "error");
      return null;
    }
  },

  clearCache: async () => {
    await commands.clearMoodCache();
    await get().refreshServiceStatus();
    toast("Analyzed chapters forgotten", "info");
  },

  setLastDetectedMood: (mood, intensity) =>
    set({ lastDetectedMood: mood, lastDetectedIntensity: intensity ?? null }),
  setCommittedMood: (mood, intensity, summary) =>
    set({ committedMood: mood, committedIntensity: intensity ?? null, sceneSummary: summary ?? "" }),
}));
