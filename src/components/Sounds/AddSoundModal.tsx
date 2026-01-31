import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useProfileStore } from "../../stores/profileStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";
import { keyCodeToDisplay } from "../../utils/keyMapping";
import { formatDuration } from "../../utils/fileHelpers";
import * as commands from "../../utils/tauriCommands";
import { KeyCaptureSlot } from "../Keys/KeyCaptureSlot";
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
  const unlistenRef = useRef<(() => void) | null>(null);
  const downloadIdCounter = useRef(0);
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

  const tracks = currentProfile?.tracks || [];
  const isSingleFile = files.length <= 1;

  // Handle initialFiles: on mount just fetch durations, on subsequent changes append files
  useEffect(() => {
    if (!initialFiles || initialFiles.length === 0) return;
    if (processedFilesRef.current === initialFiles) {
      // Same reference (mount or StrictMode re-run): just fetch durations
      for (const path of initialFiles) {
        commands.getAudioDuration(path).then((duration) => {
          setFiles((prev) =>
            prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
          );
        }).catch(() => {});
      }
      return;
    }
    // New reference = new drop while modal is open: append files
    processedFilesRef.current = initialFiles;
    const entries: FileEntry[] = initialFiles.map((path) => ({ path, momentum: 0, duration: 0 }));
    setFiles((prev) => [...prev, ...entries]);
    for (const path of initialFiles) {
      commands.getAudioDuration(path).then((duration) => {
        setFiles((prev) =>
          prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
        );
      }).catch(() => {});
    }
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

      // Fetch durations in background
      for (const path of paths) {
        commands.getAudioDuration(path).then((duration) => {
          setFiles((prev) =>
            prev.map((f) =>
              f.path === path && f.duration === 0 ? { ...f, duration } : f
            )
          );
        }).catch(() => {});
      }
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
        currentlyPlaying: null,
        playbackPosition: 0,
        isPlaying: false,
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
                      if (e.key === "Enter" && youtubeUrl.trim()) handleYoutubeDownload();
                    }}
                    placeholder="https://youtube.com/watch?v=..."
                    className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1.5 text-sm text-text-primary focus:border-border-focus outline-none"
                  />
                  <button
                    onClick={handleYoutubeDownload}
                    disabled={!youtubeUrl.trim()}
                    className="px-3 py-1.5 bg-accent-primary/20 text-accent-primary rounded text-sm hover:bg-accent-primary/30 disabled:opacity-50 disabled:cursor-not-allowed whitespace-nowrap"
                  >
                    Download
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
