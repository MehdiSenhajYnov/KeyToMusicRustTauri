import { listen } from "@tauri-apps/api/event";
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
  const committedMood = useMoodStore((s) => s.committedMood);
  const committedIntensity = useMoodStore((s) => s.committedIntensity);
  const sceneSummary = useMoodStore((s) => s.sceneSummary);
  const status = useMoodStore((s) => s.status);
  const refreshServiceStatus = useMoodStore((s) => s.refreshServiceStatus);
  const moodAiEnabled = useSettingsStore((s) => s.config.moodAiEnabled);

  useEffect(() => {
    if (!moodAiEnabled) return;
    refreshServiceStatus();
    const cleanups: (() => void)[] = [];
    let disposed = false;
    const register = (unlisten: () => void) => { if (disposed) unlisten(); else cleanups.push(unlisten); };
    for (const name of ["mood_api_status", "mood_progress"]) {
      listen(name, () => { refreshServiceStatus(); }).then(register);
    }
    return () => { disposed = true; cleanups.forEach((u) => u()); };
  }, [moodAiEnabled, refreshServiceStatus]);

  if (!moodAiEnabled) return null;

  const chapter = status?.chapter ?? null;
  const state = status?.error
    ? { dot: "bg-accent-error", text: status.error }
    : !status?.configured
      ? { dot: "bg-yellow-400", text: "Add an API key in Settings" }
      : status.api !== "running"
        ? { dot: "bg-yellow-400", text: "Extension API stopped" }
        : chapter
          ? { dot: "bg-accent-success", text: `${chapter.analyzed}/${chapter.totalPages} pages analyzed` }
          : { dot: "bg-accent-success", text: "Waiting for a chapter" };

  return (
    <div className="border-t border-border-color px-3 py-2 space-y-1.5">
      <div className="flex items-center gap-1.5">
        <span className={`w-2 h-2 rounded-full shrink-0 ${state.dot}`} />
        <span className="text-text-muted text-xs">Mood AI</span>
      </div>
      <div className="text-[11px] text-text-muted truncate" title={state.text}>{state.text}</div>
      {committedMood && (() => {
        const colors = MOOD_COLORS[committedMood];
        return (
          <div className="flex items-center gap-1.5" title={sceneSummary}>
            <span className="text-text-muted text-xs">Playing:</span>
            <span className={`text-xs px-1.5 py-0.5 rounded-full ${colors.bg} ${colors.text}`}>
              {MOOD_DISPLAY[committedMood]}
            </span>
            {committedIntensity && <span className="text-text-muted text-xs">lv.{committedIntensity}</span>}
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
