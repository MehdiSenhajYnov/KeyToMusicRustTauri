import { create } from "zustand";
import type { BaseMood, MoodIntensity } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";
import { open } from "@tauri-apps/plugin-shell";

interface MoodState {
  status: commands.MoodServiceStatus | null;
  presets: commands.MoodPreset[];
  models: commands.MoodModel[];
  isSigningIn: boolean;
  isTesting: boolean;
  lastDetectedMood: BaseMood | null;
  lastDetectedIntensity: MoodIntensity | null;
  committedMood: BaseMood | null;
  committedIntensity: MoodIntensity | null;
  sceneSummary: string;

  refreshServiceStatus: () => Promise<void>;
  loadPresets: () => Promise<void>;
  saveApiKey: (key: string | null) => Promise<void>;
  testConnection: (quiet?: boolean) => Promise<void>;
  signIn: () => Promise<void>;
  signOut: () => Promise<void>;
  onAccount: (payload: { planEnabled: boolean; error: string | null }) => Promise<void>;
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
  isSigningIn: false,
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

  testConnection: async (quiet = false) => {
    if (get().isTesting) return;
    set({ isTesting: true });
    try {
      const models = await commands.testMoodConnection();
      set({ models });
      if (!quiet) toast(`Connected — ${models.length} model${models.length === 1 ? "" : "s"} available`, "success");
    } catch (e) {
      if (!quiet) toast(`Connection failed: ${e}`, "error");
    } finally {
      set({ isTesting: false });
      await get().refreshServiceStatus();
    }
  },

  signIn: async () => {
    set({ isSigningIn: true });
    try {
      const url = await commands.chatgptSignIn();
      await open(url);
    } catch (e) {
      set({ isSigningIn: false });
      toast(`Could not start the ChatGPT sign-in: ${e}`, "error");
    }
  },

  signOut: async () => {
    try {
      await commands.chatgptSignOut();
    } catch (e) {
      toast(`${e}`, "info");
    }
    set({ models: [] });
    await get().refreshServiceStatus();
  },

  onAccount: async ({ planEnabled, error }) => {
    set({ isSigningIn: false });
    if (error) toast(error, "error");
    else if (planEnabled)
      toast("You're using your ChatGPT plan: Mood AI uses it to analyze chapters. Manage usage in ChatGPT settings.", "success");
    await get().refreshServiceStatus();
    if (planEnabled) await get().testConnection(true);
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
