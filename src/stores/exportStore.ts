import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import * as commands from "../utils/tauriCommands";
import { useProfileStore } from "./profileStore";
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
  cancelExport: () => Promise<void>;
}

let completion: Promise<void> = Promise.resolve();
let cancelRequested = false;

export const useExportStore = create<ExportState>((set, get) => ({
  isExporting: false,
  progress: null,

  startExport: async (profileId, profileName) => {
    if (get().isExporting) return;
    cancelRequested = false;
    let finish!: () => void;
    completion = new Promise<void>((resolve) => { finish = resolve; });
    set({ isExporting: true, progress: null });
    let unlisten: (() => void) | undefined;
    try {
      const defaultName = `${profileName.replace(/[^a-zA-Z0-9_\- ]/g, "") || "Profile"}.ktm`;
      const path = await commands.pickSaveLocation(defaultName);
      if (!path) return;
      if (!await useProfileStore.getState().saveCurrentProfile()) return;
      set({ progress: { current: 0, total: 0, filename: "" } });
      unlisten = await listen<ExportProgress>("export_progress", (event) => {
        set({ progress: event.payload });
      });
      if (cancelRequested) return;
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
      unlisten?.();
      set({ isExporting: false, progress: null });
      finish();
    }
  },

  cancelExport: async () => {
    cancelRequested = true;
    await commands.cancelExport();
    await completion;
  },
}));
