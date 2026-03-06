import type { Sound, Track, KeyBinding } from "../types";

export function getSoundFilePath(sound: Sound): string {
  if (sound.source.type === "local") return sound.source.path;
  return sound.source.cachedPath;
}

/**
 * Find the track with the fewest bindings assigned.
 * Returns the first track if all are equal.
 */
export function findLeastUsedTrack(
  tracks: Track[],
  bindings: KeyBinding[]
): string {
  if (tracks.length === 0) return "";

  const counts = new Map<string, number>();
  for (const t of tracks) counts.set(t.id, 0);
  for (const b of bindings) {
    counts.set(b.trackId, (counts.get(b.trackId) ?? 0) + 1);
  }

  let minId = tracks[0].id;
  let minCount = counts.get(minId) ?? 0;
  for (const t of tracks) {
    const c = counts.get(t.id) ?? 0;
    if (c < minCount) {
      minCount = c;
      minId = t.id;
    }
  }
  return minId;
}
