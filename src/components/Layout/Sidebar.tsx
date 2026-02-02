import { lazy, Suspense } from "react";
import { ProfileSelector } from "../Profiles/ProfileSelector";
import { GlobalToggles } from "../Controls/GlobalToggles";
import { StopAllButton } from "../Controls/StopAllButton";
import { NowPlaying } from "../Controls/NowPlaying";

const DiscoveryPanel = lazy(() => import("../Discovery/DiscoveryPanel").then(m => ({ default: m.DiscoveryPanel })));

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

      <div className="border-t border-border-color flex-1 overflow-y-auto">
        <NowPlaying />
      </div>

      <Suspense fallback={null}>
        <DiscoveryPanel />
      </Suspense>
    </aside>
  );
}
