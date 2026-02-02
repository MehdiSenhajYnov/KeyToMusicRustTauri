import { useRef } from "react";
import { useProfileStore } from "../../stores/profileStore";
import { useAudioStore } from "../../stores/audioStore";
import { useSettingsStore } from "../../stores/settingsStore";
import {
  keyCodeToDisplay,
  getKeyMomentumConflict,
  buildShortcutsList,
  type MomentumModifierType,
} from "../../utils/keyMapping";
import { WarningTooltip } from "../common/WarningTooltip";

interface KeyGridProps {
  selectedKeys: Set<string>;
  onKeySelect: (keyCode: string, event: React.MouseEvent) => void;
  onSelectAll: () => void;
  matchingKeys?: Set<string> | null;
}

/** Extract only the set of playing sound IDs from playingTracks.
 *  Uses shallow comparison to avoid re-renders on position-only changes. */
export function usePlayingSoundIds(): Set<string> {
  const prevRef = useRef<Set<string>>(new Set());
  return useAudioStore((state) => {
    const next = new Set<string>();
    for (const pt of state.playingTracks.values()) {
      next.add(pt.soundId);
    }
    // Return previous reference if contents are identical (prevents re-render)
    const prev = prevRef.current;
    if (next.size === prev.size && (() => { for (const id of next) { if (!prev.has(id)) return false; } return true; })()) {
      return prev;
    }
    prevRef.current = next;
    return next;
  });
}

export function KeyGrid({ selectedKeys, onKeySelect, onSelectAll, matchingKeys }: KeyGridProps) {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const playingSoundIds = usePlayingSoundIds();
  const lastKeyPressed = useAudioStore((s) => s.lastKeyPressed);
  const config = useSettingsStore((s) => s.config);

  if (!currentProfile) return null;

  const { keyBindings, sounds } = currentProfile;

  // Build shortcuts array for conflict detection
  const shortcuts = buildShortcutsList(config);

  return (
    <div className="space-y-2">
      {keyBindings.length === 0 ? (
        <p className="text-text-muted text-xs italic">
          No keys assigned. Use "Add Sound" to create key bindings.
        </p>
      ) : matchingKeys != null && matchingKeys.size === 0 ? (
        <p className="text-text-muted text-xs italic">
          No matching keys
        </p>
      ) : (
        <div
          className="flex flex-wrap gap-2 outline-none focus:outline-none"
          tabIndex={0}
          onKeyDown={(e) => {
            if ((e.ctrlKey || e.metaKey) && e.key === "a") {
              const tag = (document.activeElement as HTMLElement)?.tagName;
              if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
              e.preventDefault();
              onSelectAll();
            }
          }}
        >
          {keyBindings.map((kb) => {
            const firstSound = sounds.find((s) => kb.soundIds.includes(s.id));
            const displayName = kb.name || firstSound?.name || "No sound";
            const soundCount = kb.soundIds.length;
            const isSelected = selectedKeys.has(kb.keyCode);
            const isPlaying = kb.soundIds.some((id) => playingSoundIds.has(id));
            const isJustPressed = lastKeyPressed === kb.keyCode;
            const isFiltered = matchingKeys != null && !matchingKeys.has(kb.keyCode);

            // Check for momentum conflict
            const conflict = getKeyMomentumConflict(
              kb.keyCode,
              config.momentumModifier as MomentumModifierType,
              shortcuts
            );

            return (
              <button
                key={kb.keyCode}
                onClick={(e) => onKeySelect(kb.keyCode, e)}
                title={keyCodeToDisplay(kb.keyCode)}
                className={`
                  relative px-3 py-2 rounded border text-left min-w-[100px] max-w-[180px] transition-all
                  ${isSelected
                    ? "border-accent-primary bg-accent-primary/10"
                    : "border-border-color bg-bg-secondary hover:border-border-focus"
                  }
                  ${isPlaying ? "ring-1 ring-accent-success" : ""}
                  ${isJustPressed ? "scale-95" : ""}
                  ${isFiltered ? "opacity-30 pointer-events-none" : ""}
                `}
              >
                {/* Warning icon for momentum conflict */}
                {conflict && (
                  <div className="absolute top-1 right-1">
                    <WarningTooltip
                      message={`${config.momentumModifier}+${keyCodeToDisplay(kb.keyCode)} is used for "${conflict.shortcutName}". Change the shortcut or reassign this key.`}
                    />
                  </div>
                )}
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className="text-accent-primary font-mono text-xs font-bold bg-bg-tertiary px-1.5 py-0.5 rounded truncate max-w-full"
                    title={keyCodeToDisplay(kb.keyCode)}
                  >
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
