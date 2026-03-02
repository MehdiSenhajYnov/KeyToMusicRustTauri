import { useRef, useMemo } from "react";
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
import { MOOD_DISPLAY, MOOD_COLORS } from "../../utils/moodHelpers";
import type { MoodCategory } from "../../types";

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

  // Group bindings by keyCode to avoid duplicate React keys (multi-track support)
  const groupedKeys = useMemo(() => {
    const map = new Map<string, { keyCode: string; allSoundIds: string[]; trackCount: number; name?: string; mood?: MoodCategory }>();
    for (const kb of keyBindings) {
      const existing = map.get(kb.keyCode);
      if (existing) {
        existing.allSoundIds.push(...kb.soundIds);
        existing.trackCount++;
        if (!existing.name && kb.name) existing.name = kb.name;
        if (!existing.mood && kb.mood) existing.mood = kb.mood;
      } else {
        map.set(kb.keyCode, {
          keyCode: kb.keyCode,
          allSoundIds: [...kb.soundIds],
          trackCount: 1,
          name: kb.name,
          mood: kb.mood,
        });
      }
    }
    return [...map.values()];
  }, [keyBindings]);

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
          {groupedKeys.map((group) => {
            const firstSound = sounds.find((s) => group.allSoundIds.includes(s.id));
            const displayName = group.name || firstSound?.name || "No sound";
            const soundCount = group.allSoundIds.length;
            const isSelected = selectedKeys.has(group.keyCode);
            const isPlaying = group.allSoundIds.some((id) => playingSoundIds.has(id));
            const isJustPressed = lastKeyPressed === group.keyCode;
            const isFiltered = matchingKeys != null && !matchingKeys.has(group.keyCode);

            // Check for momentum conflict
            const conflict = getKeyMomentumConflict(
              group.keyCode,
              config.momentumModifier as MomentumModifierType,
              shortcuts
            );

            return (
              <button
                key={group.keyCode}
                onClick={(e) => onKeySelect(group.keyCode, e)}
                title={keyCodeToDisplay(group.keyCode)}
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
                      message={`${config.momentumModifier}+${keyCodeToDisplay(group.keyCode)} is used for "${conflict.shortcutName}". Change the shortcut or reassign this key.`}
                    />
                  </div>
                )}
                <div className="flex items-center gap-2 min-w-0">
                  <span
                    className="text-accent-primary font-mono text-xs font-bold bg-bg-tertiary px-1.5 py-0.5 rounded truncate max-w-full"
                    title={keyCodeToDisplay(group.keyCode)}
                  >
                    {keyCodeToDisplay(group.keyCode)}
                  </span>
                  {isPlaying && (
                    <span className="w-1.5 h-1.5 rounded-full bg-accent-success animate-pulse" />
                  )}
                </div>
                <p className="text-text-primary text-xs mt-1 truncate">
                  {displayName}
                </p>
                <div className="flex items-center gap-1 flex-wrap">
                  <span className="text-text-muted text-xs">
                    {soundCount} {soundCount > 1 ? "sons" : "son"}
                  </span>
                  {group.trackCount > 1 && (
                    <span className="text-text-muted text-[10px] bg-bg-tertiary px-1 rounded">
                      {group.trackCount}T
                    </span>
                  )}
                  {group.mood && (
                    <span className={`text-[9px] px-1 py-px rounded-full ${MOOD_COLORS[group.mood].bg} ${MOOD_COLORS[group.mood].text}`}>
                      {MOOD_DISPLAY[group.mood]}
                    </span>
                  )}
                </div>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
