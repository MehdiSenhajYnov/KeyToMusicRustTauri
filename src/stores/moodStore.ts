import { create } from "zustand";
import type { BaseMood, MoodIntensity } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";

interface MoodState {
  serverStatus: "stopped" | "starting" | "running" | "error";
  apiStatus: "disabled" | "stopped" | "running" | "error";
  apiPort: number;
  serverInstalled: boolean;
  modelInstalled: boolean;
  modelDownloadProgress: { downloaded: number; total: number } | null;
  lastDetectedMood: BaseMood | null;
  lastDetectedIntensity: MoodIntensity | null;
  committedMood: BaseMood | null;
  committedIntensity: MoodIntensity | null;
  isAnalyzing: boolean;

  checkInstallation: () => Promise<void>;
  refreshServiceStatus: () => Promise<void>;
  installServer: () => Promise<void>;
  installModel: () => Promise<void>;
  startServer: () => Promise<void>;
  stopServer: () => Promise<void>;
  setServerStatus: (status: MoodState["serverStatus"]) => void;
  setApiStatus: (status: MoodState["apiStatus"], port?: number) => void;
  setModelDownloadProgress: (progress: { downloaded: number; total: number } | null) => void;
  setLastDetectedMood: (mood: BaseMood | null, intensity?: MoodIntensity | null) => void;
  setCommittedMood: (mood: BaseMood | null, intensity?: MoodIntensity | null) => void;
}

export const useMoodStore = create<MoodState>((set) => ({
  serverStatus: "stopped",
  apiStatus: "disabled",
  apiPort: 8765,
  serverInstalled: false,
  modelInstalled: false,
  modelDownloadProgress: null,
  lastDetectedMood: null,
  lastDetectedIntensity: null,
  committedMood: null,
  committedIntensity: null,
  isAnalyzing: false,

  checkInstallation: async () => {
    try {
      const [serverInstalled, modelInstalled] = await Promise.all([
        commands.checkLlamaServerInstalled(),
        commands.checkMoodModelInstalled(),
      ]);
      set({ serverInstalled, modelInstalled });
      await useMoodStore.getState().refreshServiceStatus();
    } catch (e) {
      console.error("Failed to check mood AI installation:", e);
    }
  },

  refreshServiceStatus: async () => {
    try {
      const status = await commands.getMoodServiceStatus();
      set({
        serverStatus: status.runtime as MoodState["serverStatus"],
        apiStatus: status.api as MoodState["apiStatus"],
        apiPort: status.port,
      });
    } catch (e) {
      console.error("Failed to refresh mood AI service status:", e);
      set({ serverStatus: "stopped", apiStatus: "stopped" });
    }
  },

  installServer: async () => {
    try {
      await commands.installLlamaServer();
      set({ serverInstalled: true });
      useToastStore.getState().addToast("llama-server installed", "success");
    } catch (e) {
      useToastStore.getState().addToast(`Failed to install llama-server: ${e}`, "error");
    }
  },

  installModel: async () => {
    try {
      set({ modelDownloadProgress: { downloaded: 0, total: 0 } });
      await commands.installMoodModel();
      set({ modelInstalled: true, modelDownloadProgress: null });
      useToastStore.getState().addToast("Model downloaded", "success");
    } catch (e) {
      set({ modelDownloadProgress: null });
      useToastStore.getState().addToast(`Failed to download model: ${e}`, "error");
    }
  },

  startServer: async () => {
    try {
      set({ serverStatus: "starting" });
      await commands.startMoodServer();
      await useMoodStore.getState().refreshServiceStatus();
      useToastStore.getState().addToast("Mood AI runtime and extension API started", "success");
    } catch (e) {
      set({ serverStatus: "error" });
      useToastStore.getState().addToast(`Failed to start server: ${e}`, "error");
    }
  },

  stopServer: async () => {
    try {
      await commands.stopMoodServer();
      await useMoodStore.getState().refreshServiceStatus();
      useToastStore.getState().addToast("Mood AI runtime stopped", "info");
    } catch (e) {
      useToastStore.getState().addToast(`Failed to stop server: ${e}`, "error");
    }
  },

  setServerStatus: (status) => set({ serverStatus: status }),
  setApiStatus: (status, port) => set((state) => ({ apiStatus: status, apiPort: port ?? state.apiPort })),
  setModelDownloadProgress: (progress) => set({ modelDownloadProgress: progress }),
  setLastDetectedMood: (mood, intensity) => set({ lastDetectedMood: mood, lastDetectedIntensity: intensity ?? null }),
  setCommittedMood: (mood, intensity) => set({ committedMood: mood, committedIntensity: intensity ?? null }),
}));
