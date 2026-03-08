import { lazy, Suspense, useEffect } from "react";
import { ProfileSelector } from "../Profiles/ProfileSelector";
import { GlobalToggles } from "../Controls/GlobalToggles";
import { StopAllButton } from "../Controls/StopAllButton";
import { NowPlaying } from "../Controls/NowPlaying";
import { useMoodStore } from "../../stores/moodStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { MOOD_DISPLAY, MOOD_COLORS } from "../../utils/moodHelpers";

const DiscoveryPanel = lazy(() => import("../Discovery/DiscoveryPanel").then(m => ({ default: m.DiscoveryPanel })));

function MoodIndicator() {
  const lastMood = useMoodStore((s) => s.lastDetectedMood);
  const lastIntensity = useMoodStore((s) => s.lastDetectedIntensity);
  const committedMood = useMoodStore((s) => s.committedMood);
  const committedIntensity = useMoodStore((s) => s.committedIntensity);
  const serverStatus = useMoodStore((s) => s.serverStatus);
  const apiStatus = useMoodStore((s) => s.apiStatus);
  const serverInstalled = useMoodStore((s) => s.serverInstalled);
  const modelInstalled = useMoodStore((s) => s.modelInstalled);
  const startServer = useMoodStore((s) => s.startServer);
  const stopServer = useMoodStore((s) => s.stopServer);
  const checkInstallation = useMoodStore((s) => s.checkInstallation);
  const refreshServiceStatus = useMoodStore((s) => s.refreshServiceStatus);
  const moodAiEnabled = useSettingsStore((s) => s.config.moodAiEnabled);

  useEffect(() => {
    if (!moodAiEnabled) return;
    checkInstallation();
    let cleanups: (() => void)[] = [];
    import("@tauri-apps/api/event").then(({ listen }) => {
      listen("mood_server_status", () => {
        refreshServiceStatus();
      }).then((u) => { cleanups.push(u); });
      listen("mood_api_status", () => {
        refreshServiceStatus();
      }).then((u) => { cleanups.push(u); });
    });
    return () => { cleanups.forEach((u) => u()); };
  }, [checkInstallation, moodAiEnabled, refreshServiceStatus]);

  if (!moodAiEnabled) return null;

  const ready = serverInstalled && modelInstalled;

  return (
    <div className="border-t border-border-color px-3 py-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <span className={`w-2 h-2 rounded-full shrink-0 ${
            serverStatus === "running" ? "bg-accent-success" :
            serverStatus === "starting" ? "bg-yellow-400 animate-pulse" :
            "bg-text-muted"
          }`} />
          <span className="text-text-muted text-xs">Mood AI</span>
        </div>
        {ready && (serverStatus === "stopped" || serverStatus === "error") && (
          <button
            onClick={startServer}
            className="px-2 py-0.5 text-xs bg-accent-primary/20 text-accent-primary rounded hover:bg-accent-primary/30 transition-colors"
          >
            Start
          </button>
        )}
        {ready && serverStatus === "running" && (
          <button
            onClick={stopServer}
            className="px-2 py-0.5 text-xs bg-accent-error/20 text-accent-error rounded hover:bg-accent-error/30 transition-colors"
          >
            Stop
          </button>
        )}
        {!ready && (
          <span className="text-text-muted text-xs italic">Not installed</span>
        )}
      </div>
      {serverStatus === "running" && apiStatus !== "running" && (
        <div className="text-[11px] text-text-muted">
          Extension API not ready
        </div>
      )}
      {committedMood && (() => {
        const colors = MOOD_COLORS[committedMood];
        return (
          <div className="flex items-center gap-1.5">
            <span className="text-text-muted text-xs">Playing:</span>
            <span className={`text-xs px-1.5 py-0.5 rounded-full ${colors.bg} ${colors.text}`}>
              {MOOD_DISPLAY[committedMood]}
            </span>
            {committedIntensity && (
              <span className="text-text-muted text-xs">
                lv.{committedIntensity}
              </span>
            )}
          </div>
        );
      })()}
      {lastMood && lastMood !== committedMood && (() => {
        const colors = MOOD_COLORS[lastMood];
        return (
          <div className="flex items-center gap-1.5">
            <span className="text-text-muted text-xs">Raw:</span>
            <span className={`text-xs px-1.5 py-0.5 rounded-full opacity-60 ${colors.bg} ${colors.text}`}>
              {MOOD_DISPLAY[lastMood]}
            </span>
            {lastIntensity && (
              <span className="text-text-muted text-xs opacity-60">
                lv.{lastIntensity}
              </span>
            )}
          </div>
        );
      })()}
    </div>
  );
}

export function Sidebar() {
  return (
    <aside className="w-56 bg-bg-secondary border-r border-border-color flex flex-col shrink-0 overflow-hidden">
      <ProfileSelector />

      <div className="border-t border-border-color p-3 space-y-3">
        <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
          Controls
        </h3>
        <GlobalToggles />
        <StopAllButton />
      </div>

      <MoodIndicator />

      <div className="border-t border-border-color flex-1 overflow-y-auto">
        <NowPlaying />
      </div>

      <Suspense fallback={null}>
        <DiscoveryPanel />
      </Suspense>
    </aside>
  );
}
