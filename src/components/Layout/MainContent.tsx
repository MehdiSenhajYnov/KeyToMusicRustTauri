import { lazy, Suspense, useState, useEffect, useRef, useCallback, useMemo } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { TrackView } from "../Tracks/TrackView";
import { KeyGrid, usePlayingSoundIds } from "../Keys/KeyGrid";
import { SoundDetails } from "../Sounds/SoundDetails";
import { MultiKeyDetails } from "../Sounds/MultiKeyDetails";
import { useProfileStore } from "../../stores/profileStore";
import { isAudioFile } from "../../utils/fileHelpers";
import { SearchFilterBar, type SearchFilterBarHandle } from "../common/SearchFilterBar";
import { keyCodeToDisplay } from "../../utils/keyMapping";
import type { KeyGridFilter } from "../../types";

const AddSoundModal = lazy(() => import("../Sounds/AddSoundModal").then(m => ({ default: m.AddSoundModal })));

function MainContentSkeleton() {
  return (
    <main className="flex-1 flex flex-col bg-bg-primary overflow-hidden p-4 space-y-4">
      {/* Track skeleton */}
      <div className="flex gap-3">
        {[1, 2, 3].map((i) => (
          <div key={i} className="flex-1 h-12 bg-bg-tertiary rounded-md animate-pulse" />
        ))}
      </div>
      {/* Key Assignments label skeleton */}
      <div className="h-3 w-32 bg-bg-tertiary rounded animate-pulse" />
      {/* Key grid skeleton */}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(56px,1fr))] gap-1.5">
        {Array.from({ length: 15 }, (_, i) => (
          <div key={i} className="aspect-square bg-bg-tertiary rounded-md animate-pulse" />
        ))}
      </div>
    </main>
  );
}

