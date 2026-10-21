import { useKeyboardLayout } from "./hooks/useKeyboardLayout";
import { isKeyDetectionSuspended } from "./utils/keyDetectionSuspension";
import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Header } from "./components/Layout/Header";
import { Sidebar } from "./components/Layout/Sidebar";
import { MainContent } from "./components/Layout/MainContent";
import { ToastContainer } from "./components/Toast/ToastContainer";
import { ExportProgress } from "./components/Export/ExportProgress";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { useConfirmStore } from "./stores/confirmStore";
import { useSettingsStore, flushSettings } from "./stores/settingsStore";
import { useProfileStore } from "./stores/profileStore";
import { useExportStore } from "./stores/exportStore";
import { useRuntimeStore } from "./stores/runtimeStore";
import { useToastStore } from "./stores/toastStore";
import * as commands from "./utils/tauriCommands";
import { useAudioEvents } from "./hooks/useAudioEvents";
import { useKeyDetection } from "./hooks/useKeyDetection";
import { useTextInputFocus } from "./hooks/useTextInputFocus";
import { useUndoRedo } from "./hooks/useUndoRedo";
import { useDiscovery } from "./hooks/useDiscovery";
import { useDiscoveryPredownload } from "./hooks/useDiscoveryPredownload";
import { useLoopPlayback } from "./hooks/useLoopPlayback";
import { useMoodPlayback } from "./hooks/useMoodPlayback";
import { isTextInput } from "./utils/inputHelpers";
// Code splitting: lazy load modals and heavy components not needed at startup
const SettingsModal = lazy(() => import("./components/Settings/SettingsModal").then(m => ({ default: m.SettingsModal })));
const FileNotFoundModal = lazy(() => import("./components/Errors/FileNotFoundModal").then(m => ({ default: m.FileNotFoundModal })));
const KeyboardShortcutsModal = lazy(() => import("./components/common/KeyboardShortcutsModal").then(m => ({ default: m.KeyboardShortcutsModal })));

function App() {
  useKeyboardLayout();
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const [startupError, setStartupError] = useState<string | null>(null);
  const [startupAttempt, setStartupAttempt] = useState(0);
  const inputRuntime = useRuntimeStore((s) => s.inputRuntime);
  const addToast = useToastStore((s) => s.addToast);
  const backgroundSetupPromptedRef = useRef(false);

  // Initialize hooks
  useAudioEvents();
  useKeyDetection();
  useTextInputFocus();
  useUndoRedo();
  useDiscovery();
  useDiscoveryPredownload();
  useLoopPlayback();
  useMoodPlayback();

  // Global shortcut: ? or F1 to toggle help modal
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented || isKeyDetectionSuspended() || isTextInput(e.target)) return;
      if (e.key === "?" || e.key === "F1") {
        e.preventDefault();
        setShowHelp((prev) => !prev);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Unified initial state load — replaces 3 sequential IPC calls with 1
  useEffect(() => {
    let cancelled = false;
    setStartupError(null);
    commands.getInitialState().then((state) => {
      if (cancelled) return;
      useSettingsStore.getState().setConfig(state.config);
      useRuntimeStore.getState().setInputRuntime(state.inputRuntime);
      useProfileStore.setState({
        profiles: state.profiles,
        currentProfile: state.currentProfile,
        isLoading: false,
      });
      if (state.profileError) useToastStore.getState().addToast(`Could not restore the last profile: ${state.profileError}`, "error");
      if (state.currentProfile) void useProfileStore.getState().prepareProfile(state.currentProfile);
    }).catch((e) => {
      if (cancelled) return;
      setStartupError(String(e));
      console.error("Failed to load initial state:", e);
      useProfileStore.setState({ isLoading: false });
    });
    return () => { cancelled = true; };
  }, [startupAttempt]);

  useEffect(() => {
    if (!inputRuntime.isLinux || !inputRuntime.isWayland) return;
    if (backgroundSetupPromptedRef.current) return;

    let cancelled = false;

    const checkAndEnableBackgroundDetection = async () => {
      try {
        const status = await commands.getLinuxInputAccessStatus();
        if (cancelled || status.backgroundDetectionAvailable || !status.canAutoFix) {
          return;
        }

        backgroundSetupPromptedRef.current = true;
        const confirmed = await useConfirmStore.getState().confirm(
          "Background key detection needs a one-time Linux permission on Wayland. KeyToMusic can enable it automatically now and will show the usual system admin password prompt. Enable it?"
        );

        if (!confirmed || cancelled) {
          return;
        }

        const result = await commands.enableLinuxBackgroundDetection();
        if (cancelled) return;

        addToast(
          result.message,
          result.status.backgroundDetectionAvailable ? "success" : "info"
        );
      } catch (error) {
        if (!cancelled) {
          addToast(String(error), "error");
        }
      }
    };

    void checkAndEnableBackgroundDetection();

    return () => {
      cancelled = true;
    };
  }, [inputRuntime.isLinux, inputRuntime.isWayland, addToast]);

  // Sync profile bindings to backend for multi-key chord detection
  // Deduplicate keyCodes since multi-track bindings share the same keyCode
  useEffect(() => {
    if (currentProfile) {
      const uniqueKeys = [...new Set(currentProfile.keyBindings.map((kb) => kb.keyCode))];
      commands.setProfileBindings(uniqueKeys).catch(console.error);
    } else {
      commands.setProfileBindings([]).catch(console.error);
    }
  }, [currentProfile?.keyBindings]);

  // Intercept window close during export
  const forceCloseRef = useRef(false);
  useEffect(() => {
    const appWindow = getCurrentWindow();
    const unlisten = appWindow.onCloseRequested(async (event) => {
      if (forceCloseRef.current) return;
      event.preventDefault();
      if (useExportStore.getState().isExporting) {
        const confirmed = await useConfirmStore.getState().confirm(
          "An export is in progress. Cancel it and close the application?"
        );
        if (!confirmed) return;
        try { await useExportStore.getState().cancelExport(); } catch (error) {
          useToastStore.getState().addToast(`Could not cancel export: ${error}`, "error");
          return;
        }
      }
      if (!await useProfileStore.getState().saveCurrentProfile()) return;
      await flushSettings();
      try { await commands.flushConfig(); } catch (error) {
        useToastStore.getState().addToast(`Could not save settings before closing: ${error}`, "error");
        return;
      }
      forceCloseRef.current = true;
      await appWindow.close();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  return (
    <div className="h-screen flex flex-col bg-bg-primary">
      <Header onSettingsClick={() => setShowSettings(true)} onHelpClick={() => setShowHelp(true)} />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        {startupError ? (
          <main className="flex-1 flex flex-col items-center justify-center gap-4 p-8" role="alert">
            <h1 className="text-lg font-semibold">Could not open your library</h1>
            <p className="text-text-secondary text-sm max-w-lg break-words">{startupError}</p>
            <button className="px-4 py-2 rounded bg-accent-primary text-white" onClick={() => setStartupAttempt((n) => n + 1)}>Retry</button>
          </main>
        ) : <MainContent />}
      </div>

      <Suspense fallback={null}>
        {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
        {showHelp && <KeyboardShortcutsModal onClose={() => setShowHelp(false)} />}
        <FileNotFoundModal />
      </Suspense>
      <ExportProgress />
      <ToastContainer />
      <ConfirmDialog />
    </div>
  );
}

export default App;
