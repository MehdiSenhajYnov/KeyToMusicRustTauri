import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  useDiscoveryStore,
  type DiscoverySuggestion,
  type EnrichedSuggestion,
} from "../../stores/discoveryStore";
import { useWheelSlider } from "../../hooks/useWheelSlider";
import { useProfileStore } from "../../stores/profileStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";
import { useAudioStore } from "../../stores/audioStore";
import { formatDuration } from "../../utils/fileHelpers";
import {
  keyCodeToDisplay,
  getKeyCode,
  recordKeyLayout,
  buildComboFromPressedKeys,
  checkShortcutConflicts,
} from "../../utils/keyMapping";
import { WaveformDisplay } from "../common/WaveformDisplay";
import * as commands from "../../utils/tauriCommands";
import type { SoundSource } from "../../types";
import { analyzeProfile, computeAutoAssign } from "../../utils/profileAnalysis";

export function DiscoveryPanel() {
  const profile = useProfileStore((s) => s.currentProfile);
  const { addSound, addKeyBinding, updateKeyBinding, saveCurrentProfile } = useProfileStore.getState();
  const config = useSettingsStore((s) => s.config);
  const addToast = useToastStore((s) => s.addToast);

  const allSuggestions = useDiscoveryStore((s) => s.allSuggestions);
  const visibleSuggestions = useDiscoveryStore((s) => s.visibleSuggestions);
  const currentIndex = useDiscoveryStore((s) => s.currentIndex);
  const revealedCount = useDiscoveryStore((s) => s.revealedCount);
  const isGenerating = useDiscoveryStore((s) => s.isGenerating);
  const isResolvingLocals = useDiscoveryStore((s) => s.isResolvingLocals);
  const resolvingCount = useDiscoveryStore((s) => s.resolvingCount);
  const progress = useDiscoveryStore((s) => s.progress);
  const error = useDiscoveryStore((s) => s.error);

  // Read actions via getState() — stable references, never cause re-renders
  const {
    mergeStreamingSuggestions,
    removeSuggestion,
    setGenerating,
    setBackgroundFetching,
    setResolvingLocals,
    setPreviewPlaying,
    updateSuggestionAssignment,
    goToNext,
    goToPrev,
    revealMore,
    appendToPool,
    restoreFromCache,
    clear,
  } = useDiscoveryStore.getState();

  const [isCapturingKey, setIsCapturingKey] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const downloadIdCounter = useRef(0);
  const discoveryGenRef = useRef(0);
  const cursorSaveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const hasDiscoverableSounds =
    profile?.sounds.some((s) =>
      s.source.type === "youtube" || s.source.type === "local"
    ) ?? false;

  // Debounced cursor persistence (500ms)
  const persistCursor = useCallback(() => {
    if (!profile) return;
    if (cursorSaveTimerRef.current) clearTimeout(cursorSaveTimerRef.current);
    cursorSaveTimerRef.current = setTimeout(() => {
      const state = useDiscoveryStore.getState();
      commands.saveDiscoveryCursor(
        profile.id,
        state.currentIndex,
        state.revealedCount,
        state.visitedIndex
      ).catch(() => {});
    }, 500);
  }, [profile]);

  // Background fetch when approaching end of pool
  const triggerBackgroundFetchIfNeeded = useCallback(() => {
    if (!profile) return;
    const state = useDiscoveryStore.getState();
    const THRESHOLD = 15;
    if (
      state.currentIndex >= state.allSuggestions.length - THRESHOLD &&
      !state.isBackgroundFetching &&
      !state.isGenerating
    ) {
      setBackgroundFetching(true);
      const gen = discoveryGenRef.current;
      const poolIds = state.allSuggestions.map(s => s.videoId);
      commands.startDiscovery(profile.id, poolIds, true)
        .then((results) => {
          if (discoveryGenRef.current !== gen) return;
          if (results.length > 0) {
            appendToPool(results);
          }
        })
        .catch(() => {})
        .finally(() => {
          if (discoveryGenRef.current === gen) setBackgroundFetching(false);
        });
    }
  }, [profile, setBackgroundFetching, appendToPool]);

  // Build enricher function for auto-assignment
  const buildEnricher = useCallback(() => {
    if (!profile) return (_s: DiscoverySuggestion, _i: number) => ({ suggestedKey: "", suggestedTrackId: "" });

    const usedKeys = new Set(profile.keyBindings.map((kb) => kb.keyCode));
    const alreadySuggested = new Set<string>();
    const mode = analyzeProfile(profile);

    // Also include keys already suggested in visible suggestions
    const vis = useDiscoveryStore.getState().visibleSuggestions;
    for (const s of vis) {
      if (s.suggestedKey) alreadySuggested.add(s.suggestedKey);
    }

    return (s: DiscoverySuggestion, _i: number) => {
      const assign = computeAutoAssign(s, profile, usedKeys, alreadySuggested);
      // In single-sound mode, mark the key as taken for subsequent suggestions
      if (mode === "single-sound" && assign.suggestedKey) {
        alreadySuggested.add(assign.suggestedKey);
      }
      return assign;
    };
  }, [profile]);

  // Listen for partial discovery results (streaming) — merge on every partial,
  // respecting items the user has already navigated to (visitedIndex watermark).
  // Captures discoveryGenRef at listen-time to discard partials from stale discovery runs.
  useEffect(() => {
    const gen = discoveryGenRef.current;
    const unlisten = listen<DiscoverySuggestion[]>(
      "discovery_partial",
      (event) => {
        if (discoveryGenRef.current !== gen) return; // stale discovery run
        const state = useDiscoveryStore.getState();
        if (state.isGenerating && event.payload.length > 0) {
          mergeStreamingSuggestions(event.payload, buildEnricher());
        }
      }
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, [mergeStreamingSuggestions, buildEnricher]);

  // Listen for local sound resolution progress
  useEffect(() => {
    const unlisten = listen<{ count: number }>(
      "discovery_resolving",
      (event) => {
        setResolvingLocals(true, event.payload.count);
      }
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, [setResolvingLocals]);

  // Load cached suggestions when profile changes, auto-discover if empty
  useEffect(() => {
    if (!profile) {
      clear();
      return;
    }
    // Always clear stale suggestions immediately on profile switch
    clear();
    const gen = ++discoveryGenRef.current;
    const hasSounds = profile.sounds.length > 0;
    commands
      .getDiscoverySuggestions(profile.id)
      .then((cached) => {
        if (discoveryGenRef.current !== gen) return; // stale — user triggered refresh
        if (cached && cached.suggestions.length > 0) {
          restoreFromCache(
            cached.suggestions,
            cached.cursorIndex,
            cached.revealedCount,
            cached.visitedIndex,
            buildEnricher()
          );
          // Check if background fetch needed after restore
          setTimeout(() => triggerBackgroundFetchIfNeeded(), 100);
        } else if (hasSounds) {
          // Auto-trigger discovery when no cached results (YouTube + local seeds)
          setGenerating(true);
          commands.startDiscovery(profile.id)
            .then((results) => {
              if (discoveryGenRef.current !== gen) return;
              setResolvingLocals(false);
              // Final merge — partials already populated incrementally, this ensures consistency
              mergeStreamingSuggestions(results, buildEnricher());
              if (results.length === 0) {
                addToast("No new suggestions found", "info");
              }
            })
            .catch((e) => {
              if (discoveryGenRef.current !== gen) return;
              setResolvingLocals(false);
              const msg = String(e);
              if (!msg.includes("cancelled")) {
                addToast(`Discovery failed: ${msg}`, "error");
              }
            })
            .finally(() => {
              if (discoveryGenRef.current === gen) setGenerating(false);
            });
        } else {
          // No sounds and no cached suggestions — already cleared above
        }
      })
      .catch(() => {});

    return () => {
      // Cancel ongoing discovery on profile change
      commands.cancelDiscovery().catch(() => {});
    };
  }, [profile?.id]);

  const handleGenerate = async () => {
    if (!profile || isGenerating) return;

    const state = useDiscoveryStore.getState();
    // Skip everything currently revealed — user wants a fresh batch
    const unseenStart = state.revealedCount;
    const unseen = state.allSuggestions.slice(unseenStart);

    // If pool has unseen items, discard seen ones and present unseen as fresh batch (1/10)
    if (unseen.length > 0) {
      stopPreview();
      useDiscoveryStore.getState().setSuggestions(unseen, buildEnricher());
      // Persist trimmed pool to cache so restart shows the same fresh state
      const newState = useDiscoveryStore.getState();
      commands.updateDiscoveryPool(
        profile.id,
        unseen,
        0,
        newState.revealedCount,
        -1
      ).catch(() => {});
      triggerBackgroundFetchIfNeeded();
      return;
    }

    // Pool exhausted — trigger foreground generation
    const gen = ++discoveryGenRef.current;
    const currentIds = state.allSuggestions.map(s => s.videoId);
    setGenerating(true);
    // Clear old suggestions so partials from this run can populate fresh
    useDiscoveryStore.setState({
      allSuggestions: [],
      visibleSuggestions: [],
      revealedCount: 10,
      currentIndex: 0,
      visitedIndex: -1,
    });
    try {
      const results = await commands.startDiscovery(profile.id, currentIds);
      if (discoveryGenRef.current !== gen) return; // stale
      mergeStreamingSuggestions(results, buildEnricher());
      if (results.length === 0) {
        addToast("No new suggestions found", "info");
      }
    } catch (e) {
      if (discoveryGenRef.current !== gen) return;
      const msg = String(e);
      if (!msg.includes("cancelled")) {
        addToast(`Discovery failed: ${msg}`, "error");
      }
    } finally {
      if (discoveryGenRef.current === gen) setGenerating(false);
    }
  };

  const handleCancel = () => {
    commands.cancelDiscovery().catch(() => {});
  };

  const handleDismiss = (videoId: string) => {
    if (!profile) return;
    stopPreview();
    removeSuggestion(videoId);
    commands.dismissDiscovery(profile.id, videoId).catch(() => {});
    persistCursor();
  };

  // Preview playback
  const stopPreview = useCallback(async () => {
    try {
      await commands.stopSound("__preview__");
    } catch {
      // ignore
    }
    // Reset all preview states
    const store = useDiscoveryStore.getState();
    for (const s of store.visibleSuggestions) {
      if (s.isPreviewPlaying) {
        setPreviewPlaying(s.videoId, false);
      }
    }
  }, [setPreviewPlaying]);

  const handlePreview = async (s: EnrichedSuggestion) => {
    if (s.isPreviewPlaying) {
      await stopPreview();
      return;
    }
    if (s.predownloadStatus !== "ready" || !s.cachedPath) return;

    await stopPreview();
    setPreviewPlaying(s.videoId, true);
    try {
      const previewVolume = useDiscoveryStore.getState().previewVolume;
      await commands.playSound(
        "__preview__",
        s.videoId,
        s.cachedPath,
        s.suggestedMomentum,
        previewVolume
      );
    } catch (e) {
      console.error("[Discovery] Preview playback failed:", e, {
        videoId: s.videoId,
        cachedPath: s.cachedPath,
        predownloadStatus: s.predownloadStatus,
        momentum: s.suggestedMomentum,
      });
      setPreviewPlaying(s.videoId, false);
      addToast("Preview failed — try again", "error");
    }
  };

  // Listen for sound_ended on __preview__ track — use soundId to avoid race conditions.
  // When stopping A and starting B, the sound_ended for A must NOT reset B's preview state.
  useEffect(() => {
    const unlisten = listen<{ trackId: string; soundId: string }>(
      "sound_ended",
      (event) => {
        if (event.payload.trackId === "__preview__") {
          setPreviewPlaying(event.payload.soundId, false);
        }
      }
    );
    return () => {
      unlisten.then((f) => f());
    };
  }, [setPreviewPlaying]);

  const handleSeekPreview = async (s: EnrichedSuggestion, position: number) => {
    if (!s.cachedPath) return;
    try {
      const previewVolume = useDiscoveryStore.getState().previewVolume;
      await commands.playSound("__preview__", s.videoId, s.cachedPath, position, previewVolume);
    } catch {
      // ignore
    }
  };

  /** Add sound to binding: merge into existing binding or create new one. */
  const addToBinding = (soundId: string, suggestedKey: string, suggestedTrackId: string) => {
    if (!suggestedKey || !profile) return;

    const existingBinding = profile.keyBindings.find(
      (kb) => kb.keyCode === suggestedKey
    );
    if (existingBinding) {
      // Multi-sound: append to existing binding
      updateKeyBinding(suggestedKey, {
        soundIds: [...existingBinding.soundIds, soundId],
      });
    } else {
      // New binding
      addKeyBinding({
        keyCode: suggestedKey,
        trackId: suggestedTrackId,
        soundIds: [soundId],
        loopMode: "off",
        currentIndex: 0,
      });
    }
  };

  const handleAdd = async (s: EnrichedSuggestion) => {
    if (!profile) return;

    await stopPreview();

    if (s.predownloadStatus === "ready" && s.cachedPath) {
      // Instant add — file already downloaded
      const soundId = crypto.randomUUID();
      const source: SoundSource = {
        type: "youtube",
        url: s.url,
        cachedPath: s.cachedPath,
      };

      addSound({
        id: soundId,
        name: s.title,
        source,
        momentum: s.suggestedMomentum,
        volume: 1.0,
        duration: s.duration,
      });

      addToBinding(soundId, s.suggestedKey, s.suggestedTrackId);

      setTimeout(() => saveCurrentProfile(), 100);
      removeSuggestion(s.videoId);
      addToast(`Added: ${s.title}`, "success");
      persistCursor();
    } else {
      // Fallback — download then add
      const downloadId = `disc_${Date.now()}_${downloadIdCounter.current++}`;
      addToast(`Downloading: ${s.title}`, "info");

      try {
        const sound = await commands.addSoundFromYoutube(s.url, downloadId);
        const soundId = crypto.randomUUID();

        addSound({
          id: soundId,
          name: sound.name,
          source: sound.source,
          momentum: s.suggestedMomentum,
          volume: 1.0,
          duration: sound.duration,
        });

        addToBinding(soundId, s.suggestedKey, s.suggestedTrackId);

        setTimeout(() => saveCurrentProfile(), 100);
        removeSuggestion(s.videoId);
        addToast(`Added: ${sound.name}`, "success");
        persistCursor();
      } catch (e) {
        addToast(`Failed to add: ${e}`, "error");
      }
    }
  };

  // Arrow navigation when panel focused
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (isCapturingKey) return;
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      handlePrev();
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      handleNext();
    }
  };

  const handleNext = () => {
    stopPreview();
    const state = useDiscoveryStore.getState();
    // At page boundary — reveal next batch if pool has more
    if (state.currentIndex >= state.visibleSuggestions.length - 1) {
      if (state.allSuggestions.length > state.revealedCount) {
        revealMore(buildEnricher());
      }
    }
    goToNext();
    persistCursor();
    triggerBackgroundFetchIfNeeded();
  };

  const handlePrev = () => {
    stopPreview();
    goToPrev();
    persistCursor();
  };

  if (!profile) return null;

  const current = visibleSuggestions[currentIndex];
  const showing = visibleSuggestions.length;
  // hasNext: more visible items OR more pool items to reveal
  const hasNext = currentIndex < showing - 1 || allSuggestions.length > revealedCount;

  return (
    <div
      ref={panelRef}
      className="border-t border-border-color flex flex-col shrink-0"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      {/* Header: [Discover (1/10) Refresh]  ...  [X] */}
      <div className="px-3 pt-2 pb-1 flex items-center justify-between">
        <div className="flex items-center gap-1">
          <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
            Discover
          </h3>
          {showing > 0 && (
            <span className="text-accent-primary text-[10px]">
              ({currentIndex + 1}/{showing})
            </span>
          )}
          {hasDiscoverableSounds && (
            isGenerating ? (
              <button
                onClick={handleCancel}
                className="px-1.5 py-0.5 text-[10px] rounded bg-accent-error/20 text-accent-error hover:bg-accent-error/30 transition-colors"
              >
                Cancel
              </button>
            ) : (
              <button
                onClick={handleGenerate}
                className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-bg-hover transition-colors"
                title="Refresh suggestions"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h5M20 20v-5h-5M4 9a8 8 0 0114.5-4.5M20 15a8 8 0 01-14.5 4.5" />
                </svg>
              </button>
            )
          )}
        </div>
        {current && (
          <button
            onClick={() => handleDismiss(current.videoId)}
            className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-accent-error hover:bg-accent-error/10 transition-colors"
            title="Dismiss suggestion"
          >
            <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        )}
      </div>

      {/* Content */}
      <div className="px-3 pb-3">
        {/* No sounds at all */}
        {!hasDiscoverableSounds && (
          <p className="text-text-muted text-[10px] italic py-1">
            Add sounds to get recommendations
          </p>
        )}

        {/* Has sounds but no suggestions yet */}
        {hasDiscoverableSounds && showing === 0 && !isGenerating && !error && (
          <p className="text-text-muted text-[10px] italic py-1">
            No suggestions yet
          </p>
        )}

        {/* Resolving local sounds */}
        {isResolvingLocals && (
          <div className="flex items-center gap-2 py-1">
            <div className="w-3 h-3 border-2 border-accent-secondary border-t-transparent rounded-full animate-spin shrink-0" />
            <span className="text-text-muted text-[10px]">
              Identifying {resolvingCount} local sound{resolvingCount !== 1 ? "s" : ""}...
            </span>
          </div>
        )}

        {/* Generating progress */}
        {isGenerating && progress && (
          <div className="space-y-1 py-1">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 border-2 border-accent-primary border-t-transparent rounded-full animate-spin shrink-0" />
              <span className="text-text-muted text-[10px] truncate">
                {progress.current}/{progress.total}: {progress.seedName}
              </span>
            </div>
            <div className="w-full bg-bg-tertiary rounded-full h-1 overflow-hidden">
              <div
                className="bg-accent-primary h-full rounded-full transition-all"
                style={{
                  width: `${(progress.current / progress.total) * 100}%`,
                }}
              />
            </div>
          </div>
        )}

        {/* Error */}
        {error && <p className="text-accent-error text-[10px] py-1">{error}</p>}

        {/* Current suggestion card */}
        {current && <SuggestionCard
          suggestion={current}
          profile={profile}
          config={config}
          onAdd={handleAdd}
          onPreview={handlePreview}
          onSeekPreview={handleSeekPreview}
          onUpdateAssignment={updateSuggestionAssignment}
          onCapturingChange={setIsCapturingKey}
          onPrev={handlePrev}
          onNext={handleNext}
          hasPrev={currentIndex > 0}
          hasNext={hasNext}
        />}

        {/* Preview volume — full-width, below the card */}
        {current && <PreviewVolumeControl />}
      </div>
    </div>
  );
}

// ─── Preview Volume ─────────────────────────────────────────────────────────

function PreviewVolumeControl() {
  const previewVolume = useDiscoveryStore((s) => s.previewVolume);
  const setPreviewVolume = useDiscoveryStore((s) => s.setPreviewVolume);

  const volWheelRef = useWheelSlider({
    value: previewVolume, min: 0, max: 1, step: 0.01,
    onChange: setPreviewVolume,
  });

  return (
    <div className="mt-2 flex items-center gap-2 opacity-40 hover:opacity-100 transition-opacity">
      <svg
        className="w-3 h-3 shrink-0 text-text-muted"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        {previewVolume === 0 ? (
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15zM17 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2" />
        ) : previewVolume < 0.5 ? (
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15zM15.536 8.464a5 5 0 010 7.072" />
        ) : (
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5.586 15H4a1 1 0 01-1-1v-4a1 1 0 011-1h1.586l4.707-4.707C10.923 3.663 12 4.109 12 5v14c0 .891-1.077 1.337-1.707.707L5.586 15zM15.536 8.464a5 5 0 010 7.072M18.364 5.636a9 9 0 010 12.728" />
        )}
      </svg>
      <input
        ref={volWheelRef}
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={previewVolume}
        onChange={(e) => setPreviewVolume(parseFloat(e.target.value))}
        className="flex-1 h-2 accent-accent-primary cursor-pointer"
        title={`Preview volume: ${Math.round(previewVolume * 100)}%`}
      />
    </div>
  );
}

// ─── Suggestion Card ────────────────────────────────────────────────────────

interface SuggestionCardProps {
  suggestion: EnrichedSuggestion;
  profile: NonNullable<ReturnType<typeof useProfileStore.getState>["currentProfile"]>;
  config: ReturnType<typeof useSettingsStore.getState>["config"];
  onAdd: (s: EnrichedSuggestion) => void;
  onPreview: (s: EnrichedSuggestion) => void;
  onSeekPreview: (s: EnrichedSuggestion, position: number) => void;
  onUpdateAssignment: (
    videoId: string,
    updates: Partial<{
      suggestedKey: string;
      suggestedTrackId: string;
      suggestedMomentum: number;
    }>
  ) => void;
  onCapturingChange: (capturing: boolean) => void;
  onPrev: () => void;
  onNext: () => void;
  hasPrev: boolean;
  hasNext: boolean;
}

/** Truncate names smartly: short by default, but extend when names share the same prefix */
function smartTruncateNames(names: string[], maxLen = 25, extraChars = 5): string[] {
  return names.map((name, i) => {
    if (name.length <= maxLen) return name;

    // Check if any other name collides (same prefix up to maxLen)
    const hasCollision = names.some(
      (other, j) => j !== i && other.slice(0, maxLen) === name.slice(0, maxLen)
    );

    if (!hasCollision) return name.slice(0, maxLen) + "…";

    // Find longest shared prefix with any colliding name
    let maxCommon = 0;
    for (let j = 0; j < names.length; j++) {
      if (j === i) continue;
      const other = names[j];
      if (other.slice(0, maxLen) !== name.slice(0, maxLen)) continue;
      let k = 0;
      const limit = Math.min(name.length, other.length);
      while (k < limit && name[k] === other[k]) k++;
      if (k > maxCommon) maxCommon = k;
    }

    const showLen = Math.min(name.length, maxCommon + extraChars);
    return showLen >= name.length ? name : name.slice(0, showLen) + "…";
  });
}

function SuggestionCard({
  suggestion: s,
  profile,
  config,
  onAdd,
  onPreview,
  onSeekPreview,
  onUpdateAssignment,
  onCapturingChange,
  onPrev,
  onNext,
  hasPrev,
  hasNext,
}: SuggestionCardProps) {
  const playingTracks = useAudioStore((st) => st.playingTracks);
  const addToast = useToastStore((st) => st.addToast);
  const downloadProgress = useDiscoveryStore((st) => st.downloadProgresses[s.videoId] ?? s.downloadProgress);
  const previewEntry = playingTracks.get("__preview__");
  const previewPosition =
    previewEntry && previewEntry.soundId === s.videoId
      ? useAudioStore.getState().getPosition("__preview__")
      : undefined;

  const isReady = s.predownloadStatus === "ready";
  const isDownloading = s.predownloadStatus === "downloading";
  const isError = s.predownloadStatus === "error";

  // Inline key capture state
  const [isCapturing, setIsCapturing] = useState(false);
  const pressedKeysRef = useRef<Set<string>>(new Set());
  const badgeRef = useRef<HTMLButtonElement>(null);

  const startCapture = useCallback(() => {
    setIsCapturing(true);
    onCapturingChange(true);
    pressedKeysRef.current.clear();
  }, [onCapturingChange]);

  const cancelCapture = useCallback(() => {
    setIsCapturing(false);
    onCapturingChange(false);
    pressedKeysRef.current.clear();
  }, [onCapturingChange]);

  const finishCapture = useCallback(
    (combo: string) => {
      const conflict = checkShortcutConflicts(combo, {
        masterStopShortcut: config.masterStopShortcut,
        autoMomentumShortcut: config.autoMomentumShortcut,
        keyDetectionShortcut: config.keyDetectionShortcut,
      });
      if (conflict?.type === "error") {
        addToast(conflict.message, "error");
        pressedKeysRef.current.clear();
        return;
      }
      if (conflict?.type === "warning") {
        addToast(conflict.message, "info");
      }
      onUpdateAssignment(s.videoId, { suggestedKey: combo });
      setIsCapturing(false);
      onCapturingChange(false);
      pressedKeysRef.current.clear();
    },
    [config, s.videoId, onUpdateAssignment, onCapturingChange, addToast]
  );

  useEffect(() => {
    if (!isCapturing) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") {
        cancelCapture();
        return;
      }
      const code = getKeyCode(e);
      recordKeyLayout(code, e.key);
      pressedKeysRef.current.add(code);
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const code = getKeyCode(e);
      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (combo) finishCapture(combo);
      pressedKeysRef.current.delete(code);
    };

    const handleClickOutside = (e: MouseEvent) => {
      if (badgeRef.current && !badgeRef.current.contains(e.target as Node)) {
        cancelCapture();
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("keyup", handleKeyUp, true);
    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("keyup", handleKeyUp, true);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isCapturing, cancelCapture, finishCapture]);

  // Track cycling
  const currentTrack = profile.tracks.find((t) => t.id === s.suggestedTrackId);
  const cycleTrack = () => {
    const idx = profile.tracks.findIndex((t) => t.id === s.suggestedTrackId);
    const nextIdx = (idx + 1) % profile.tracks.length;
    onUpdateAssignment(s.videoId, { suggestedTrackId: profile.tracks[nextIdx].id });
  };

  const keyDisplay = s.suggestedKey ? keyCodeToDisplay(s.suggestedKey) : "?";

  const titleTooltip = [
    s.title,
    s.channel && `Channel: ${s.channel}`,
    s.occurrenceCount > 1 && `Score: ${s.occurrenceCount}`,
    s.occurrenceCount > 1 && `Found via: ${smartTruncateNames(s.sourceSeedNames).join(", ")}`,
  ].filter(Boolean).join("\n");

  const timingDisplay = s.suggestedMomentum > 0
    ? `${formatDuration(s.suggestedMomentum)}/${formatDuration(s.duration)}`
    : formatDuration(s.duration);

  return (
    <div className="space-y-1.5 group">
      {/* Row 1: Title */}
      <p className="text-text-primary text-sm truncate" title={titleTooltip}>
        {s.title}
      </p>

      {/* Row 2: Play button + Waveform */}
      <div className="flex items-center gap-1.5">
        <button
          onClick={() => onPreview(s)}
          disabled={!isReady}
          className={`w-5 h-5 shrink-0 flex items-center justify-center rounded transition-colors ${
            s.isPreviewPlaying
              ? "bg-accent-primary/30 text-accent-primary"
              : "bg-accent-primary/20 text-accent-primary hover:bg-accent-primary/30"
          } disabled:opacity-30 disabled:cursor-default`}
          title={isReady ? (s.isPreviewPlaying ? "Stop preview" : "Preview") : "Loading..."}
        >
          <span className="text-[10px] leading-none">
            {s.isPreviewPlaying ? "\u25A0" : "\u25B6"}
          </span>
        </button>
        <div className="flex-1 relative min-w-0">
          {isReady && s.waveform && (
            <WaveformDisplay
              waveformData={s.waveform}
              momentum={s.suggestedMomentum}
              onMomentumChange={(m) =>
                onUpdateAssignment(s.videoId, { suggestedMomentum: m })
              }
              onDragEnd={(m) => {
                onUpdateAssignment(s.videoId, { suggestedMomentum: m });
                if (s.isPreviewPlaying) onSeekPreview(s, m);
              }}
              playbackPosition={previewPosition}
              suggestedMomentum={s.waveform.suggestedMomentum}
              onAcceptSuggestion={() => {
                if (s.waveform?.suggestedMomentum != null) {
                  onUpdateAssignment(s.videoId, {
                    suggestedMomentum: s.waveform.suggestedMomentum,
                  });
                  if (s.isPreviewPlaying) onSeekPreview(s, s.waveform.suggestedMomentum);
                }
              }}
              height={28}
            />
          )}
          {s.predownloadStatus === "idle" && (
            <WaveformDisplay waveformData={null} height={28} />
          )}
          {isDownloading && (
            <WaveformDisplay waveformData={null} isLoading height={28} />
          )}
          {isError && (
            <div
              className="w-full bg-bg-secondary/50 rounded flex items-center justify-center"
              style={{ height: 28 }}
            >
              <span className="text-accent-error text-[10px]">Failed</span>
            </div>
          )}
          {/* Thin download progress bar */}
          {isDownloading && (
            <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-bg-tertiary rounded overflow-hidden">
              <div
                className="bg-accent-primary/50 h-full rounded transition-all"
                style={{ width: `${downloadProgress}%` }}
              />
            </div>
          )}
        </div>
      </div>

      {/* Row 3: Assignment + Add */}
      <div className="flex items-center gap-1 text-[10px]">
        {/* Inline key badge */}
        <button
          ref={badgeRef}
          onClick={isCapturing ? undefined : startCapture}
          className={`font-mono px-1.5 py-0.5 rounded transition-colors ${
            isCapturing
              ? "bg-accent-primary/30 text-accent-primary"
              : "text-accent-primary bg-accent-primary/10 hover:bg-accent-primary/20 cursor-pointer"
          }`}
          title={isCapturing ? "Press a key (Esc to cancel)" : "Click to change key"}
        >
          {isCapturing ? (
            <span className="animate-pulse">...</span>
          ) : (
            keyDisplay
          )}
        </button>
        <span className="text-text-muted">·</span>
        {/* Track — click to cycle */}
        <button
          onClick={cycleTrack}
          className="text-text-muted hover:text-text-primary hover:underline transition-colors cursor-pointer"
          title={`Track: ${currentTrack?.name || "Track"} (click to cycle)`}
        >
          {currentTrack?.name || "Track"}
        </button>
        <span className="text-text-muted">·</span>
        {/* Timing: momentum/duration or just duration */}
        <span className={s.suggestedMomentum > 0 ? "text-amber-400" : "text-text-muted"}>
          {timingDisplay}
        </span>
        <div className="flex-1" />
        {/* Navigation */}
        <button
          onClick={onPrev}
          disabled={!hasPrev}
          className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-default transition-colors"
          title="Previous"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <button
          onClick={onNext}
          disabled={!hasNext}
          className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-default transition-colors"
          title="Next"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
          </svg>
        </button>
        {/* Add button */}
        <button
          onClick={() => onAdd(s)}
          disabled={isDownloading}
          className="w-5 h-5 flex items-center justify-center rounded bg-accent-primary/20 text-accent-primary hover:bg-accent-primary/30 transition-colors disabled:opacity-30 disabled:cursor-default"
          title="Add to profile"
        >
          <span className="text-xs leading-none">+</span>
        </button>
      </div>
    </div>
  );
}
