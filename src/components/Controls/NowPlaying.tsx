import { useState } from "react";
import { useAudioStore } from "../../stores/audioStore";
import { useProfileStore } from "../../stores/profileStore";
import { useTrackPosition } from "../../hooks/useTrackPosition";
import { formatDuration } from "../../utils/fileHelpers";
import * as commands from "../../utils/tauriCommands";
import { getSoundFilePath } from "../../utils/soundHelpers";
import { useWheelSlider, getWheelActiveClass } from "../../hooks/useWheelSlider";

interface NowPlayingTrackProps {
  trackId: string;
  soundId: string;
}

function NowPlayingTrack({ trackId, soundId }: NowPlayingTrackProps) {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const position = useTrackPosition(trackId);
  const [seekingTrack, setSeekingTrack] = useState(false);
  const [seekPosition, setSeekPosition] = useState(0);

  if (!currentProfile) return null;

  const track = currentProfile.tracks.find((t) => t.id === trackId);
  const sound = currentProfile.sounds.find((s) => s.id === soundId);
  if (!sound) return null;

  const handleSeek = async (pos: number) => {
    const filePath = getSoundFilePath(sound);
    useAudioStore.getState().updateProgress(trackId, pos);
    try {
      await commands.playSound(trackId, soundId, filePath, pos, sound.volume);
    } catch (e) {
      console.error("Seek failed:", e);
    }
  };

  const { ref: seekWheelRef, isWheelActive: seekWheelActive } = useWheelSlider({
    value: seekingTrack ? seekPosition : position,
    min: 0, max: sound.duration > 0 ? sound.duration : 1, step: 0.5,
    onChange: (v) => handleSeek(v),
  });

  const handleStop = async () => {
    try {
      await commands.stopSound(trackId);
    } catch (e) {
      console.error("Stop failed:", e);
    }
  };

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between">
        <div className="flex-1 min-w-0">
          <p className="text-text-muted text-xs">{track?.name || "Track"}</p>
          <p className="text-text-primary text-sm truncate">{sound.name}</p>
        </div>
        <button
          onClick={handleStop}
          className="w-5 h-5 flex items-center justify-center rounded bg-accent-error/20 text-accent-error shrink-0 ml-2 hover:bg-accent-error/30"
          title="Stop"
        >
          <span className="text-xs">{"\u25A0"}</span>
        </button>
      </div>
      <div className="flex items-center gap-1.5">
        <span className="text-text-muted text-[10px] w-7 text-right shrink-0">
          {formatDuration(position)}
        </span>
        <input
          ref={seekWheelRef}
          type="range"
          min="0"
          max={sound.duration > 0 ? sound.duration : 1}
          step="0.5"
          value={seekingTrack ? seekPosition : position}
          disabled={sound.duration === 0}
          onChange={(e) => {
            setSeekingTrack(true);
            setSeekPosition(Number(e.target.value));
          }}
          onMouseUp={() => {
            if (seekingTrack) {
              handleSeek(seekPosition);
              setSeekingTrack(false);
            }
          }}
          onTouchEnd={() => {
            if (seekingTrack) {
              handleSeek(seekPosition);
              setSeekingTrack(false);
            }
          }}
          className={`flex-1 h-1 accent-accent-primary disabled:opacity-30 cursor-pointer transition-all duration-200 ${getWheelActiveClass(seekWheelActive)}`}
        />
        <span className="text-text-muted text-[10px] w-7 shrink-0">
          {formatDuration(sound.duration)}
        </span>
      </div>
    </div>
  );
}

export function NowPlaying() {
  const playingTracks = useAudioStore((s) => s.playingTracks);
  const currentProfile = useProfileStore((s) => s.currentProfile);

  const hasNonPreviewTracks = Array.from(playingTracks.keys()).some(
    (id) => id !== "__preview__"
  );

  if (!currentProfile || !hasNonPreviewTracks) {
    return (
      <div className="p-3">
        <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider mb-2">
          Now Playing
        </h3>
        <p className="text-text-muted text-xs italic">Nothing playing</p>
      </div>
    );
  }

  const entries = Array.from(playingTracks.values()).filter(
    (entry) => entry.trackId !== "__preview__"
  );

  return (
    <div className="p-3 space-y-2">
      <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
        Now Playing
      </h3>
      {entries.map((entry) => (
        <NowPlayingTrack
          key={entry.trackId}
          trackId={entry.trackId}
          soundId={entry.soundId}
        />
      ))}
    </div>
  );
}
