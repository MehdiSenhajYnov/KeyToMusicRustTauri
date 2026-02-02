import React, { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../../stores/profileStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";
import { useWheelSlider } from "../../hooks/useWheelSlider";
import { useWaveformStore } from "../../stores/waveformStore";
import { keyCodeToDisplay } from "../../utils/keyMapping";
import { formatDuration } from "../../utils/fileHelpers";
import * as commands from "../../utils/tauriCommands";
import { KeyCaptureSlot } from "../Keys/KeyCaptureSlot";
import type { LoopMode, SoundSource, YoutubeSearchResult, YoutubePlaylist, WaveformData } from "../../types";
import { WaveformDisplay } from "../common/WaveformDisplay";
import { MomentumSuggestionBadge } from "../common/MomentumSuggestionBadge";
import { SearchResultPreview } from "./SearchResultPreview";

function WheelInput(props: React.InputHTMLAttributes<HTMLInputElement> & { wheelStep: number; wheelMin: number; wheelMax: number; onWheelChange: (v: number) => void }) {
  const { wheelStep, wheelMin, wheelMax, onWheelChange, ...inputProps } = props;
  const ref = useWheelSlider({
    value: Number(inputProps.value ?? 0),
    min: wheelMin, max: wheelMax, step: wheelStep,
    onChange: onWheelChange,
  });
  return <input ref={ref} {...inputProps} />;
}

type SourceMode = "local" | "youtube";

interface FileEntry {
  path: string;
  momentum: number;
  duration: number;
  name?: string;
  source?: SoundSource;
}

/** Memoized wrapper that manages its own waveform loading state per file.
 *  Streams partial waveform data from backend for a left-to-right reveal. */
const FileWaveform = React.memo(function FileWaveform({
  path,
  duration,
  momentum,
  onMomentumChange,
  onAutoApply,
  height = 50,
}: {
  path: string;
  duration: number;
  momentum: number;
  onMomentumChange?: (m: number) => void;
  onAutoApply?: (suggestedMomentum: number) => void;
  height?: number;
}) {
  const [waveform, setWaveform] = useState<WaveformData | null>(null);
  const autoApplied = useRef(false);
  const onAutoApplyRef = useRef(onAutoApply);
  onAutoApplyRef.current = onAutoApply;

  useEffect(() => {
    if (!path || duration <= 0) return;

    let cancelled = false;
    const tryAutoApply = (suggested: number | null | undefined) => {
      if (!autoApplied.current && onAutoApplyRef.current && suggested != null) {
        autoApplied.current = true;
        onAutoApplyRef.current(suggested);
      }
    };

    // Check global store first (non-reactive)
    const cached = useWaveformStore.getState().waveforms.get(path);
    if (cached) {
      setWaveform(cached);
      tryAutoApply(cached.suggestedMomentum);
      return;
    }

    // Listen for streaming progress — backend emits raw points every 5 iterations
    const NUM_POINTS = 100;
    let runningMax = 0.001;
    const unlistenPromise = listen<{
      path: string;
      points: number[];
      duration: number;
      sampleRate: number;
    }>("waveform_progress", (event) => {
      if (cancelled || event.payload.path !== path) return;
      const raw = event.payload.points;
      const batchMax = raw.reduce((m, v) => Math.max(m, v), 0.001);
      runningMax = Math.max(runningMax, batchMax);
      // Pad to full size: normalized computed values + zeros for uncomputed portion
      const points = new Array(NUM_POINTS).fill(0);
      for (let i = 0; i < raw.length; i++) {
        points[i] = raw[i] / runningMax;
      }
      setWaveform({
        points,
        duration: event.payload.duration,
        sampleRate: event.payload.sampleRate,
        suggestedMomentum: null,
      });
    });

    // Fire the full computation (returns final normalized + smoothed + momentum data)
    commands
      .getWaveform(path, NUM_POINTS)
      .then((data) => {
        if (cancelled) return;
        setWaveform(data);
        useWaveformStore.getState().setOne(path, data);
        tryAutoApply(data.suggestedMomentum);
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      unlistenPromise.then((fn) => fn());
    };
  }, [path, duration]);

  return (
    <WaveformDisplay
      waveformData={waveform}
      momentum={momentum}
      onMomentumChange={onMomentumChange}
      suggestedMomentum={waveform?.suggestedMomentum}
      height={height}
    />
  );
});

/** Fetch audio durations with bounded concurrency. */
function fetchDurationsConcurrent(
  paths: string[],
  maxConcurrent: number,
  onResult: (path: string, duration: number) => void
) {
  let i = 0;
  function next() {
    if (i >= paths.length) return;
    const path = paths[i++];
    commands.getAudioDuration(path)
      .then((duration) => onResult(path, duration))
      .catch(() => {})
      .finally(next);
  }
  for (let j = 0; j < Math.min(maxConcurrent, paths.length); j++) {
    next();
  }
}

interface AddSoundModalProps {
  targetKey?: string;
  initialFiles?: string[];
  onClose: () => void;
}

export function AddSoundModal({ targetKey, initialFiles, onClose }: AddSoundModalProps) {
  const { currentProfile, addSound, addKeyBinding, updateKeyBinding, addTrack, saveCurrentProfile } =
    useProfileStore();
  const addToast = useToastStore((s) => s.addToast);

  const [sourceMode, setSourceMode] = useState<SourceMode>("local");
  const [files, setFiles] = useState<FileEntry[]>(() => {
    if (initialFiles && initialFiles.length > 0) {
      return initialFiles.map((path) => ({ path, momentum: 0, duration: 0 }));
    }
    return [];
  });
  const processedFilesRef = useRef<string[] | undefined>(initialFiles);
  const [youtubeUrl, setYoutubeUrl] = useState("");
  const [activeDownloads, setActiveDownloads] = useState<
    Map<string, { url: string; status: string; progress: number | null }>
  >(new Map());
  const [isInstallingYtDlp, setIsInstallingYtDlp] = useState(false);
  const [ytDlpInstalled, setYtDlpInstalled] = useState<boolean | null>(null);
  const [searchResults, setSearchResults] = useState<YoutubeSearchResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [playlist, setPlaylist] = useState<YoutubePlaylist | null>(null);
  const [playlistSelected, setPlaylistSelected] = useState<Set<string>>(new Set());
  const [isFetchingPlaylist, setIsFetchingPlaylist] = useState(false);
  const [downloadEntirePlaylist, setDownloadEntirePlaylist] = useState(
    useSettingsStore.getState().config.playlistImportEnabled
  );
  const unlistenRef = useRef<(() => void) | null>(null);
  const downloadIdCounter = useRef(0);
  const searchDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const seekTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const selectedTrackIdRef = useRef(currentProfile?.tracks[0]?.id || "");
  // Key assignment: array of key codes (e.g., ["KeyA", "Ctrl+KeyB"])
  const [assignedKeys, setAssignedKeys] = useState<string[]>(targetKey ? [targetKey] : [""]);
  const [selectedTrackId, setSelectedTrackId] = useState(
    currentProfile?.tracks[0]?.id || ""
  );
  const [newTrackName, setNewTrackName] = useState("");
  const [volume, setVolume] = useState(100);
  const [loopMode, setLoopMode] = useState<LoopMode>("off");
  const [previewingIndex, setPreviewingIndex] = useState<number | null>(null);
  const [streamPreview, setStreamPreview] = useState<{
    videoId: string;
    streamUrl: string;
    duration: number;
    isLoading: boolean;
  } | null>(null);

  // Pre-fetched stream URLs (videoId → {streamUrl, duration})
  const prefetchedUrls = useRef<Map<string, { streamUrl: string; duration: number }>>(new Map());

  const tracks = currentProfile?.tracks || [];
  const isSingleFile = files.length <= 1;

  const volumeWheelRef = useWheelSlider({
    value: volume, min: 0, max: 100, step: 1,
    onChange: setVolume,
  });

  // Auto-search YouTube when typing (debounced, non-URL only)
  useEffect(() => {
    if (searchDebounceRef.current) clearTimeout(searchDebounceRef.current);
    const query = youtubeUrl.trim();
    const looksLikeUrl = query.toLowerCase().includes("youtube.com") || query.toLowerCase().includes("youtu.be");
    if (sourceMode !== "youtube" || query.length < 3 || looksLikeUrl) return;
    searchDebounceRef.current = setTimeout(async () => {
      setIsSearching(true);
      try {
        const results = await commands.searchYoutube(query, 6);
        setSearchResults(results);
      } catch {
        // Silent fail for auto-search — user can still click Search
      } finally {
        setIsSearching(false);
      }
    }, 400);
    return () => {
      if (searchDebounceRef.current) clearTimeout(searchDebounceRef.current);
    };
  }, [youtubeUrl, sourceMode]);

  // Pre-fetch stream URLs for first 2 search results (background, silent)
  // By the time user clicks play, the URL is already cached in backend → instant preview
  useEffect(() => {
    if (searchResults.length === 0 || !ytDlpInstalled) return;
    for (const result of searchResults.slice(0, 4)) {
      if (prefetchedUrls.current.has(result.videoId)) continue;
      commands.getYoutubeStreamUrl(result.videoId)
        .then((r) => {
          prefetchedUrls.current.set(result.videoId, { streamUrl: r.url, duration: r.duration });
        })
        .catch(() => {});
    }
  }, [searchResults, ytDlpInstalled]);

  // Handle initialFiles: on mount just fetch durations, on subsequent changes append files
  useEffect(() => {
    if (!initialFiles || initialFiles.length === 0) return;
    if (processedFilesRef.current === initialFiles) {
      // Same reference (mount or StrictMode re-run): just fetch durations (max 5 concurrent)
      fetchDurationsConcurrent(initialFiles, 5, (path, duration) => {
        setFiles((prev) =>
          prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
        );
      });
      return;
    }
    // New reference = new drop while modal is open: append files
    processedFilesRef.current = initialFiles;
    const entries: FileEntry[] = initialFiles.map((path) => ({ path, momentum: 0, duration: 0 }));
    setFiles((prev) => [...prev, ...entries]);
    fetchDurationsConcurrent(initialFiles, 5, (path, duration) => {
      setFiles((prev) =>
        prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
      );
    });
  }, [initialFiles]);


  const handleStopPreview = useCallback(() => {
    if (selectedTrackId) {
      commands.stopSound(selectedTrackId).catch(() => {});
    }
    setPreviewingIndex(null);
  }, [selectedTrackId]);

  // Close on Escape
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        handleStopPreview();
        onClose();
      }
    };
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [onClose, handleStopPreview]);

  // Keep ref in sync with state
  useEffect(() => {
    selectedTrackIdRef.current = selectedTrackId;
  }, [selectedTrackId]);

  // Stop preview on unmount
  useEffect(() => {
    return () => {
      if (selectedTrackIdRef.current) {
        commands.stopSound(selectedTrackIdRef.current).catch(() => {});
      }
    };
  }, []);

  // Listen for download progress events
  useEffect(() => {
    let cancelled = false;
    listen<{ downloadId: string; status: string; progress: number | null }>("youtube_download_progress", (event) => {
      if (cancelled) return;
      const { downloadId, status, progress } = event.payload;
      setActiveDownloads((prev) => {
        const next = new Map(prev);
        const existing = next.get(downloadId);
        if (existing) {
          next.set(downloadId, { ...existing, status, progress });
        }
        return next;
      });
    }).then((unlisten) => {
      if (cancelled) {
        unlisten();
      } else {
        unlistenRef.current = unlisten;
      }
    });
    return () => {
      cancelled = true;
      unlistenRef.current?.();
      unlistenRef.current = null;
    };
  }, []);

  // Check yt-dlp when switching to YouTube mode
  useEffect(() => {
    if (sourceMode === "youtube" && ytDlpInstalled === null) {
      commands.checkYtDlpInstalled().then(setYtDlpInstalled).catch(() => setYtDlpInstalled(false));
    }
  }, [sourceMode, ytDlpInstalled]);

  const handleInstallYtDlp = async () => {
    setIsInstallingYtDlp(true);
    try {
      await commands.installYtDlp();
      setYtDlpInstalled(true);
      addToast("yt-dlp installed successfully", "success");
    } catch (e) {
      addToast(`Failed to install yt-dlp: ${e}`, "error");
    } finally {
      setIsInstallingYtDlp(false);
    }
  };

  const isYoutubeUrl = (text: string): boolean => {
    const lower = text.toLowerCase();
    return lower.includes("youtube.com") || lower.includes("youtu.be");
  };

  const handleYoutubeSearch = async () => {
    const query = youtubeUrl.trim();
    if (!query) return;
    setIsSearching(true);
    try {
      const results = await commands.searchYoutube(query, 6);
      setSearchResults(results);
    } catch (e) {
      addToast(`Search failed: ${e}`, "error");
    } finally {
      setIsSearching(false);
    }
  };

  const hasPlaylistParam = (url: string): boolean => url.includes("list=");
  const hasVideoId = (url: string): boolean => url.includes("v=") || url.includes("youtu.be/");

  const handleFetchPlaylist = async (url: string) => {
    setIsFetchingPlaylist(true);
    try {
      const pl = await commands.fetchPlaylist(url);
      setPlaylist(pl);
      setPlaylistSelected(new Set(pl.entries.map((e) => e.videoId)));
    } catch (e) {
      addToast(`Failed to fetch playlist: ${e}`, "error");
    } finally {
      setIsFetchingPlaylist(false);
    }
  };

  const handlePlaylistDownloadSelected = () => {
    if (!playlist) return;
    const selected = playlist.entries.filter((e) => playlistSelected.has(e.videoId));
    for (const entry of selected) {
      handleSearchResultAdd(entry);
    }
    setPlaylist(null);
  };

  const handleYoutubeInput = () => {
    const input = youtubeUrl.trim();
    if (!input) return;
    if (isYoutubeUrl(input)) {
      if (hasPlaylistParam(input)) {
        if (!hasVideoId(input)) {
          // Pure playlist URL (no video ID)
          handleFetchPlaylist(input);
        } else if (downloadEntirePlaylist) {
          // Video + playlist, user wants entire playlist
          handleFetchPlaylist(input);
        } else {
          // Video + playlist, download single video (strip list param)
          handleYoutubeDownload();
        }
      } else {
        handleYoutubeDownload();
      }
    } else {
      handleYoutubeSearch();
    }
  };

  const handleSearchResultAdd = (result: YoutubeSearchResult) => {
    // Use the result URL to download
    const downloadId = `dl_${Date.now()}_${downloadIdCounter.current++}`;
    setActiveDownloads((prev) => {
      const next = new Map(prev);
      next.set(downloadId, { url: result.url, status: "Starting...", progress: null });
      return next;
    });

    commands.addSoundFromYoutube(result.url, downloadId)
      .then((sound) => {
        const entry: FileEntry = {
          path: sound.source.type === "youtube" ? sound.source.cachedPath : "",
          momentum: 0,
          duration: sound.duration,
          name: sound.name,
          source: sound.source,
        };
        setFiles((prev) => [...prev, entry]);
        addToast(`Downloaded: ${sound.name}`, "success");
        // Mark as downloaded in results
        setSearchResults((prev) =>
          prev.map((r) =>
            r.videoId === result.videoId ? { ...r, alreadyDownloaded: true } : r
          )
        );
      })
      .catch((e) => {
        addToast(String(e), "error");
      })
      .finally(() => {
        setActiveDownloads((prev) => {
          const next = new Map(prev);
          next.delete(downloadId);
          return next;
        });
      });
  };

  const handleStreamPreview = async (videoId: string) => {
    // If already previewing this video, close it
    if (streamPreview?.videoId === videoId) {
      setStreamPreview(null);
      return;
    }

    // Check pre-fetch cache for instant play
    const cached = prefetchedUrls.current.get(videoId);
    if (cached) {
      setStreamPreview({
        videoId,
        streamUrl: cached.streamUrl,
        duration: cached.duration,
        isLoading: false,
      });
      return;
    }

    // Fallback: fetch on demand
    setStreamPreview({ videoId, streamUrl: "", duration: 0, isLoading: true });

    try {
      const result = await commands.getYoutubeStreamUrl(videoId);
      // Guard: only apply result if this video is still the target
      setStreamPreview((prev) => {
        if (prev?.videoId !== videoId) return prev;
        return {
          videoId,
          streamUrl: result.url,
          duration: result.duration,
          isLoading: false,
        };
      });
    } catch {
      setStreamPreview((prev) => {
        if (prev?.videoId !== videoId) return prev;
        return null;
      });
      addToast("Failed to load stream preview", "error");
    }
  };

  const handleYoutubeDownload = async () => {
    const url = youtubeUrl.trim();
    if (!url) return;

    const downloadId = `dl_${Date.now()}_${downloadIdCounter.current++}`;
    setActiveDownloads((prev) => {
      const next = new Map(prev);
      next.set(downloadId, { url, status: "Starting...", progress: null });
      return next;
    });
    setYoutubeUrl("");

    try {
      const sound = await commands.addSoundFromYoutube(url, downloadId);
      const entry: FileEntry = {
        path: sound.source.type === "youtube" ? sound.source.cachedPath : "",
        momentum: 0,
        duration: sound.duration,
        name: sound.name,
        source: sound.source,
      };
      setFiles((prev) => [...prev, entry]);
      addToast(`Downloaded: ${sound.name}`, "success");
    } catch (e) {
      addToast(String(e), "error");
    } finally {
      setActiveDownloads((prev) => {
        const next = new Map(prev);
        next.delete(downloadId);
        return next;
      });
    }
  };

  const handleBrowseFiles = async () => {
    try {
      const paths = await commands.pickAudioFiles();
      if (paths.length === 0) return;

      const newEntries: FileEntry[] = paths.map((path) => ({
        path,
        momentum: 0,
        duration: 0,
      }));
      setFiles((prev) => [...prev, ...newEntries]);

      // Fetch durations in background (max 5 concurrent)
      fetchDurationsConcurrent(paths, 5, (path, duration) => {
        setFiles((prev) =>
          prev.map((f) =>
            f.path === path && f.duration === 0 ? { ...f, duration } : f
          )
        );
      });
    } catch (e) {
      addToast("Failed to open file picker", "error");
    }
  };

  const handleRemovePath = (index: number) => {
    if (previewingIndex === index) {
      handleStopPreview();
    } else if (previewingIndex !== null && previewingIndex > index) {
      setPreviewingIndex(previewingIndex - 1);
    }
    setFiles((prev) => prev.filter((_, i) => i !== index));
  };

  const handleMomentumChange = (index: number, momentum: number) => {
    setFiles((prev) =>
      prev.map((f, i) => (i === index ? { ...f, momentum } : f))
    );
    if (previewingIndex === index && selectedTrackId) {
      if (seekTimerRef.current) clearTimeout(seekTimerRef.current);
      const file = files[index];
      seekTimerRef.current = setTimeout(() => {
        commands.playSound(selectedTrackId, `preview-${index}`, file.path, momentum, volume / 100).catch(() => {});
      }, 150);
    }
  };

  const handlePreviewToggle = async (index: number) => {
    if (previewingIndex === index) {
      // Stop current preview
      handleStopPreview();
      return;
    }

    // Stop any currently playing preview
    if (previewingIndex !== null) {
      await commands.stopSound(selectedTrackId).catch(() => {});
    }

    const file = files[index];
    const trackId = selectedTrackId || currentProfile?.tracks[0]?.id;
    if (!trackId) {
      addToast("Select a track first to preview", "warning");
      return;
    }

    try {
      await commands.playSound(
        trackId,
        `preview-${index}`,
        file.path,
        file.momentum,
        volume / 100
      );
      setPreviewingIndex(index);
    } catch (e) {
      console.error("Preview failed:", e);
      addToast("Failed to play preview", "error");
    }
  };

  // Get config for conflict checking
  const config = useSettingsStore((s) => s.config);

  // Build conflict config for KeyCaptureSlot
  const conflictConfig = {
    masterStopShortcut: config?.masterStopShortcut,
    autoMomentumShortcut: config?.autoMomentumShortcut,
    keyDetectionShortcut: config?.keyDetectionShortcut,
  };

  // Key slot management
  const handleKeyChange = (index: number, keyCode: string) => {
    setAssignedKeys((prev) => {
      const next = [...prev];
      next[index] = keyCode;
      return next;
    });
  };

  const handleAddKeySlot = () => {
    setAssignedKeys((prev) => [...prev, ""]);
  };

  const handleRemoveKeySlot = (index: number) => {
    if (assignedKeys.length <= 1) return;
    setAssignedKeys((prev) => prev.filter((_, i) => i !== index));
  };

  // Get valid keys (non-empty)
  const validKeys = assignedKeys.filter((k) => k !== "");

  // Calculate cycling preview: which key each file will be assigned to
  const getKeyForFile = (fileIndex: number): string | null => {
    if (validKeys.length === 0) return null;
    return validKeys[fileIndex % validKeys.length];
  };

  const handleSubmit = () => {
    if (files.length === 0) {
      addToast("Add at least one audio file", "warning");
      return;
    }

    handleStopPreview();

    let trackId = selectedTrackId;

    if (!trackId && newTrackName.trim()) {
      trackId = crypto.randomUUID();
      addTrack({
        id: trackId,
        name: newTrackName.trim(),
        volume: 1.0,
      });
    }

    if (!trackId) {
      addToast("Select or create a track", "warning");
      return;
    }

    const keyCodes = targetKey ? [targetKey] : validKeys;
    if (keyCodes.length === 0) {
      addToast("Assign at least one key", "warning");
      return;
    }

    // Group sounds by key, then create/update bindings once per key
    const keyGroups: Map<string, string[]> = new Map();

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const fileName = file.path.split(/[/\\]/).pop() || "Sound";
      const name = file.name || fileName.replace(/\.[^.]+$/, "");
      const soundId = crypto.randomUUID();
      const source: SoundSource = file.source || { type: "local", path: file.path };

      addSound({
        id: soundId,
        name,
        source,
        momentum: file.momentum,
        volume: volume / 100,
        duration: file.duration,
      });

      const keyCode = keyCodes[i % keyCodes.length];
      if (!keyGroups.has(keyCode)) {
        keyGroups.set(keyCode, []);
      }
      keyGroups.get(keyCode)!.push(soundId);
    }

    for (const [keyCode, newSoundIds] of keyGroups) {
      const existingBinding = currentProfile?.keyBindings.find(
        (kb) => kb.keyCode === keyCode
      );

      if (existingBinding) {
        updateKeyBinding(keyCode, {
          soundIds: [...existingBinding.soundIds, ...newSoundIds],
        });
      } else {
        addKeyBinding({
          keyCode,
          trackId,
          soundIds: newSoundIds,
          loopMode,
          currentIndex: 0,
        });
      }
    }

    setTimeout(() => saveCurrentProfile(), 100);
    addToast(
      `${files.length} sound${files.length > 1 ? "s" : ""} added`,
      "success"
    );
    onClose();
  };

  const handleClose = () => {
    handleStopPreview();
    setStreamPreview(null);
    onClose();
  };

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={handleClose}>
      <div
        className="bg-bg-secondary border border-border-color rounded-lg w-[520px] max-h-[85vh] overflow-y-auto p-5 space-y-4"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="text-text-primary font-semibold">
            Add Sound{files.length > 1 ? "s" : ""}
          </h2>
          <button
            onClick={handleClose}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            &times;
          </button>
        </div>

        {/* Source mode toggle */}
        <div className="flex gap-1 bg-bg-tertiary rounded p-0.5">
          <button
            onClick={() => { setSourceMode("local"); setStreamPreview(null); }}
            className={`flex-1 px-3 py-1.5 text-sm rounded transition-colors ${
              sourceMode === "local"
                ? "bg-bg-secondary text-text-primary"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            Local File
          </button>
          <button
            onClick={() => setSourceMode("youtube")}
            className={`flex-1 px-3 py-1.5 text-sm rounded transition-colors ${
              sourceMode === "youtube"
                ? "bg-bg-secondary text-text-primary"
                : "text-text-muted hover:text-text-secondary"
            }`}
          >
            YouTube
          </button>
        </div>

        {/* Local file input */}
        {sourceMode === "local" && (
          <div className="space-y-2">
            <label className="text-text-secondary text-sm">Audio Files</label>
            <button
              onClick={handleBrowseFiles}
              className="w-full px-3 py-2 bg-accent-primary/20 text-accent-primary rounded text-sm hover:bg-accent-primary/30 transition-colors"
            >
              Add Files
            </button>
          </div>
        )}

        {/* YouTube URL input */}
        {sourceMode === "youtube" && (
          <div className="space-y-2">
            <label className="text-text-secondary text-sm">YouTube URL</label>
            {ytDlpInstalled === false && (
              <div className="flex items-center gap-2 bg-bg-tertiary border border-border-color rounded p-2.5">
                <span className="text-text-secondary text-xs flex-1">
                  yt-dlp is required for YouTube downloads.
                </span>
                <button
                  onClick={handleInstallYtDlp}
                  disabled={isInstallingYtDlp}
                  className="px-3 py-1.5 bg-accent-primary text-white rounded text-xs hover:bg-accent-primary/80 disabled:opacity-50 whitespace-nowrap"
                >
                  {isInstallingYtDlp ? "Installing..." : "Install yt-dlp"}
                </button>
              </div>
            )}
            {ytDlpInstalled !== false && (
              <>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={youtubeUrl}
                    onChange={(e) => setYoutubeUrl(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && youtubeUrl.trim()) handleYoutubeInput();
                    }}
                    placeholder="Paste YouTube URL or search..."
                    className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none"
                  />
                  <button
                    onClick={handleYoutubeInput}
                    disabled={!youtubeUrl.trim() || isSearching}
                    className="px-3 py-1.5 bg-accent-primary/20 text-accent-primary rounded text-sm hover:bg-accent-primary/30 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                  >
                    {isSearching ? "Searching..." : isYoutubeUrl(youtubeUrl) ? "Download" : "Search"}
                  </button>
                </div>
                {activeDownloads.size > 0 && (
                  <div className="space-y-2">
                    {[...activeDownloads.entries()].map(([id, dl]) => (
                      <div key={id} className="space-y-1.5 bg-bg-tertiary rounded p-2">
                        <div className="flex items-center gap-2">
                          <div className="w-3 h-3 border-2 border-accent-primary border-t-transparent rounded-full animate-spin shrink-0" />
                          <span className="text-text-secondary text-xs truncate flex-1">
                            {dl.status}
                            {dl.progress != null && ` ${Math.round(dl.progress)}%`}
                          </span>
                        </div>
                        {dl.progress != null && (
                          <div className="w-full bg-bg-secondary rounded-full h-1.5 overflow-hidden">
                            <div
                              className="bg-accent-primary h-full rounded-full transition-all duration-300"
                              style={{ width: `${Math.min(100, dl.progress)}%` }}
                            />
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                )}
                {/* Search results */}
                {searchResults.length > 0 && (
                  <div className="space-y-1 max-h-[200px] overflow-y-auto">
                    {searchResults.map((result) => {
                      const isPreviewTarget = streamPreview?.videoId === result.videoId;
                      const isPreviewLoading = isPreviewTarget && streamPreview?.isLoading;
                      return (
                        <div key={result.videoId}>
                          <div className="flex items-center gap-2 bg-bg-tertiary rounded p-2 hover:bg-bg-hover transition-colors">
                            <div className="flex-1 min-w-0">
                              <p className="text-text-primary text-xs truncate">{result.title}</p>
                              <div className="flex items-center gap-2 text-text-muted text-[10px]">
                                {result.channel && <span className="truncate">{result.channel}</span>}
                                {result.duration > 0 && (
                                  <span className="shrink-0">{formatDuration(result.duration)}</span>
                                )}
                                {result.alreadyDownloaded && (
                                  <span className="text-green-400 shrink-0">Downloaded</span>
                                )}
                              </div>
                            </div>
                            {ytDlpInstalled && (
                              <button
                                onClick={() => handleStreamPreview(result.videoId)}
                                disabled={isPreviewLoading}
                                className={`w-6 h-6 flex items-center justify-center rounded shrink-0 ${
                                  isPreviewTarget
                                    ? "bg-accent-primary/20 text-accent-primary"
                                    : "text-text-muted hover:text-accent-primary hover:bg-bg-secondary"
                                } disabled:opacity-50`}
                                title={isPreviewTarget ? "Stop preview" : "Preview"}
                              >
                                {isPreviewLoading ? (
                                  <div className="w-3 h-3 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
                                ) : isPreviewTarget ? (
                                  "\u25A0"
                                ) : (
                                  "\u25B6"
                                )}
                              </button>
                            )}
                            <button
                              onClick={() => { handleSearchResultAdd(result); if (isPreviewTarget) setStreamPreview(null); }}
                              disabled={activeDownloads.size > 0 && [...activeDownloads.values()].some(d => d.url === result.url)}
                              className="px-2 py-1 bg-accent-primary/20 text-accent-primary rounded text-xs hover:bg-accent-primary/30 disabled:opacity-50 shrink-0"
                            >
                              Add
                            </button>
                          </div>
                          {isPreviewTarget && streamPreview.streamUrl && (
                            <SearchResultPreview
                              streamUrl={streamPreview.streamUrl}
                              duration={streamPreview.duration}
                              onClose={() => setStreamPreview(null)}
                            />
                          )}
                        </div>
                      );
                    })}
                  </div>
                )}
                {/* Playlist checkbox for video+playlist URLs */}
                {isYoutubeUrl(youtubeUrl) && hasPlaylistParam(youtubeUrl) && hasVideoId(youtubeUrl) && (
                  <label className="flex items-center gap-2 text-text-secondary text-xs bg-bg-tertiary rounded p-2 cursor-pointer">
                    <input
                      type="checkbox"
                      checked={downloadEntirePlaylist}
                      onChange={(e) => {
                        setDownloadEntirePlaylist(e.target.checked);
                        useSettingsStore.getState().setPlaylistImportEnabled(e.target.checked);
                      }}
                      className="accent-accent-primary"
                    />
                    Download entire playlist
                  </label>
                )}
                {/* Playlist selector */}
                {playlist && (
                  <div className="space-y-2 border border-border-color rounded p-2">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-text-primary text-xs font-semibold">{playlist.title}</p>
                        <p className="text-text-muted text-[10px]">{playlist.totalCount} videos</p>
                      </div>
                      <div className="flex gap-1">
                        <button
                          onClick={() => setPlaylistSelected(new Set(playlist.entries.map((e) => e.videoId)))}
                          className="text-accent-primary text-[10px] hover:underline"
                        >
                          All
                        </button>
                        <span className="text-text-muted text-[10px]">/</span>
                        <button
                          onClick={() => setPlaylistSelected(new Set())}
                          className="text-accent-primary text-[10px] hover:underline"
                        >
                          None
                        </button>
                      </div>
                    </div>
                    <div className="space-y-0.5 max-h-[180px] overflow-y-auto">
                      {playlist.entries.map((entry) => (
                        <label
                          key={entry.videoId}
                          className="flex items-center gap-2 p-1.5 rounded hover:bg-bg-hover cursor-pointer text-xs"
                        >
                          <input
                            type="checkbox"
                            checked={playlistSelected.has(entry.videoId)}
                            onChange={(e) => {
                              setPlaylistSelected((prev) => {
                                const next = new Set(prev);
                                if (e.target.checked) {
                                  next.add(entry.videoId);
                                } else {
                                  next.delete(entry.videoId);
                                }
                                return next;
                              });
                            }}
                            className="accent-accent-primary shrink-0"
                          />
                          <span className="text-text-primary truncate flex-1">{entry.title}</span>
                          {entry.duration > 0 && (
                            <span className="text-text-muted text-[10px] shrink-0">
                              {formatDuration(entry.duration)}
                            </span>
                          )}
                          {entry.alreadyDownloaded && (
                            <span className="text-green-400 text-[10px] shrink-0">DL'd</span>
                          )}
                        </label>
                      ))}
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={handlePlaylistDownloadSelected}
                        disabled={playlistSelected.size === 0}
                        className="flex-1 px-3 py-1.5 bg-accent-primary/20 text-accent-primary rounded text-xs hover:bg-accent-primary/30 disabled:opacity-50"
                      >
                        Add {playlistSelected.size} selected
                      </button>
                      <button
                        onClick={() => setPlaylist(null)}
                        className="px-3 py-1.5 text-text-muted hover:text-text-primary text-xs rounded hover:bg-bg-hover"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                )}
                {(isSearching || isFetchingPlaylist) && (
                  <div className="flex items-center justify-center gap-2 py-3">
                    <div className="w-4 h-4 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
                    <span className="text-text-muted text-xs">
                      {isFetchingPlaylist ? "Fetching playlist..." : "Searching YouTube..."}
                    </span>
                  </div>
                )}
              </>
            )}
          </div>
        )}

        {/* Per-file momentum editors */}
        {files.length > 0 && (
          <div className="space-y-2 max-h-[300px] overflow-y-auto">
            {files.map((file, i) => {
              const fileName = file.name || file.path.split(/[/\\]/).pop() || "File";
              const isPreviewing = previewingIndex === i;

              return (
                <div key={i} className="bg-bg-tertiary rounded p-2 space-y-1.5">
                  <div className="flex items-center gap-2">
                    {!isSingleFile && (
                      <span className="text-accent-primary font-mono text-xs font-bold bg-bg-secondary px-1.5 py-0.5 rounded shrink-0 min-w-[24px] text-center">
                        {getKeyForFile(i)
                          ? keyCodeToDisplay(getKeyForFile(i)!)
                          : "-"}
                      </span>
                    )}
                    <span className="text-text-primary text-xs flex-1 truncate">
                      {fileName}
                    </span>
                    {(() => {
                      const wf = useWaveformStore.getState().waveforms.get(file.path);
                      return wf?.suggestedMomentum != null &&
                        Math.abs(wf.suggestedMomentum - file.momentum) > 0.3 ? (
                        <span
                          className="shrink-0 px-1 py-0.5 rounded text-[8px] font-medium
                                     bg-cyan-500/15 text-cyan-400 border border-cyan-500/30"
                          title={`Suggested: ${wf.suggestedMomentum.toFixed(1)}s`}
                        >
                          Auto
                        </span>
                      ) : null;
                    })()}
                    <button
                      onClick={() => handleRemovePath(i)}
                      className="text-text-muted hover:text-accent-error text-sm shrink-0"
                    >
                      &times;
                    </button>
                  </div>
                  {/* Waveform display */}
                  <FileWaveform
                    path={file.path}
                    duration={file.duration}
                    momentum={file.momentum}
                    onMomentumChange={(m) => handleMomentumChange(i, m)}
                    onAutoApply={config.autoMomentum ? (suggested) => {
                      handleMomentumChange(i, Math.round(suggested * 10) / 10);
                    } : undefined}
                    height={50}
                  />
                  {/* Momentum editor */}
                  <div className="flex items-center gap-2 text-xs text-text-muted">
                    <button
                      onClick={() => handlePreviewToggle(i)}
                      className={`w-6 h-6 flex items-center justify-center rounded shrink-0 ${
                        isPreviewing
                          ? "bg-accent-error/20 text-accent-error"
                          : "bg-bg-secondary text-text-secondary hover:text-text-primary"
                      }`}
                      title={isPreviewing ? "Stop" : "Play from momentum"}
                    >
                      {isPreviewing ? "\u25A0" : "\u25B6"}
                    </button>
                    <span className="text-text-secondary whitespace-nowrap">Momentum:</span>
                    {(() => {
                      const wf = useWaveformStore.getState().waveforms.get(file.path);
                      return wf?.suggestedMomentum != null ? (
                        <MomentumSuggestionBadge
                          suggestedMomentum={wf.suggestedMomentum}
                          currentMomentum={file.momentum}
                          onApply={() => handleMomentumChange(i, Math.round(wf.suggestedMomentum! * 10) / 10)}
                          size="sm"
                        />
                      ) : null;
                    })()}
                    <input
                      type="number"
                      min="0"
                      max={file.duration || undefined}
                      step="0.5"
                      value={file.momentum}
                      onChange={(e) => {
                        const val = Math.max(0, Number(e.target.value));
                        handleMomentumChange(i, val);
                      }}
                      className="w-14 bg-bg-secondary border border-border-color rounded px-1 py-0.5 text-text-primary text-xs"
                    />
                    <span>s</span>
                    <WheelInput
                      type="range"
                      min="0"
                      max={file.duration > 0 ? file.duration : 1}
                      step="0.1"
                      value={file.momentum}
                      disabled={file.duration === 0}
                      onChange={(e) => {
                        handleMomentumChange(i, Number(e.target.value));
                      }}
                      className="flex-1 h-1 accent-accent-primary disabled:opacity-30"
                      wheelStep={0.5} wheelMin={0} wheelMax={file.duration > 0 ? file.duration : 1}
                      onWheelChange={(v) => handleMomentumChange(i, v)}
                    />
                    <span className="text-text-muted whitespace-nowrap">
                      {formatDuration(file.duration)}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Key assignment */}
        {!targetKey && (
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-text-secondary text-sm">
                {isSingleFile ? "Key" : `Keys`}
              </label>
              {files.length > 1 && validKeys.length < files.length && (
                <span className="text-text-muted text-xs">
                  {validKeys.length} key{validKeys.length !== 1 ? "s" : ""} for {files.length} sounds
                  {validKeys.length > 0 && " (will cycle)"}
                </span>
              )}
            </div>

            {/* Key capture slots */}
            <div className="space-y-2">
              {assignedKeys.map((keyCode, index) => (
                <KeyCaptureSlot
                  key={index}
                  value={keyCode}
                  onChange={(newKey) => handleKeyChange(index, newKey)}
                  onRemove={assignedKeys.length > 1 ? () => handleRemoveKeySlot(index) : undefined}
                  removable={assignedKeys.length > 1}
                  conflictConfig={conflictConfig}
                  index={assignedKeys.length > 1 ? index + 1 : undefined}
                  placeholder="Click to assign key"
                />
              ))}
            </div>

            {/* Add key button - only show when keys < sounds */}
            {files.length > 1 && assignedKeys.length < files.length && (
              <button
                type="button"
                onClick={handleAddKeySlot}
                className="w-full px-3 py-2 text-sm text-text-muted hover:text-accent-primary border border-dashed border-border-color hover:border-accent-primary/50 rounded transition-colors flex items-center justify-center gap-2"
              >
                <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
                Add another key
              </button>
            )}

            {/* Cycling explanation */}
            {files.length > 1 && validKeys.length > 0 && validKeys.length < files.length && (
              <p className="text-text-muted text-xs bg-bg-tertiary rounded p-2">
                Sounds will cycle through keys: {files.map((_, i) =>
                  keyCodeToDisplay(getKeyForFile(i) || "?")
                ).join(", ")}
              </p>
            )}
          </div>
        )}

        {/* Track selection */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm">Track</label>
          {tracks.length > 0 ? (
            <select
              value={selectedTrackId}
              onChange={(e) => {
                setSelectedTrackId(e.target.value);
                if (e.target.value) setNewTrackName("");
              }}
              className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary"
            >
              {tracks.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
              <option value="">+ New Track</option>
            </select>
          ) : null}
          {(!selectedTrackId || tracks.length === 0) && (
            <input
              type="text"
              value={newTrackName}
              onChange={(e) => setNewTrackName(e.target.value)}
              placeholder="New track name"
              className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none mt-1"
            />
          )}
        </div>

        {/* Volume */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm">
            Volume ({volume}%)
          </label>
          <input
            ref={volumeWheelRef}
            type="range"
            min="0"
            max="100"
            value={volume}
            onChange={(e) => setVolume(Number(e.target.value))}
            className="w-full h-1 accent-accent-secondary"
          />
        </div>

        {/* Loop Mode */}
        <div className="space-y-1">
          <label className="text-text-secondary text-sm">Loop Mode</label>
          <select
            value={loopMode}
            onChange={(e) => setLoopMode(e.target.value as LoopMode)}
            className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary"
          >
            <option value="off">Off - Play once</option>
            <option value="single">Single - Loop same sound</option>
            <option value="sequential">Sequential - Cycle in order</option>
            <option value="random">Random - Pick randomly</option>
          </select>
        </div>

        {/* Actions */}
        <div className="flex justify-end gap-2 pt-2 border-t border-border-color">
          <button
            onClick={handleClose}
            className="px-4 py-2 text-text-secondary hover:text-text-primary text-sm rounded hover:bg-bg-hover"
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            className="px-4 py-2 bg-accent-primary text-white text-sm rounded hover:bg-accent-primary/80"
          >
            Add {files.length > 1 ? `${files.length} Sounds` : "Sound"}
          </button>
        </div>
      </div>
    </div>
  );
}
