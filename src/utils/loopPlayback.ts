import type { LoopMode, Sound, TrackId } from "../types";

export type LoopMomentumBehavior = "config_only" | "prefer_sound_momentum";

export interface LoopSelectionResult {
  sound: Sound | null;
  nextIndex: number;
}

export interface ActiveLoopContext {
  keyCode: string;
  trackId: TrackId;
  loopMode: LoopMode;
  momentumBehavior: LoopMomentumBehavior;
  persistSingleMomentum: boolean;
}

export function selectLoopSound(
  soundIds: string[],
  soundMap: Map<string, Sound>,
  loopMode: LoopMode,
  currentIndex: number
): LoopSelectionResult {
  const available = soundIds
    .map((id) => soundMap.get(id))
    .filter((sound): sound is Sound => sound !== undefined);

  if (available.length === 0) {
    return { sound: null, nextIndex: 0 };
  }

  currentIndex = Number.isFinite(currentIndex) ? Math.max(0, Math.trunc(currentIndex)) : 0;

  switch (loopMode) {
    case "off":
    case "random": {
      if (available.length === 1) {
        return { sound: available[0], nextIndex: 0 };
      }

      let idx: number;
      do {
        idx = Math.floor(Math.random() * available.length);
      } while (idx === currentIndex && available.length > 1);

      return { sound: available[idx], nextIndex: idx };
    }
    case "single":
      return {
        sound: available[Math.min(currentIndex, available.length - 1)],
        nextIndex: Math.min(currentIndex, available.length - 1),
      };
    case "sequential": {
      const idx = currentIndex % available.length;
      return { sound: available[idx], nextIndex: idx + 1 };
    }
  }
}

export function shouldAutoReplayLoopMode(loopMode: LoopMode): boolean {
  return loopMode !== "off";
}

export function getLoopReplayStartPosition(
  sound: Sound,
  loopMode: LoopMode,
  autoMomentum: boolean,
  context: ActiveLoopContext
): number {
  if (loopMode === "single" && context.persistSingleMomentum) {
    return sound.momentum;
  }

  if (context.momentumBehavior === "prefer_sound_momentum") {
    return autoMomentum || sound.momentum > 0 ? sound.momentum : 0;
  }

  return autoMomentum ? sound.momentum : 0;
}
