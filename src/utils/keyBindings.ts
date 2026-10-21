import type { KeyBinding } from "../types";

/** A key may target several tracks, but each key/track pair has one sound group. */
export function mergeKeyBindings(bindings: KeyBinding[]): KeyBinding[] {
  const merged = new Map<string, KeyBinding>();
  for (const binding of bindings) {
    const key = JSON.stringify([binding.keyCode, binding.trackId]);
    const existing = merged.get(key);
    merged.set(key, existing ? {
      ...existing, currentIndex: 0,
      soundIds: [...new Set([...existing.soundIds, ...binding.soundIds])],
    } : binding);
  }
  return [...merged.values()];
}

export function moveKeyBinding(bindings: KeyBinding[], keyCode: string, trackId: string, newKey: string, soundId?: string): KeyBinding[] {
  const source = bindings.find((b) => b.keyCode === keyCode && b.trackId === trackId);
  if (!source || newKey === keyCode || (soundId && !source.soundIds.includes(soundId))) return bindings;
  const movedIds = soundId ? [soundId] : source.soundIds;
  const remaining = source.soundIds.filter((id) => !movedIds.includes(id));
  return mergeKeyBindings([
    ...bindings.flatMap((binding) => binding !== source ? [binding] : remaining.length ? [{ ...binding, soundIds: remaining, currentIndex: 0 }] : []),
    { ...source, keyCode: newKey, soundIds: movedIds, currentIndex: 0 },
  ]);
}
