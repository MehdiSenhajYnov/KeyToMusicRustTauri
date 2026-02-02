import { create } from "zustand";
import type { WaveformData } from "../types";

const MAX_ENTRIES = 50;

interface WaveformState {
  waveforms: Map<string, WaveformData>;
  accessOrder: string[]; // Most recently accessed at the end
  loading: boolean;

  setBatch: (record: Record<string, WaveformData>) => void;
  setOne: (path: string, data: WaveformData) => void;
  get: (path: string) => WaveformData | undefined;
  clear: () => void;
  setLoading: (loading: boolean) => void;
}

/** Move a key to the end of accessOrder (most recent), or append if new. */
function touchKey(order: string[], key: string): string[] {
  const idx = order.indexOf(key);
  if (idx === order.length - 1) return order; // already at end
  const next = idx >= 0 ? [...order.slice(0, idx), ...order.slice(idx + 1)] : [...order];
  next.push(key);
  return next;
}

/** Evict least recently used entries until size <= MAX_ENTRIES. */
function evict(
  map: Map<string, WaveformData>,
  order: string[],
): { waveforms: Map<string, WaveformData>; accessOrder: string[] } {
  if (map.size <= MAX_ENTRIES) return { waveforms: map, accessOrder: order };
  const excess = map.size - MAX_ENTRIES;
  const toRemove = order.slice(0, excess);
  const newMap = new Map(map);
  for (const key of toRemove) {
    newMap.delete(key);
  }
  return { waveforms: newMap, accessOrder: order.slice(excess) };
}

export const useWaveformStore = create<WaveformState>((set, getState) => ({
  waveforms: new Map(),
  accessOrder: [],
  loading: false,

  setBatch: (record) => {
    set((state) => {
      const newMap = new Map(state.waveforms);
      let order = [...state.accessOrder];
      for (const [path, data] of Object.entries(record)) {
        newMap.set(path, data);
        order = touchKey(order, path);
      }
      const evicted = evict(newMap, order);
      return { ...evicted, loading: false };
    });
  },

  setOne: (path, data) => {
    set((state) => {
      const newMap = new Map(state.waveforms);
      newMap.set(path, data);
      const order = touchKey([...state.accessOrder], path);
      const evicted = evict(newMap, order);
      return evicted;
    });
  },

  get: (path) => {
    // Non-reactive read — does NOT call set() to avoid triggering re-renders.
    // LRU order is updated on setOne/setBatch (write paths) which is sufficient.
    return getState().waveforms.get(path);
  },

  clear: () => set({ waveforms: new Map(), accessOrder: [], loading: false }),

  setLoading: (loading) => set({ loading }),
}));