export function MainContent() {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const isLoading = useProfileStore((s) => s.isLoading);
  const [selectedKeys, setSelectedKeys] = useState<Set<string>>(new Set());
  const [anchorKey, setAnchorKey] = useState<string | null>(null);
  const [showAddSound, setShowAddSound] = useState(false);
  const [droppedFiles, setDroppedFiles] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [panelHeight, setPanelHeight] = useState(256);
  const [filter, setFilter] = useState<KeyGridFilter | null>(null);
  const isResizing = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);
  const containerRef = useRef<HTMLElement>(null);
  const searchBarRef = useRef<SearchFilterBarHandle>(null);

  const handleResizeStart = useCallback((e: React.MouseEvent | React.TouchEvent) => {
    e.preventDefault();
    isResizing.current = true;
    const clientY = "touches" in e ? e.touches[0].clientY : e.clientY;
    startY.current = clientY;
    startHeight.current = panelHeight;
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, [panelHeight]);

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent | TouchEvent) => {
      if (!isResizing.current || !containerRef.current) return;
      const clientY = "touches" in e ? e.touches[0].clientY : e.clientY;
      const delta = startY.current - clientY;
      const containerHeight = containerRef.current.clientHeight;
      const maxHeight = containerHeight - 100;
      requestAnimationFrame(() => {
        const newHeight = Math.min(maxHeight, Math.max(120, startHeight.current + delta));
        setPanelHeight(newHeight);
      });
    };

    const handleMouseUp = () => {
      if (!isResizing.current) return;
      isResizing.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    window.addEventListener("touchmove", handleMouseMove);
    window.addEventListener("touchend", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      window.removeEventListener("touchmove", handleMouseMove);
      window.removeEventListener("touchend", handleMouseUp);
    };
  }, []);

  // Listen for Tauri drag-drop events
  useEffect(() => {
    const appWindow = getCurrentWebviewWindow();
    const unlisten = appWindow.onDragDropEvent((event) => {
      const { type } = event.payload;
      if (type === "enter" || type === "over") {
        setIsDragOver(true);
      } else if (type === "drop") {
        setIsDragOver(false);
        const audioFiles = event.payload.paths.filter(isAudioFile);
        if (audioFiles.length > 0 && currentProfile && currentProfile.tracks.length > 0) {
          setDroppedFiles(audioFiles);
          setShowAddSound(true);
        }
      } else {
        setIsDragOver(false);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [currentProfile]);

  // Reset selection and filter when profile changes
  useEffect(() => {
    setSelectedKeys(new Set());
    setAnchorKey(null);
    setFilter(null);
  }, [currentProfile?.id]);

  // Ctrl+F to focus search bar
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "f") {
        e.preventDefault();
        searchBarRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Filter out stale keys (e.g. after undo/redo removes bindings)
  const validSelectedKeys = currentProfile
    ? new Set([...selectedKeys].filter((k) => currentProfile.keyBindings.some((kb) => kb.keyCode === k)))
    : new Set<string>();

  // Reuse the same stable hook as KeyGrid (shallow-compared, no re-render on position-only changes)
  const playingSoundIds = usePlayingSoundIds();

  const matchingKeys = useMemo(() => {
    if (!currentProfile || !filter) return null;

    const { keyBindings, sounds, tracks } = currentProfile;
    const matched = new Set<string>();

    for (const kb of keyBindings) {
      // Text search
      const matchesText = !filter.searchText || (() => {
        const text = filter.searchText.toLowerCase();
        if (keyCodeToDisplay(kb.keyCode).toLowerCase().includes(text)) return true;
        if (kb.name?.toLowerCase().includes(text)) return true;
        return kb.soundIds.some((sid) => {
          const sound = sounds.find((s) => s.id === sid);
          return sound?.name.toLowerCase().includes(text);
        });
      })();

      // Track filter
      const matchesTrack = !filter.trackName || (() => {
        const track = tracks.find((t) => t.id === kb.trackId);
        return track?.name.toLowerCase().includes(filter.trackName!.toLowerCase()) ?? false;
      })();

      // Loop mode filter
      const matchesLoop = !filter.loopMode || kb.loopMode === filter.loopMode;

      // Status filter
      const matchesStatus = !filter.status || (() => {
        const isPlaying = kb.soundIds.some((id) => playingSoundIds.has(id));
        return filter.status === "playing" ? isPlaying : !isPlaying;
      })();

      if (matchesText && matchesTrack && matchesLoop && matchesStatus) {
        matched.add(kb.keyCode);
      }
    }

    return matched;
  }, [currentProfile, filter, playingSoundIds]);

  const handleKeySelect = useCallback((keyCode: string, event: React.MouseEvent) => {
    if (!currentProfile) return;
    const bindings = currentProfile.keyBindings;

    if (event.ctrlKey || event.metaKey) {
      // Ctrl+Click: toggle in selection
      setSelectedKeys((prev) => {
        const next = new Set(prev);
        if (next.has(keyCode)) next.delete(keyCode);
        else next.add(keyCode);
        return next;
      });
      setAnchorKey(keyCode);
    } else if (event.shiftKey && anchorKey) {
      // Shift+Click: range selection
      const anchorIdx = bindings.findIndex((kb) => kb.keyCode === anchorKey);
      const targetIdx = bindings.findIndex((kb) => kb.keyCode === keyCode);
      if (anchorIdx !== -1 && targetIdx !== -1) {
        const start = Math.min(anchorIdx, targetIdx);
        const end = Math.max(anchorIdx, targetIdx);
        const rangeKeys = bindings.slice(start, end + 1).map((kb) => kb.keyCode);
        setSelectedKeys(new Set(rangeKeys));
      }
      // anchorKey doesn't change on Shift+Click
    } else {
      // Simple click: single selection (toggle if already the only one selected)
      if (selectedKeys.size === 1 && selectedKeys.has(keyCode)) {
        setSelectedKeys(new Set());
        setAnchorKey(null);
      } else {
        setSelectedKeys(new Set([keyCode]));
        setAnchorKey(keyCode);
      }
    }
  }, [currentProfile, anchorKey, selectedKeys]);

  const handleSelectAll = useCallback(() => {
    if (!currentProfile) return;
    const allKeys = currentProfile.keyBindings
      .filter((kb) => !matchingKeys || matchingKeys.has(kb.keyCode))
      .map((kb) => kb.keyCode);
    setSelectedKeys(new Set(allKeys));
  }, [currentProfile, matchingKeys]);

  const handleCloseModal = () => {
    setShowAddSound(false);
    setDroppedFiles([]);
  };

  if (isLoading) {
    return <MainContentSkeleton />;
  }

  if (!currentProfile) {
    return (
      <main className="flex-1 flex items-center justify-center bg-bg-primary">
        <div className="text-center">
          <p className="text-text-muted text-lg">No profile selected</p>
          <p className="text-text-muted text-sm mt-1">
            Create or select a profile to get started
          </p>
        </div>
      </main>
    );
  }

  return (
    <main ref={containerRef} className="flex-1 flex flex-col bg-bg-primary overflow-hidden relative">
      {/* Drag overlay */}
      {isDragOver && (
        <div className="absolute inset-0 z-40 bg-accent-primary/10 border-2 border-dashed border-accent-primary rounded-lg flex items-center justify-center">
          <p className="text-accent-primary text-lg font-semibold">
            Drop audio files here
          </p>
        </div>
      )}

      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        <TrackView />

        <div className="flex items-center justify-between gap-2">
          <h2 className="text-text-primary text-sm font-semibold whitespace-nowrap">
            Key Assignments
          </h2>
          <SearchFilterBar
            ref={searchBarRef}
            totalCount={currentProfile.keyBindings.length}
            matchCount={matchingKeys?.size ?? currentProfile.keyBindings.length}
            onFilterChange={setFilter}
            tracks={currentProfile.tracks}
          />
          {currentProfile.tracks.length > 0 && (
            <button
              onClick={() => setShowAddSound(true)}
              className="px-3 py-1.5 bg-accent-primary text-white text-xs rounded hover:bg-accent-primary/80 whitespace-nowrap"
            >
              + Add Sound
            </button>
          )}
        </div>

        <KeyGrid
          selectedKeys={validSelectedKeys}
          onKeySelect={handleKeySelect}
          onSelectAll={handleSelectAll}
          matchingKeys={matchingKeys}
        />

        {currentProfile.tracks.length === 0 && (
          <p className="text-text-muted text-xs italic text-center py-4">
            Create a track first, then add sounds and assign keys
          </p>
        )}
      </div>

      {validSelectedKeys.size > 0 && (
        <>
          <div
            onMouseDown={handleResizeStart}
            onTouchStart={handleResizeStart}
            className="h-1.5 shrink-0 cursor-ns-resize bg-border-color hover:bg-accent-primary/50 transition-colors"
          />
          <div
            style={{ height: panelHeight }}
            className="overflow-y-auto shrink-0"
          >
            {validSelectedKeys.size === 1 ? (
              <SoundDetails
                selectedKey={[...validSelectedKeys][0]}
                onClose={() => setSelectedKeys(new Set())}
                onKeyChanged={(newKey) => {
                  setSelectedKeys(new Set([newKey]));
                  setAnchorKey(newKey);
                }}
              />
            ) : (
              <MultiKeyDetails
                selectedKeys={validSelectedKeys}
                onClose={() => setSelectedKeys(new Set())}
              />
            )}
          </div>
        </>
      )}

      <Suspense fallback={null}>
        {showAddSound && (
          <AddSoundModal
            initialFiles={droppedFiles.length > 0 ? droppedFiles : undefined}
            onClose={handleCloseModal}
          />
        )}
      </Suspense>
    </main>
  );
}
