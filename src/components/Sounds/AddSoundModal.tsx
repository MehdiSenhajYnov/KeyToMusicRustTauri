import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../../stores/profileStore";
import { useToastStore } from "../../stores/toastStore";
import { parseKeyCombination } from "../../utils/keyMapping";
import { isAudioFile, formatDuration } from "../../utils/fileHelpers";
import * as commands from "../../utils/tauriCommands";
import type { LoopMode, SoundSource } from "../../types";

type SourceMode = "local" | "youtube";

interface FileEntry {
  path: string;
  momentum: number;
  duration: number;
  name?: string;
  source?: SoundSource;
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
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [pathInput, setPathInput] = useState("");
  const [youtubeUrl, setYoutubeUrl] = useState("");
  const [isDownloading, setIsDownloading] = useState(false);
  const [downloadStatus, setDownloadStatus] = useState("");
  const [downloadProgress, setDownloadProgress] = useState<number | null>(null);
  const [isInstallingYtDlp, setIsInstallingYtDlp] = useState(false);
  const [ytDlpInstalled, setYtDlpInstalled] = useState<boolean | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);
  const [keysInput, setKeysInput] = useState(targetKey || "");
  const [selectedTrackId, setSelectedTrackId] = useState(
    currentProfile?.tracks[0]?.id || ""
  );
  const [newTrackName, setNewTrackName] = useState("");
  const [volume, setVolume] = useState(100);
  const [loopMode, setLoopMode] = useState<LoopMode>("off");
  const [previewingIndex, setPreviewingIndex] = useState<number | null>(null);

  const tracks = currentProfile?.tracks || [];
  const isSingleFile = files.length <= 1;

  // Initialize from initialFiles
  useEffect(() => {
    if (initialFiles && initialFiles.length > 0) {
      const entries = initialFiles.map((path) => ({ path, momentum: 0, duration: 0 }));
      setFiles(entries);
      // Fetch durations
      initialFiles.forEach((path, i) => {
        commands.getAudioDuration(path).then((duration) => {
          setFiles((prev) =>
            prev.map((f, idx) => (idx === i ? { ...f, duration } : f))
          );
        }).catch(() => {});
      });
    }
  }, [initialFiles]);

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
  }, [onClose]);

  // Stop preview on unmount
  useEffect(() => {
    return () => {
      if (selectedTrackId) {
        commands.stopSound(selectedTrackId).catch(() => {});
      }
    };
  }, [selectedTrackId]);

  // Listen for download progress events
  useEffect(() => {
    let cancelled = false;
    listen<{ status: string; progress: number | null }>("youtube_download_progress", (event) => {
      if (cancelled) return;
      setDownloadStatus(event.payload.status);
      setDownloadProgress(event.payload.progress);
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

  const handleYoutubeDownload = async () => {
    const url = youtubeUrl.trim();
    if (!url) return;

    setIsDownloading(true);
    setDownloadStatus("Starting...");
    setDownloadProgress(null);
    try {
      const sound = await commands.addSoundFromYoutube(url);
      const entry: FileEntry = {
        path: sound.source.type === "youtube" ? sound.source.cachedPath : "",
        momentum: 0,
        duration: sound.duration,
        name: sound.name,
        source: sound.source,
      };
      setFiles((prev) => [...prev, entry]);
      setYoutubeUrl("");
      addToast(`Downloaded: ${sound.name}`, "success");
    } catch (e) {
      addToast(String(e), "error");
    } finally {
      setIsDownloading(false);
      setDownloadStatus("");
      setDownloadProgress(null);
    }
  };

  const handleAddPath = async () => {
    const path = pathInput.trim();
    if (!path) return;
    if (!isAudioFile(path)) {
      addToast("Unsupported format. Use MP3, WAV, OGG, or FLAC.", "error");
      return;
    }
    const entry: FileEntry = { path, momentum: 0, duration: 0 };
    setFiles((prev) => [...prev, entry]);
    setPathInput("");

    // Fetch duration in background
    try {
      const duration = await commands.getAudioDuration(path);
      setFiles((prev) =>
        prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
      );
    } catch {
      // Duration will stay 0
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
  };

  const handleStopPreview = useCallback(() => {
    if (selectedTrackId) {
      commands.stopSound(selectedTrackId).catch(() => {});
    }
    setPreviewingIndex(null);
  }, [selectedTrackId]);

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

  // Key input behavior: single file = replace, multiple = build up
  const handleKeyInput = (value: string) => {
    if (targetKey) return;

    if (isSingleFile) {
      const lastChar = value.slice(-1).toLowerCase();
      setKeysInput(lastChar);
    } else {
      const limited = value.toLowerCase().slice(0, files.length);
      setKeysInput(limited);
    }
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
        currentlyPlaying: null,
        playbackPosition: 0,
        isPlaying: false,
      });
    }

    if (!trackId) {
      addToast("Select or create a track", "warning");
      return;
    }

    const keyCodes = targetKey ? [targetKey] : parseKeyCombination(keysInput);
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
            onClick={() => setSourceMode("local")}
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
            <div className="flex gap-2">
              <input
                type="text"
                value={pathInput}
                onChange={(e) => setPathInput(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleAddPath();
                }}
                placeholder="C:\path\to\audio.mp3"
                className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none"
              />
              <button
                onClick={handleAddPath}
                className="px-3 py-1.5 bg-accent-primary/20 text-accent-primary rounded text-sm hover:bg-accent-primary/30"
              >
                Add
              </button>
            </div>
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
                      if (e.key === "Enter" && !isDownloading) handleYoutubeDownload();
                    }}
                    placeholder="https://youtube.com/watch?v=..."
                    disabled={isDownloading}
                    className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none disabled:opacity-50"
                  />
                  <button
                    onClick={handleYoutubeDownload}
                    disabled={isDownloading || !youtubeUrl.trim()}
                    className="px-3 py-1.5 bg-accent-primary/20 text-accent-primary rounded text-sm hover:bg-accent-primary/30 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                  >
                    {isDownloading ? "..." : "Download"}
                  </button>
                </div>
                {isDownloading && (
                  <div className="space-y-1.5">
                    <div className="flex items-center gap-2">
                      <div className="w-3 h-3 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
                      <span className="text-text-secondary text-xs">
                        {downloadStatus}
                        {downloadProgress != null && ` ${Math.round(downloadProgress)}%`}
                      </span>
                    </div>
                    {downloadProgress != null && (
                      <div className="w-full bg-bg-tertiary rounded-full h-1.5 overflow-hidden">
                        <div
                          className="bg-accent-primary h-full rounded-full transition-all duration-300"
                          style={{ width: `${Math.min(100, downloadProgress)}%` }}
                        />
                      </div>
                    )}
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
                      <span className="text-accent-primary font-mono text-xs font-bold bg-bg-secondary px-1.5 py-0.5 rounded shrink-0">
                        {keysInput[i]?.toUpperCase() || "-"}
                      </span>
                    )}
                    <span className="text-text-primary text-xs flex-1 truncate">
                      {fileName}
                    </span>
                    <button
                      onClick={() => handleRemovePath(i)}
                      className="text-text-muted hover:text-accent-error text-sm shrink-0"
                    >
                      &times;
                    </button>
                  </div>
                  {/* Momentum editor */}
                  <div className="flex items-center gap-2 text-xs text-text-muted">
                    <span className="text-text-secondary whitespace-nowrap">Mom:</span>
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
                    <input
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
                    />
                    <span className="text-text-muted whitespace-nowrap">
                      {formatDuration(file.duration)}
                    </span>
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
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* Key assignment */}
        {!targetKey && (
          <div className="space-y-1">
            <label className="text-text-secondary text-sm">
              {isSingleFile ? "Key" : `Keys (${files.length} needed)`}
            </label>
            <input
              type="text"
              value={keysInput}
              onChange={(e) => handleKeyInput(e.target.value)}
              placeholder={isSingleFile ? "a" : "asdfjkl"}
              maxLength={isSingleFile ? 1 : files.length}
              className="w-full bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none font-mono tracking-widest"
            />
            {!isSingleFile && files.length > 0 && (
              <p className="text-text-muted text-xs">
                Each character maps to a file in order
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
