import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  useDiscoveryStore,
  type EnrichedSuggestion,
} from "../../stores/discoveryStore";
import { useProfileStore } from "../../stores/profileStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";
import { useAudioStore } from "../../stores/audioStore";
import { formatDuration } from "../../utils/fileHelpers";
import {
  findNextAvailableKey,
  keyCodeToDisplay,
  getKeyCode,
  recordKeyLayout,
  buildComboFromPressedKeys,
  checkShortcutConflicts,
} from "../../utils/keyMapping";
import { findLeastUsedTrack } from "../../utils/soundHelpers";
import { WaveformDisplay } from "../common/WaveformDisplay";
import * as commands from "../../utils/tauriCommands";
import type { SoundSource } from "../../types";

export function DiscoveryPanel() {
  const profile = useProfileStore((s) => s.currentProfile);
  const { addSound, addKeyBinding, saveCurrentProfile } = useProfileStore();
  const config = useSettingsStore((s) => s.config);
  const addToast = useToastStore((s) => s.addToast);

  const {
    allSuggestions,
    visibleSuggestions,
    currentIndex,
    isGenerating,
    progress,
    error,
    setSuggestions,
    removeSuggestion,
    setGenerating,
    setPreviewPlaying,
    updateSuggestionAssignment,
    goToNext,
    goToPrev,
    revealMore,
    clear,
  } = useDiscoveryStore();

  const [isCapturingKey, setIsCapturingKey] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const downloadIdCounter = useRef(0);

  const hasYoutubeSounds =
    profile?.sounds.some((s) => s.source.type === "youtube") ?? false;

  // Build enricher function for auto-assignment
  const buildEnricher = useCallback(() => {
    if (!profile) return (_s: unknown, _i: number) => ({ suggestedKey: "", suggestedTrackId: "" });

    const usedKeys = new Set(profile.keyBindings.map((kb) => kb.keyCode));
    const alreadySuggested = new Set<string>();
    const trackId = findLeastUsedTrack(profile.tracks, profile.keyBindings);

    // Also include keys already suggested in visible suggestions
    const vis = useDiscoveryStore.getState().visibleSuggestions;
    for (const s of vis) {
      if (s.suggestedKey) alreadySuggested.add(s.suggestedKey);
    }

    return (_s: unknown, _i: number) => {
      const key = findNextAvailableKey(usedKeys, alreadySuggested);
      if (key) alreadySuggested.add(key);
      return { suggestedKey: key, suggestedTrackId: trackId };
    };
  }, [profile]);

  // Load cached suggestions when profile changes, auto-discover if empty
  useEffect(() => {
    if (!profile) {
      clear();
      return;
    }
    const hasYt = profile.sounds.some((s) => s.source.type === "youtube");
    commands
      .getDiscoverySuggestions(profile.id)
      .then((cached) => {
        if (cached && cached.length > 0) {
          setSuggestions(cached, buildEnricher());
        } else if (hasYt) {
          // Auto-trigger discovery when no cached results
          setGenerating(true);
          commands.startDiscovery(profile.id)
            .then((results) => {
              setSuggestions(results, buildEnricher());
              if (results.length === 0) {
                addToast("No new suggestions found", "info");
              }
            })
            .catch((e) => {
              const msg = String(e);
              if (!msg.includes("cancelled")) {
                addToast(`Discovery failed: ${msg}`, "error");
              }
            })
            .finally(() => setGenerating(false));
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
    setGenerating(true);
    try {
      const results = await commands.startDiscovery(profile.id);
      setSuggestions(results, buildEnricher());
      if (results.length === 0) {
        addToast("No new suggestions found", "info");
      }
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("cancelled")) {
        addToast(`Discovery failed: ${msg}`, "error");
      }
    } finally {
      setGenerating(false);
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
      await commands.playSound(
        "__preview__",
        s.videoId,
        s.cachedPath,
        s.suggestedMomentum,
        1.0
      );
    } catch {
      setPreviewPlaying(s.videoId, false);
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
      await commands.playSound("__preview__", s.videoId, s.cachedPath, position, 1.0);
    } catch {
      // ignore
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

      if (s.suggestedKey) {
        addKeyBinding({
          keyCode: s.suggestedKey,
          trackId: s.suggestedTrackId,
          soundIds: [soundId],
          loopMode: "off",
          currentIndex: 0,
        });
      }

      setTimeout(() => saveCurrentProfile(), 100);
      removeSuggestion(s.videoId);
      addToast(`Added: ${s.title}`, "success");
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

        if (s.suggestedKey) {
          addKeyBinding({
            keyCode: s.suggestedKey,
            trackId: s.suggestedTrackId,
            soundIds: [soundId],
            loopMode: "off",
            currentIndex: 0,
          });
        }

        setTimeout(() => saveCurrentProfile(), 100);
        removeSuggestion(s.videoId);
        addToast(`Added: ${sound.name}`, "success");
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
    goToNext();
    // Trigger reveal if near end
    const state = useDiscoveryStore.getState();
    if (state.currentIndex >= state.revealedCount - 3) {
      revealMore(buildEnricher());
    }
  };

  const handlePrev = () => {
    stopPreview();
    goToPrev();
  };

  if (!profile) return null;

  const current = visibleSuggestions[currentIndex];
  const total = allSuggestions.length;
  const showing = visibleSuggestions.length;

  return (
    <div
      ref={panelRef}
      className="border-t border-border-color flex flex-col shrink-0"
      tabIndex={0}
      onKeyDown={handleKeyDown}
    >
      {/* Header */}
      <div className="px-3 pt-2 pb-1 flex items-center justify-between">
        <div className="flex items-center gap-1">
          <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
            Discover
          </h3>
          {total > 0 && (
            <span className="text-accent-primary text-[10px]">
              ({currentIndex + 1}/{total})
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {showing > 0 && (
            <>
              <button
                onClick={handlePrev}
                disabled={currentIndex <= 0}
                className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-default transition-colors"
                title="Previous"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
                </svg>
              </button>
              <button
                onClick={handleNext}
                disabled={currentIndex >= showing - 1}
                className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-text-primary hover:bg-bg-hover disabled:opacity-30 disabled:cursor-default transition-colors"
                title="Next"
              >
                <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                </svg>
              </button>
            </>
          )}
          {hasYoutubeSounds && (
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
      </div>

      {/* Content */}
      <div className="px-3 pb-3">
        {/* No YouTube sounds */}
        {!hasYoutubeSounds && (
          <p className="text-text-muted text-[10px] italic py-1">
            Add YouTube sounds to get recommendations
          </p>
        )}

        {/* Has YouTube sounds but no suggestions yet */}
        {hasYoutubeSounds && showing === 0 && !isGenerating && !error && (
          <p className="text-text-muted text-[10px] italic py-1">
            No suggestions yet
          </p>
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
          onDismiss={handleDismiss}
          onAdd={handleAdd}
          onPreview={handlePreview}
          onSeekPreview={handleSeekPreview}
          onUpdateAssignment={updateSuggestionAssignment}
          onCapturingChange={setIsCapturingKey}
        />}
      </div>
    </div>
  );
}

// ─── Suggestion Card ────────────────────────────────────────────────────────

interface SuggestionCardProps {
  suggestion: EnrichedSuggestion;
  profile: NonNullable<ReturnType<typeof useProfileStore.getState>["currentProfile"]>;
  config: ReturnType<typeof useSettingsStore.getState>["config"];
  onDismiss: (videoId: string) => void;
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
}

function SuggestionCard({
  suggestion: s,
  profile,
  config,
  onDismiss,
  onAdd,
  onPreview,
  onSeekPreview,
  onUpdateAssignment,
  onCapturingChange,
}: SuggestionCardProps) {
  const playingTracks = useAudioStore((st) => st.playingTracks);
  const addToast = useToastStore((st) => st.addToast);
  const previewEntry = playingTracks.get("__preview__");
  const previewPosition =
    previewEntry && previewEntry.soundId === s.videoId
      ? previewEntry.position
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
    s.occurrenceCount > 1 && `Found via: ${s.sourceSeedNames.join(", ")}`,
  ].filter(Boolean).join("\n");

  const timingDisplay = s.suggestedMomentum > 0
    ? `${formatDuration(s.suggestedMomentum)}/${formatDuration(s.duration)}`
    : formatDuration(s.duration);

  return (
    <div className="space-y-1.5 group">
      {/* Row 1: Title + dismiss */}
      <div className="flex items-center gap-1">
        <p className="text-text-primary text-sm truncate flex-1" title={titleTooltip}>
          {s.title}
        </p>
        {s.occurrenceCount > 1 && (
          <span
            className="text-accent-primary text-[10px] shrink-0"
            title={`Found via: ${s.sourceSeedNames.join(", ")}`}
          >
            {"·".repeat(Math.min(s.occurrenceCount, 3))}
          </span>
        )}
        <button
          onClick={() => onDismiss(s.videoId)}
          className="shrink-0 w-4 h-4 flex items-center justify-center text-text-muted hover:text-accent-error opacity-0 group-hover:opacity-100 transition-all"
          title="Dismiss"
        >
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

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
            <div className="w-full bg-bg-secondary/50 rounded" style={{ height: 28 }} />
          )}
          {isDownloading && (
            <div className="w-full bg-bg-secondary/50 rounded" style={{ height: 28 }} />
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
                style={{ width: `${s.downloadProgress}%` }}
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
