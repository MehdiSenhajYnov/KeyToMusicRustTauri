import { create } from "zustand";
import type { WaveformData } from "../types";

export interface DiscoverySuggestion {
  videoId: string;
  title: string;
  channel: string;
  duration: number;
  url: string;
  occurrenceCount: number;
  sourceSeedNames: string[];
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

interface DiscoveryState {
  allSuggestions: DiscoverySuggestion[];
  visibleSuggestions: EnrichedSuggestion[];
  revealedCount: number;

  currentIndex: number;

  isGenerating: boolean;
  progress: DiscoveryProgress | null;
  error: string | null;

  // Actions
  setSuggestions: (
    suggestions: DiscoverySuggestion[],
    enricher: (s: DiscoverySuggestion, index: number) => {
      suggestedKey: string;
      suggestedTrackId: string;
    }
  ) => void;
  removeSuggestion: (videoId: string) => void;
  setGenerating: (generating: boolean) => void;
  setProgress: (progress: DiscoveryProgress | null) => void;
  setError: (error: string | null) => void;
  clear: () => void;

  // Carousel
  goToNext: () => void;
  goToPrev: () => void;
  goToIndex: (index: number) => void;

  // Pagination
  revealMore: (
    enricher: (s: DiscoverySuggestion, index: number) => {
      suggestedKey: string;
      suggestedTrackId: string;
    }
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
  setPreviewPlaying: (videoId: string, playing: boolean) => void;
}

const INITIAL_REVEAL = 10;
const REVEAL_INCREMENT = 5;

function enrichSuggestion(
  s: DiscoverySuggestion,
  assignment: { suggestedKey: string; suggestedTrackId: string }
): EnrichedSuggestion {
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
  isGenerating: false,
  progress: null,
  error: null,

  setSuggestions: (suggestions, enricher) => {
    const count = Math.min(INITIAL_REVEAL, suggestions.length);
    const visible = suggestions.slice(0, count).map((s, i) =>
      enrichSuggestion(s, enricher(s, i))
    );
    set({
      allSuggestions: suggestions,
      visibleSuggestions: visible,
      revealedCount: count,
      currentIndex: 0,
      error: null,
    });
  },

  removeSuggestion: (videoId) =>
    set((state) => {
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
      return { allSuggestions, visibleSuggestions, currentIndex };
    }),

  setGenerating: (generating) =>
    set({ isGenerating: generating, ...(generating ? { error: null } : {}) }),

  setProgress: (progress) => set({ progress }),

  setError: (error) => set({ error, isGenerating: false, progress: null }),

  clear: () =>
    set({
      allSuggestions: [],
      visibleSuggestions: [],
      revealedCount: INITIAL_REVEAL,
      currentIndex: 0,
      isGenerating: false,
      progress: null,
      error: null,
    }),

  goToNext: () => {
    const state = get();
    if (state.currentIndex < state.visibleSuggestions.length - 1) {
      set({ currentIndex: state.currentIndex + 1 });
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
    set({ currentIndex: clamped });
  },

  revealMore: (enricher) => {
    const state = get();
    const newCount = Math.min(
      state.revealedCount + REVEAL_INCREMENT,
      state.allSuggestions.length
    );
    if (newCount <= state.revealedCount) return;

    const newEntries = state.allSuggestions
      .slice(state.revealedCount, newCount)
      .map((s, i) =>
        enrichSuggestion(s, enricher(s, state.revealedCount + i))
      );

    set({
      visibleSuggestions: [...state.visibleSuggestions, ...newEntries],
      revealedCount: newCount,
    });
  },

  setPredownloadStatus: (videoId, status, data) =>
    set((state) => ({
      visibleSuggestions: state.visibleSuggestions.map((s) =>
        s.videoId === videoId
          ? {
              ...s,
              predownloadStatus: status,
              ...(data?.cachedPath !== undefined
                ? { cachedPath: data.cachedPath }
                : {}),
              ...(data?.waveform !== undefined
                ? { waveform: data.waveform }
                : {}),
              ...(data?.downloadId !== undefined
                ? { downloadId: data.downloadId }
                : {}),
              ...(data?.duration !== undefined
                ? { duration: data.duration }
                : {}),
            }
          : s
      ),
    })),

  updateDownloadProgress: (videoId, progress) =>
    set((state) => ({
      visibleSuggestions: state.visibleSuggestions.map((s) =>
        s.videoId === videoId ? { ...s, downloadProgress: progress } : s
      ),
    })),

  updateSuggestionAssignment: (videoId, updates) =>
    set((state) => ({
      visibleSuggestions: state.visibleSuggestions.map((s) =>
        s.videoId === videoId ? { ...s, ...updates } : s
      ),
    })),

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
