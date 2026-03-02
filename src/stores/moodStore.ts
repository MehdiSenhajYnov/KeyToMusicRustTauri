import { create } from "zustand";
import type { MoodCategory } from "../types";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";

interface MoodState {
  serverStatus: "stopped" | "starting" | "running" | "error";
  serverInstalled: boolean;
  modelInstalled: boolean;
  modelDownloadProgress: { downloaded: number; total: number } | null;
  lastDetectedMood: MoodCategory | null;
  isAnalyzing: boolean;

  checkInstallation: () => Promise<void>;
  installServer: () => Promise<void>;
  installModel: () => Promise<void>;
  startServer: () => Promise<void>;
  stopServer: () => Promise<void>;
  setServerStatus: (status: MoodState["serverStatus"]) => void;
  setModelDownloadProgress: (progress: { downloaded: number; total: number } | null) => void;
  setLastDetectedMood: (mood: MoodCategory | null) => void;
}

export const useMoodStore = create<MoodState>((set) => ({
  serverStatus: "stopped",
  serverInstalled: false,
  modelInstalled: false,
  modelDownloadProgress: null,
  lastDetectedMood: null,
  isAnalyzing: false,

  checkInstallation: async () => {
    try {
      const [serverInstalled, modelInstalled] = await Promise.all([
        commands.checkLlamaServerInstalled(),
        commands.checkMoodModelInstalled(),
      ]);
      set({ serverInstalled, modelInstalled });

      // Also check server status
      const status = await commands.getMoodServerStatus();
      set({ serverStatus: status as MoodState["serverStatus"] });
    } catch (e) {
      console.error("Failed to check mood AI installation:", e);
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
      set({ serverStatus: "running" });
      useToastStore.getState().addToast("Mood AI server started", "success");
    } catch (e) {
      set({ serverStatus: "error" });
      useToastStore.getState().addToast(`Failed to start server: ${e}`, "error");
    }
  },

  stopServer: async () => {
    try {
      await commands.stopMoodServer();
      set({ serverStatus: "stopped" });
      useToastStore.getState().addToast("Mood AI server stopped", "info");
    } catch (e) {
      useToastStore.getState().addToast(`Failed to stop server: ${e}`, "error");
    }
  },

  setServerStatus: (status) => set({ serverStatus: status }),
  setModelDownloadProgress: (progress) => set({ modelDownloadProgress: progress }),
  setLastDetectedMood: (mood) => set({ lastDetectedMood: mood }),
}));
