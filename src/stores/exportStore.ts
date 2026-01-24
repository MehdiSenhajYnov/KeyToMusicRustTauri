import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import * as commands from "../utils/tauriCommands";
import { useToastStore } from "./toastStore";

interface ExportProgress {
  current: number;
  total: number;
  filename: string;
}

interface ExportState {
  isExporting: boolean;
  progress: ExportProgress | null;
  startExport: (profileId: string, profileName: string) => Promise<void>;
  cancelExport: () => void;
}

export const useExportStore = create<ExportState>((set) => ({
  isExporting: false,
  progress: null,

  startExport: async (profileId, profileName) => {
    const defaultName = `${profileName.replace(/[^a-zA-Z0-9_\- ]/g, "")}.ktm`;
    const path = await commands.pickSaveLocation(defaultName);
    if (!path) return;

    set({ isExporting: true, progress: { current: 0, total: 0, filename: "" } });

    const unlisten = await listen<ExportProgress>("export_progress", (event) => {
      set({ progress: event.payload });
    });

    try {
      await commands.exportProfile(profileId, path);
      useToastStore.getState().addToast("Export completed!", "success");
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Export cancelled")) {
        useToastStore.getState().addToast("Export cancelled", "info");
      } else {
        useToastStore.getState().addToast(`Export failed: ${e}`, "error");
      }
    } finally {
      unlisten();
      set({ isExporting: false, progress: null });
    }
  },

  cancelExport: () => {
    commands.cancelExport();
  },
}));
