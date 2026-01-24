import { useState } from "react";
import { useAudioStore } from "../../stores/audioStore";
import { useProfileStore } from "../../stores/profileStore";
import { formatDuration } from "../../utils/fileHelpers";
import * as commands from "../../utils/tauriCommands";
import type { Sound } from "../../types";

function getSoundFilePath(sound: Sound): string {
  if (sound.source.type === "local") return sound.source.path;
  return sound.source.cachedPath;
}

export function NowPlaying() {
  const playingTracks = useAudioStore((s) => s.playingTracks);
  const updateProgress = useAudioStore((s) => s.updateProgress);
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const [seekingTrack, setSeekingTrack] = useState<string | null>(null);
  const [seekPosition, setSeekPosition] = useState(0);

  if (!currentProfile || playingTracks.size === 0) {
    return (
      <div className="p-3">
        <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider mb-2">
          Now Playing
        </h3>
        <p className="text-text-muted text-xs italic">Nothing playing</p>
      </div>
    );
  }

  const entries = Array.from(playingTracks.values());

  const handleSeek = async (trackId: string, soundId: string, position: number) => {
    const sound = currentProfile.sounds.find((s) => s.id === soundId);
    if (!sound) return;
    const filePath = getSoundFilePath(sound);
    // Immediately update store position to prevent slider jumping back
    updateProgress(trackId, position);
    try {
      await commands.playSound(trackId, soundId, filePath, position, sound.volume);
    } catch (e) {
      console.error("Seek failed:", e);
    }
  };

  const handleStop = async (trackId: string) => {
    try {
      await commands.stopSound(trackId);
    } catch (e) {
      console.error("Stop failed:", e);
    }
  };

  return (
    <div className="p-3 space-y-2">
      <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
        Now Playing
      </h3>
      {entries.map((entry) => {
        const track = currentProfile.tracks.find((t) => t.id === entry.trackId);
        const sound = currentProfile.sounds.find((s) => s.id === entry.soundId);
        if (!sound) return null;

        return (
          <div key={entry.trackId} className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="flex-1 min-w-0">
                <p className="text-text-muted text-xs">{track?.name || "Track"}</p>
                <p className="text-text-primary text-sm truncate">{sound.name}</p>
              </div>
              <button
                onClick={() => handleStop(entry.trackId)}
                className="w-5 h-5 flex items-center justify-center rounded bg-accent-error/20 text-accent-error shrink-0 ml-2 hover:bg-accent-error/30"
                title="Stop"
              >
                <span className="text-xs">{"\u25A0"}</span>
              </button>
            </div>
            <div className="flex items-center gap-1.5">
              <span className="text-text-muted text-[10px] w-7 text-right shrink-0">
                {formatDuration(entry.position)}
              </span>
              <input
                type="range"
                min="0"
                max={sound.duration > 0 ? sound.duration : 1}
                step="0.5"
                value={seekingTrack === entry.trackId ? seekPosition : entry.position}
                disabled={sound.duration === 0}
                onChange={(e) => {
                  setSeekingTrack(entry.trackId);
                  setSeekPosition(Number(e.target.value));
                }}
                onMouseUp={() => {
                  if (seekingTrack === entry.trackId) {
                    handleSeek(entry.trackId, entry.soundId, seekPosition);
                    setSeekingTrack(null);
                  }
                }}
                onTouchEnd={() => {
                  if (seekingTrack === entry.trackId) {
                    handleSeek(entry.trackId, entry.soundId, seekPosition);
                    setSeekingTrack(null);
                  }
                }}
                className="flex-1 h-1 accent-accent-primary disabled:opacity-30 cursor-pointer"
              />
              <span className="text-text-muted text-[10px] w-7 shrink-0">
                {formatDuration(sound.duration)}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
