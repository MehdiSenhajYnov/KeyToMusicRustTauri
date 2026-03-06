import { create } from "zustand";
import type { WaveformData } from "../types";
import * as commands from "../utils/tauriCommands";

export interface DiscoverySuggestion {
  videoId: string;
  title: string;
  channel: string;
  duration: number;
  url: string;
  occurrenceCount: number;
  sourceSeedNames: string[];
  sourceSeedIds: string[];
}

export interface EnrichedSuggestion extends DiscoverySuggestion {
  // Pre-download
  predownloadStatus: "idle" | "downloading" | "ready" | "error";
  cachedPath: string | null;
  downloadProgress: number;
  downloadId: string | null;

  // Waveform (after pre-download)
  waveform: WaveformData | null;

  // Auto-assigned (editable before add)
  suggestedKey: string;
  suggestedTrackId: string;
  suggestedMomentum: number;

  // Preview
  isPreviewPlaying: boolean;
}

export interface DiscoveryProgress {
  current: number;
  total: number;
  seedName: string;
}

interface PredownloadData {
  cachedPath?: string;
  waveform?: WaveformData | null;
  downloadId?: string;
  duration?: number;
}

type Enricher = (s: DiscoverySuggestion, index: number) => {
  suggestedKey: string;
  suggestedTrackId: string;
};

interface DiscoveryState {
  allSuggestions: DiscoverySuggestion[];
  visibleSuggestions: EnrichedSuggestion[];
  revealedCount: number;

  currentIndex: number;
  // Highest index the user has navigated to during streaming.
  // Items at indices 0..visitedIndex are "locked" and won't be moved/replaced by streaming updates.
  visitedIndex: number;

  isGenerating: boolean;
  isBackgroundFetching: boolean;
  isResolvingLocals: boolean;
  resolvingCount: number;
  progress: DiscoveryProgress | null;
  error: string | null;

  // Separate download progress tracking — updates don't rebuild the entire visibleSuggestions array
  downloadProgresses: Record<string, number>;

  // Pre-downloaded pool items (not yet visible) — applied when items become visible
  poolPredownloads: Record<string, PoolPredownloadData>;

  // Pre-downloaded refresh items — always mirrors visited+1/+2 for instant refresh
  refreshPredownloads: Record<string, PoolPredownloadData>;

  // Actions
  setSuggestions: (suggestions: DiscoverySuggestion[], enricher: Enricher) => void;
  mergeStreamingSuggestions: (suggestions: DiscoverySuggestion[], enricher: Enricher) => void;
  removeSuggestion: (videoId: string) => void;
  setGenerating: (generating: boolean) => void;
  setBackgroundFetching: (fetching: boolean) => void;
  setResolvingLocals: (resolving: boolean, count?: number) => void;
  setProgress: (progress: DiscoveryProgress | null) => void;
  setError: (error: string | null) => void;
  clear: () => void;

  // Carousel
  goToNext: () => void;
  goToPrev: () => void;
  goToIndex: (index: number) => void;

  // Pagination
  revealMore: (enricher: Enricher) => void;

  // Infinite pool
  appendToPool: (suggestions: DiscoverySuggestion[]) => void;
  setPoolPredownload: (videoId: string, data: PoolPredownloadData) => void;
  setRefreshPredownloads: (data: Record<string, PoolPredownloadData>) => void;
  restoreFromCache: (
    suggestions: DiscoverySuggestion[],
    cursorIndex: number,
    revealedCount: number,
    visitedIndex: number,
    enricher: Enricher
  ) => void;

  // Pre-download tracking
  setPredownloadStatus: (
    videoId: string,
    status: EnrichedSuggestion["predownloadStatus"],
    data?: PredownloadData
  ) => void;
  updateDownloadProgress: (videoId: string, progress: number) => void;

  // Edit assignment before add
  updateSuggestionAssignment: (
    videoId: string,
    updates: Partial<{
      suggestedKey: string;
      suggestedTrackId: string;
      suggestedMomentum: number;
    }>
  ) => void;

  // Preview
  previewVolume: number;
  setPreviewVolume: (volume: number) => void;
  setPreviewPlaying: (videoId: string, playing: boolean) => void;
}

