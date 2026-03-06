import { lazy, Suspense, useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Header } from "./components/Layout/Header";
import { Sidebar } from "./components/Layout/Sidebar";
import { MainContent } from "./components/Layout/MainContent";
import { ToastContainer } from "./components/Toast/ToastContainer";
import { ExportProgress } from "./components/Export/ExportProgress";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { useConfirmStore } from "./stores/confirmStore";
import { useSettingsStore } from "./stores/settingsStore";
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
import { useMoodPlayback } from "./hooks/useMoodPlayback";
import { isTextInput } from "./utils/inputHelpers";
import { getSoundFilePath } from "./utils/soundHelpers";
import { useWaveformStore } from "./stores/waveformStore";

/** Preload waveforms for the initial profile (mirrors profileStore's preloadWaveforms). */
import type { Sound } from "./types";
function preloadWaveformsForProfile(sounds: Sound[]) {
  const entries = sounds
    .filter((s) => s.duration > 0)
    .map((s) => ({ path: getSoundFilePath(s), numPoints: 100 }));
  if (entries.length === 0) return;
  const wfStore = useWaveformStore.getState();
  const needed = entries.filter((e) => !wfStore.waveforms.has(e.path));
  if (needed.length === 0) return;
  wfStore.setLoading(true);
  commands.getWaveformsBatch(needed).then((result) => {
    useWaveformStore.getState().setBatch(result);
  }).catch(() => {
    useWaveformStore.getState().setLoading(false);
  });
}

// Code splitting: lazy load modals and heavy components not needed at startup
const SettingsModal = lazy(() => import("./components/Settings/SettingsModal").then(m => ({ default: m.SettingsModal })));
const FileNotFoundModal = lazy(() => import("./components/Errors/FileNotFoundModal").then(m => ({ default: m.FileNotFoundModal })));
const KeyboardShortcutsModal = lazy(() => import("./components/common/KeyboardShortcutsModal").then(m => ({ default: m.KeyboardShortcutsModal })));

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const currentProfileId = useSettingsStore((s) => s.config.currentProfileId);
  const { loadProfile, currentProfile } = useProfileStore();
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
  useMoodPlayback();

  // Global shortcut: ? or F1 to toggle help modal
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isTextInput(e.target)) return;
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
    commands.getInitialState().then((state) => {
      useSettingsStore.getState().setConfig(state.config);
      useRuntimeStore.getState().setInputRuntime(state.inputRuntime);
      useProfileStore.setState({
        profiles: state.profiles,
        currentProfile: state.currentProfile,
        isLoading: false,
      });
    }).catch((e) => {
      console.error("Failed to load initial state:", e);
      useProfileStore.setState({ isLoading: false });
    });
  }, []);

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

  // Load current profile when ID changes (after initial load, e.g. user switches profile)
  const initialLoadDone = useRef(false);
  useEffect(() => {
    if (!initialLoadDone.current) {
      // Skip the first trigger — initial profile was loaded by getInitialState
      initialLoadDone.current = true;
      return;
    }
    if (currentProfileId) {
      loadProfile(currentProfileId);
    }
  }, [currentProfileId, loadProfile]);

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

  // Fire-and-forget background tasks after initial profile is available
  const bgTasksDone = useRef(false);
  useEffect(() => {
    if (currentProfile && !bgTasksDone.current) {
      bgTasksDone.current = true;
      // Trigger verification + duration computation + waveform preload
      // These are handled inside profileStore's loadProfile flow,
      // but for the initial load via getInitialState we need to trigger them manually
      commands.verifyProfileSounds(currentProfile).catch(() => {});

      // Compute missing durations and preload waveforms (same flow as loadProfile)
      const entries = currentProfile.sounds
        .filter((s) => s.duration === 0)
        .map((s) => ({
          soundId: s.id,
          filePath: getSoundFilePath(s),
          needsDuration: true,
        }));
      if (entries.length > 0) {
        commands.preloadProfileSounds(entries).then((durations) => {
          if (Object.keys(durations).length > 0) {
            useProfileStore.setState((state) => {
              if (!state.currentProfile) return state;
              return {
                currentProfile: {
                  ...state.currentProfile,
                  sounds: state.currentProfile.sounds.map((s) =>
                    durations[s.id] != null ? { ...s, duration: durations[s.id] } : s
                  ),
                },
              };
            });
          }
          // Preload waveforms for sounds that now have durations
          const updatedProfile = useProfileStore.getState().currentProfile;
          if (updatedProfile) {
            preloadWaveformsForProfile(updatedProfile.sounds);
          }
        }).catch(() => {});
      } else {
        // All durations already known — just preload waveforms
        preloadWaveformsForProfile(currentProfile.sounds);
      }
    }
  }, [currentProfile]);

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
      <Header onSettingsClick={() => setShowSettings(true)} onHelpClick={() => setShowHelp(true)} />

      <div className="flex flex-1 overflow-hidden">
        <Sidebar />
        <MainContent />
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
