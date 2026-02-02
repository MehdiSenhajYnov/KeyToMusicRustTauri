import { useState, useRef, useEffect, useCallback } from "react";
import { useProfileStore } from "../../stores/profileStore";
import { setTrackVolume } from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { useConfirmStore } from "../../stores/confirmStore";
import { useWheelSlider } from "../../hooks/useWheelSlider";

function WheelInput(props: React.InputHTMLAttributes<HTMLInputElement> & { wheelStep: number; wheelMin: number; wheelMax: number; onWheelChange: (v: number) => void }) {
  const { wheelStep, wheelMin, wheelMax, onWheelChange, ...inputProps } = props;
  const ref = useWheelSlider({
    value: Number(inputProps.value ?? 0),
    min: wheelMin, max: wheelMax, step: wheelStep,
    onChange: onWheelChange,
  });
  return <input ref={ref} {...inputProps} />;
}

export function TrackView() {
  const { currentProfile, addTrack, removeTrack, updateTrack, saveCurrentProfile } =
    useProfileStore();
  const addToast = useToastStore((s) => s.addToast);
  const showConfirm = useConfirmStore((s) => s.confirm);
  const [isAdding, setIsAdding] = useState(false);
  const [newTrackName, setNewTrackName] = useState("");
  const [editingTrackId, setEditingTrackId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Clear timer on unmount to prevent leaks
  useEffect(() => {
    return () => {
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    };
  }, []);

  const handleRenameStart = (trackId: string, currentName: string) => {
    setEditingTrackId(trackId);
    setEditingName(currentName);
  };

  const handleRenameConfirm = () => {
    if (!editingTrackId || !editingName.trim()) {
      setEditingTrackId(null);
      return;
    }
    updateTrack(editingTrackId, { name: editingName.trim() });
    setEditingTrackId(null);
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => saveCurrentProfile(), 200);
  };

  if (!currentProfile) return null;

  const handleAddTrack = async () => {
    if (!newTrackName.trim()) return;
    if (currentProfile.tracks.length >= 20) {
      addToast("Maximum 20 tracks allowed", "warning");
      return;
    }
    const id = crypto.randomUUID();
    addTrack({
      id,
      name: newTrackName.trim(),
      volume: 1.0,
    });
    setNewTrackName("");
    setIsAdding(false);
    setTimeout(() => saveCurrentProfile(), 100);
    addToast(`Track "${newTrackName.trim()}" created`, "success");
  };

  const handleDeleteTrack = async (trackId: string, trackName: string) => {
    if (!await showConfirm(`Delete track "${trackName}"?`)) return;
    removeTrack(trackId);
    setTimeout(() => saveCurrentProfile(), 100);
    addToast(`Track "${trackName}" deleted`, "info");
  };

  const volumeDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleVolumeChange = useCallback((trackId: string, volume: number) => {
    // Update store immediately for responsive UI
    updateTrack(trackId, { volume });
    // Debounce the backend call
    if (volumeDebounceRef.current) clearTimeout(volumeDebounceRef.current);
    volumeDebounceRef.current = setTimeout(() => {
      setTrackVolume(trackId, volume).catch(console.error);
    }, 100);
  }, [updateTrack]);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <h2 className="text-text-primary text-sm font-semibold">Tracks</h2>
        {!isAdding && (
          <button
            onClick={() => setIsAdding(true)}
            className="text-accent-primary text-xs hover:underline"
          >
            + Add Track
          </button>
        )}
      </div>

      {isAdding && (
        <div className="flex gap-2 items-center">
          <input
            type="text"
            value={newTrackName}
            onChange={(e) => setNewTrackName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleAddTrack();
              if (e.key === "Escape") setIsAdding(false);
            }}
            placeholder="Track name"
            className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary focus:border-border-focus outline-none"
            autoFocus
          />
          <button
            onClick={handleAddTrack}
            className="text-accent-success text-sm px-2 py-1 rounded hover:bg-bg-hover"
          >
            Add
          </button>
          <button
            onClick={() => setIsAdding(false)}
            className="text-text-muted text-sm px-2 py-1 rounded hover:bg-bg-hover"
          >
            Cancel
          </button>
        </div>
      )}

      <div className="space-y-1">
        {currentProfile.tracks.map((track) => (
          <div
            key={track.id}
            className="flex items-center gap-3 bg-bg-secondary rounded px-3 py-2 group"
          >
            {editingTrackId === track.id ? (
              <input
                type="text"
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onBlur={handleRenameConfirm}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRenameConfirm();
                  if (e.key === "Escape") setEditingTrackId(null);
                }}
                className="w-28 bg-bg-tertiary border border-border-focus rounded px-1 py-0.5 text-sm text-text-primary outline-none"
                autoFocus
              />
            ) : (
              <span
                onDoubleClick={() => handleRenameStart(track.id, track.name)}
                className="text-text-primary text-sm flex-shrink-0 w-28 truncate cursor-pointer"
                title="Double-click to rename"
              >
                {track.name}
              </span>
            )}
            <WheelInput
              type="range"
              min="0"
              max="100"
              value={Math.round(track.volume * 100)}
              onChange={(e) =>
                handleVolumeChange(track.id, Number(e.target.value) / 100)
              }
              className="flex-1 h-1 accent-accent-secondary"
              wheelStep={1} wheelMin={0} wheelMax={100}
              onWheelChange={(v) => handleVolumeChange(track.id, v / 100)}
            />
            <span className="text-text-muted text-xs w-8">
              {Math.round(track.volume * 100)}%
            </span>
            <button
              onClick={() => handleDeleteTrack(track.id, track.name)}
              className="opacity-0 group-hover:opacity-100 text-text-muted hover:text-accent-error text-xs transition-opacity"
              title="Delete track"
            >
              x
            </button>
          </div>
        ))}
      </div>

      {currentProfile.tracks.length === 0 && (
        <p className="text-text-muted text-xs italic">
          No tracks yet. Add a track to assign sounds.
        </p>
      )}
    </div>
  );
}