const INITIAL_REVEAL = 10;
const REVEAL_INCREMENT = 10;

export type PoolPredownloadData = { cachedPath: string; waveform: WaveformData | null; duration: number; suggestedMomentum: number };

function enrichSuggestion(
  s: DiscoverySuggestion,
  assignment: { suggestedKey: string; suggestedTrackId: string },
  poolData?: PoolPredownloadData
): EnrichedSuggestion {
  if (poolData) {
    return {
      ...s,
      predownloadStatus: "ready",
      cachedPath: poolData.cachedPath,
      downloadProgress: 100,
      downloadId: null,
      waveform: poolData.waveform,
      duration: poolData.duration,
      suggestedKey: assignment.suggestedKey,
      suggestedTrackId: assignment.suggestedTrackId,
      suggestedMomentum: poolData.suggestedMomentum,
      isPreviewPlaying: false,
    };
  }
  return {
    ...s,
    predownloadStatus: "idle",
    cachedPath: null,
    downloadProgress: 0,
    downloadId: null,
    waveform: null,
    suggestedKey: assignment.suggestedKey,
    suggestedTrackId: assignment.suggestedTrackId,
    suggestedMomentum: 0,
    isPreviewPlaying: false,
  };
}

export const useDiscoveryStore = create<DiscoveryState>((set, get) => ({
  allSuggestions: [],
  visibleSuggestions: [],
  revealedCount: INITIAL_REVEAL,
  currentIndex: 0,
  visitedIndex: -1,
  isGenerating: false,
  isBackgroundFetching: false,
  isResolvingLocals: false,
  resolvingCount: 0,
  progress: null,
  error: null,
  previewVolume: parseFloat(localStorage.getItem("discoveryPreviewVolume") ?? "0.5") || 0.5,
  downloadProgresses: {},
  poolPredownloads: {},
  refreshPredownloads: {},

  // Full replacement — used for loading from cache (no streaming)
  setSuggestions: (suggestions, enricher) => {
    const pool = get().poolPredownloads;
    const refreshPool = get().refreshPredownloads;
    const count = Math.min(INITIAL_REVEAL, suggestions.length);
    const usedPoolIds = new Set<string>();
    const visible = suggestions.slice(0, count).map((s, i) => {
      const pd = refreshPool[s.videoId] || pool[s.videoId];
      if (pd && pool[s.videoId]) usedPoolIds.add(s.videoId);
      return enrichSuggestion(s, enricher(s, i), pd);
    });
    // Remove consumed entries from pool predownloads
    const remaining = { ...pool };
    for (const id of usedPoolIds) delete remaining[id];
    set({
      allSuggestions: suggestions,
      visibleSuggestions: visible,
      revealedCount: count,
      currentIndex: 0,
      visitedIndex: visible.length > 0 ? 0 : -1,
      error: null,
      poolPredownloads: remaining,
      refreshPredownloads: {},
    });
  },

  // Incremental merge — used during streaming. Locks items the user has already seen.
  mergeStreamingSuggestions: (newSuggestions, enricher) => {
    const state = get();
    const { allSuggestions, visibleSuggestions, visitedIndex, revealedCount, currentIndex } = state;

    // Items at indices 0..visitedIndex are locked (user already saw them)
    const lockedCount = Math.max(0, visitedIndex + 1);
    const lockedAll = allSuggestions.slice(0, lockedCount);
    const lockedIds = new Set(lockedAll.map(s => s.videoId));

    // Preserve enrichment data (predownload, waveform, key assignment, preview) for existing items
    const enrichedMap = new Map(visibleSuggestions.map(s => [s.videoId, s]));

    // New suggestions excluding locked ones — backend already sorts by occurrence count
    const newFiltered = newSuggestions.filter(s => !lockedIds.has(s.videoId));

    // Merge: locked items stay in place, new items fill positions after
    const updatedAll = [...lockedAll, ...newFiltered];

    // Reveal up to INITIAL_REVEAL
    const newRevealedCount = Math.min(
      Math.max(revealedCount, INITIAL_REVEAL),
      updatedAll.length
    );

    // Build visible list, preserving enrichment for known items
    const updatedVisible = updatedAll.slice(0, newRevealedCount).map((s, i) => {
      const existing = enrichedMap.get(s.videoId);
      if (existing) {
        // Keep all enrichment, just update occurrence metadata
        return {
          ...existing,
          occurrenceCount: s.occurrenceCount,
          sourceSeedNames: s.sourceSeedNames,
          sourceSeedIds: s.sourceSeedIds,
        };
      }
      return enrichSuggestion(s, enricher(s, i));
    });

    // Mark current position as visited if suggestions are visible
    const newVisitedIndex = updatedVisible.length > 0
      ? Math.max(visitedIndex, Math.min(currentIndex, updatedVisible.length - 1))
      : visitedIndex;

    set({
      allSuggestions: updatedAll,
      visibleSuggestions: updatedVisible,
      revealedCount: newRevealedCount,
      visitedIndex: newVisitedIndex,
      error: null,
    });
  },

  removeSuggestion: (videoId) =>
    set((state) => {
      const removedIdx = state.visibleSuggestions.findIndex(s => s.videoId === videoId);
      const allSuggestions = state.allSuggestions.filter(
        (s) => s.videoId !== videoId
      );
      const visibleSuggestions = state.visibleSuggestions.filter(
        (s) => s.videoId !== videoId
      );
      const currentIndex = Math.min(
        state.currentIndex,
        Math.max(0, visibleSuggestions.length - 1)
      );
      // Adjust visitedIndex if removal was within the visited range
      const visitedIndex = removedIdx !== -1 && removedIdx <= state.visitedIndex
        ? Math.max(-1, state.visitedIndex - 1)
        : state.visitedIndex;
      return { allSuggestions, visibleSuggestions, currentIndex, visitedIndex };
    }),

  setGenerating: (generating) =>
    set({ isGenerating: generating, ...(generating ? { error: null } : {}) }),

  setBackgroundFetching: (fetching) => set({ isBackgroundFetching: fetching }),

  setResolvingLocals: (resolving, count) => set({
    isResolvingLocals: resolving,
    ...(count !== undefined ? { resolvingCount: count } : {}),
  }),

  setProgress: (progress) => set({ progress }),

  setError: (error) => set({ error, isGenerating: false, progress: null }),

  clear: () =>
    set({
      allSuggestions: [],
      visibleSuggestions: [],
      revealedCount: INITIAL_REVEAL,
      currentIndex: 0,
      visitedIndex: -1,
      isGenerating: false,
      isBackgroundFetching: false,
      isResolvingLocals: false,
      resolvingCount: 0,
      progress: null,
      error: null,
      downloadProgresses: {},
      poolPredownloads: {},
      refreshPredownloads: {},
    }),

  goToNext: () => {
    const state = get();
    if (state.currentIndex < state.visibleSuggestions.length - 1) {
      const newIndex = state.currentIndex + 1;
      set({
        currentIndex: newIndex,
        visitedIndex: Math.max(state.visitedIndex, newIndex),
      });
    }
  },

  goToPrev: () => {
    const state = get();
    if (state.currentIndex > 0) {
      set({ currentIndex: state.currentIndex - 1 });
    }
  },

  goToIndex: (index) => {
    const state = get();
    const clamped = Math.max(
      0,
      Math.min(index, state.visibleSuggestions.length - 1)
    );
    set({
      currentIndex: clamped,
      visitedIndex: Math.max(state.visitedIndex, clamped),
    });
  },

  revealMore: (enricher) => {
    const state = get();
    const newCount = Math.min(
      state.revealedCount + REVEAL_INCREMENT,
      state.allSuggestions.length
    );
    if (newCount <= state.revealedCount) return;

    const pool = state.poolPredownloads;
    const usedIds = new Set<string>();
    const newEntries = state.allSuggestions
      .slice(state.revealedCount, newCount)
      .map((s, i) => {
        const pd = pool[s.videoId];
        if (pd) usedIds.add(s.videoId);
        return enrichSuggestion(s, enricher(s, state.revealedCount + i), pd);
      });

    const remaining = usedIds.size > 0
      ? Object.fromEntries(Object.entries(pool).filter(([id]) => !usedIds.has(id)))
      : pool;

    set({
      visibleSuggestions: [...state.visibleSuggestions, ...newEntries],
      revealedCount: newCount,
      poolPredownloads: remaining,
    });
  },

  appendToPool: (suggestions) => {
    const state = get();
    const existingIds = new Set(state.allSuggestions.map(s => s.videoId));
    const newItems = suggestions.filter(s => !existingIds.has(s.videoId));
    if (newItems.length === 0) return;
    set({ allSuggestions: [...state.allSuggestions, ...newItems] });
  },

  setPoolPredownload: (videoId, data) =>
    set((state) => ({
      poolPredownloads: { ...state.poolPredownloads, [videoId]: data },
    })),

  setRefreshPredownloads: (data) => set({ refreshPredownloads: data }),

  restoreFromCache: (suggestions, cursorIndex, revealedCount, visitedIndex, enricher) => {
    // Handle old caches where revealedCount=0 (pre-infinite-pool)
    const effectiveRevealed = revealedCount > 0
      ? Math.min(revealedCount, suggestions.length)
      : Math.min(INITIAL_REVEAL, suggestions.length);
    const effectiveCursor = Math.min(cursorIndex, Math.max(0, effectiveRevealed - 1));

    const pool = get().poolPredownloads;
    const refreshPool = get().refreshPredownloads;
    const usedPoolIds = new Set<string>();
    const visible = suggestions.slice(0, effectiveRevealed).map((s, i) => {
      const pd = refreshPool[s.videoId] || pool[s.videoId];
      if (pd && pool[s.videoId]) usedPoolIds.add(s.videoId);
      return enrichSuggestion(s, enricher(s, i), pd);
    });

    const remaining = usedPoolIds.size > 0
      ? Object.fromEntries(Object.entries(pool).filter(([id]) => !usedPoolIds.has(id)))
      : pool;

    set({
      allSuggestions: suggestions,
      visibleSuggestions: visible,
      revealedCount: effectiveRevealed,
      currentIndex: effectiveCursor,
      visitedIndex,
      error: null,
      poolPredownloads: remaining,
      refreshPredownloads: {},
    });
  },

  setPredownloadStatus: (videoId, status, data) =>
    set((state) => {
      const idx = state.visibleSuggestions.findIndex(s => s.videoId === videoId);
      if (idx === -1) return state;
      const updated = [...state.visibleSuggestions];
      updated[idx] = {
        ...updated[idx],
        predownloadStatus: status,
        ...(data?.cachedPath !== undefined ? { cachedPath: data.cachedPath } : {}),
        ...(data?.waveform !== undefined ? { waveform: data.waveform } : {}),
        ...(data?.downloadId !== undefined ? { downloadId: data.downloadId } : {}),
        ...(data?.duration !== undefined ? { duration: data.duration } : {}),
      };
      return { visibleSuggestions: updated };
    }),

  updateDownloadProgress: (() => {
    let lastUpdate = 0;
    return (videoId: string, progress: number) => {
      const now = Date.now();
      // Throttle to max 10 updates/sec (100ms) to reduce GC pressure
      if (now - lastUpdate < 100 && progress < 100) return;
      lastUpdate = now;
      set((state) => ({
        downloadProgresses: { ...state.downloadProgresses, [videoId]: progress },
      }));
    };
  })(),

  updateSuggestionAssignment: (videoId, updates) =>
    set((state) => ({
      visibleSuggestions: state.visibleSuggestions.map((s) =>
        s.videoId === videoId ? { ...s, ...updates } : s
      ),
    })),

  setPreviewVolume: (volume) => {
    set({ previewVolume: volume });
    localStorage.setItem("discoveryPreviewVolume", String(volume));
    // Update volume in real-time if a preview is playing
    const playing = get().visibleSuggestions.find(s => s.isPreviewPlaying);
    if (playing) {
      commands.setSoundVolume("__preview__", playing.videoId, volume).catch(() => {});
    }
  },

  setPreviewPlaying: (videoId, playing) =>
    set((state) => ({
      visibleSuggestions: state.visibleSuggestions.map((s) =>
        s.videoId === videoId
          ? { ...s, isPreviewPlaying: playing }
          : // Stop any other preview when starting a new one
            playing
            ? { ...s, isPreviewPlaying: false }
            : s
      ),
    })),
}));
