import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Header } from "./components/Layout/Header";
import { Sidebar } from "./components/Layout/Sidebar";
import { MainContent } from "./components/Layout/MainContent";
import { SettingsModal } from "./components/Settings/SettingsModal";
import { ToastContainer } from "./components/Toast/ToastContainer";
import { ExportProgress } from "./components/Export/ExportProgress";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { useConfirmStore } from "./stores/confirmStore";
import { useSettingsStore } from "./stores/settingsStore";
import { useProfileStore } from "./stores/profileStore";
import { useExportStore } from "./stores/exportStore";
import * as commands from "./utils/tauriCommands";
import { useAudioEvents } from "./hooks/useAudioEvents";
import { useKeyDetection } from "./hooks/useKeyDetection";
import { useTextInputFocus } from "./hooks/useTextInputFocus";

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const loadConfig = useSettingsStore((s) => s.loadConfig);
  const currentProfileId = useSettingsStore((s) => s.config.currentProfileId);
  const { loadProfiles, loadProfile } = useProfileStore();

  // Initialize hooks
  useAudioEvents();
  useKeyDetection();
  useTextInputFocus();

  // Load config and profiles on mount
  useEffect(() => {
    loadConfig();
    loadProfiles();
  }, [loadConfig, loadProfiles]);

  // Load current profile when ID changes
  useEffect(() => {
    if (currentProfileId) {
      loadProfile(currentProfileId);
    }
  }, [currentProfileId, loadProfile]);

  // Intercept window close during export
  const forceCloseRef = useRef(false);
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onCloseRequested(async (event) => {
      if (forceCloseRef.current) return;
      if (useExportStore.getState().isExporting) {
        event.preventDefault();
        const confirmed = await useConfirmStore.getState().confirm(
          "An export is in progress. If you close, the export will be cancelled and the file will be incomplete. Close anyway?"
        );
        if (confirmed) {
          await commands.cleanupExportTemp();
          forceCloseRef.current = true;
          appWindow.close();
        }
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="h-screen flex flex-col bg-bg-primary">
      <Header onSettingsClick={() => setShowSettings(true)} />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <MainContent />
      </div>

      {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
      <ExportProgress />
      <ToastContainer />
      <ConfirmDialog />
    </div>
  );
}

export default App;
