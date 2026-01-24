import { useState, useEffect, useRef, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { TrackView } from "../Tracks/TrackView";
import { KeyGrid } from "../Keys/KeyGrid";
import { SoundDetails } from "../Sounds/SoundDetails";
import { AddSoundModal } from "../Sounds/AddSoundModal";
import { useProfileStore } from "../../stores/profileStore";
import { isAudioFile } from "../../utils/fileHelpers";

export function MainContent() {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [showAddSound, setShowAddSound] = useState(false);
  const [droppedFiles, setDroppedFiles] = useState<string[]>([]);
  const [isDragOver, setIsDragOver] = useState(false);
  const [panelHeight, setPanelHeight] = useState(256);
  const isResizing = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);
  const containerRef = useRef<HTMLElement>(null);

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
      const newHeight = Math.min(maxHeight, Math.max(120, startHeight.current + delta));
      setPanelHeight(newHeight);
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

  const handleCloseModal = () => {
    setShowAddSound(false);
    setDroppedFiles([]);
  };

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

        <div className="flex items-center justify-between">
          <h2 className="text-text-primary text-sm font-semibold">
            Key Assignments
          </h2>
          {currentProfile.tracks.length > 0 && (
            <button
              onClick={() => setShowAddSound(true)}
              className="px-3 py-1.5 bg-accent-primary text-white text-xs rounded hover:bg-accent-primary/80"
            >
              + Add Sound
            </button>
          )}
        </div>

        <KeyGrid
          selectedKey={selectedKey}
          onKeySelect={setSelectedKey}
        />

        {currentProfile.tracks.length === 0 && (
          <p className="text-text-muted text-xs italic text-center py-4">
            Create a track first, then add sounds and assign keys
          </p>
        )}
      </div>

      {selectedKey && (
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
            <SoundDetails
              selectedKey={selectedKey}
              onClose={() => setSelectedKey(null)}
              onKeyChanged={setSelectedKey}
            />
          </div>
        </>
      )}

      {showAddSound && (
        <AddSoundModal
          initialFiles={droppedFiles.length > 0 ? droppedFiles : undefined}
          onClose={handleCloseModal}
        />
      )}
    </main>
  );
}
