import { useProfileStore } from "../../stores/profileStore";
import { useAudioStore } from "../../stores/audioStore";
import { keyCodeToDisplay } from "../../utils/keyMapping";

interface KeyGridProps {
  selectedKey: string | null;
  onKeySelect: (key: string | null) => void;
}

export function KeyGrid({ selectedKey, onKeySelect }: KeyGridProps) {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const playingTracks = useAudioStore((s) => s.playingTracks);
  const lastKeyPressed = useAudioStore((s) => s.lastKeyPressed);

  if (!currentProfile) return null;

  const { keyBindings, sounds } = currentProfile;

  return (
    <div className="space-y-2">
      {keyBindings.length === 0 ? (
        <p className="text-text-muted text-xs italic">
          No keys assigned. Use "Add Sound" to create key bindings.
        </p>
      ) : (
        <div className="flex flex-wrap gap-2">
          {keyBindings.map((kb) => {
            const firstSound = sounds.find((s) => kb.soundIds.includes(s.id));
            const displayName = kb.name || firstSound?.name || "No sound";
            const soundCount = kb.soundIds.length;
            const isSelected = selectedKey === kb.keyCode;
            const isPlaying = Array.from(playingTracks.values()).some(
              (pt) => kb.soundIds.includes(pt.soundId)
            );
            const isJustPressed = lastKeyPressed === kb.keyCode;

            return (
              <button
                key={kb.keyCode}
                onClick={() =>
                  onKeySelect(isSelected ? null : kb.keyCode)
                }
                className={`
                  relative px-3 py-2 rounded border text-left min-w-[120px] transition-all
                  ${isSelected
                    ? "border-accent-primary bg-accent-primary/10"
                    : "border-border-color bg-bg-secondary hover:border-border-focus"
                  }
                  ${isPlaying ? "ring-1 ring-accent-success" : ""}
                  ${isJustPressed ? "scale-95" : ""}
                `}
              >
                <div className="flex items-center gap-2">
                  <span className="text-accent-primary font-mono text-xs font-bold bg-bg-tertiary px-1.5 py-0.5 rounded">
                    {keyCodeToDisplay(kb.keyCode)}
                  </span>
                  {isPlaying && (
                    <span className="w-1.5 h-1.5 rounded-full bg-accent-success animate-pulse" />
                  )}
                </div>
                <p className="text-text-primary text-xs mt-1 truncate">
                  {displayName}
                </p>
                <span className="text-text-muted text-xs">
                  {soundCount} {soundCount > 1 ? "sons" : "son"}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
