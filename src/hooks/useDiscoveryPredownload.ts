import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDiscoveryStore } from "../stores/discoveryStore";
import * as commands from "../utils/tauriCommands";

/**
 * Pre-downloads a window of suggestions around the current carousel index.
 * Downloads [current-2, ..., current+3] with max 3 concurrent downloads.
 * Asymmetric window (3 ahead, 2 behind) since users mostly navigate forward.
 * Also pre-downloads the first 2 items that would appear on refresh (unseen pool items).
 */
export function useDiscoveryPredownload() {
  const currentIndex = useDiscoveryStore((s) => s.currentIndex);
  const visibleSuggestions = useDiscoveryStore((s) => s.visibleSuggestions);
  const allSuggestions = useDiscoveryStore((s) => s.allSuggestions);
  const revealedCount = useDiscoveryStore((s) => s.revealedCount);
  const setPredownloadStatus = useDiscoveryStore(
    (s) => s.setPredownloadStatus
  );
  const updateDownloadProgress = useDiscoveryStore(
    (s) => s.updateDownloadProgress
  );
  const updateSuggestionAssignment = useDiscoveryStore(
    (s) => s.updateSuggestionAssignment
  );
  const setPoolPredownload = useDiscoveryStore(
    (s) => s.setPoolPredownload
  );

  const activeDownloads = useRef(new Set<string>());
  const dlCounter = useRef(0);

  // Listen for download progress events for pre-downloads
  useEffect(() => {
    const unlisten = listen<{
      downloadId: string;
      status: string;
      progress: number | null;
    }>("youtube_download_progress", (event) => {
      const { downloadId, progress } = event.payload;
      if (!downloadId.startsWith("predl_")) return;

      // Find which suggestion has this downloadId
      const store = useDiscoveryStore.getState();
      const suggestion = store.visibleSuggestions.find(
        (s) => s.downloadId === downloadId
      );
      if (suggestion && progress != null) {
        updateDownloadProgress(suggestion.videoId, progress);
      }
    });

    return () => {
      unlisten.then((f) => f());
    };
  }, [updateDownloadProgress]);

  // Pre-download window around current index
  useEffect(() => {
    if (visibleSuggestions.length === 0) return;

    const indices = [
      currentIndex - 2,
      currentIndex - 1,
      currentIndex,
      currentIndex + 1,
      currentIndex + 2,
      currentIndex + 3,
    ].filter((i) => i >= 0 && i < visibleSuggestions.length);

    for (const idx of indices) {
      const s = visibleSuggestions[idx];
      if (
        !s ||
        s.predownloadStatus !== "idle" ||
        activeDownloads.current.size >= 3
      ) {
        continue;
      }

      const downloadId = `predl_${Date.now()}_${dlCounter.current++}`;
      activeDownloads.current.add(s.videoId);

      setPredownloadStatus(s.videoId, "downloading", { downloadId });

      commands
        .predownloadSuggestion(s.url, s.videoId, downloadId)
        .then((result) => {
          activeDownloads.current.delete(s.videoId);
          setPredownloadStatus(s.videoId, "ready", {
            cachedPath: result.cachedPath,
            waveform: result.waveform,
            duration: result.duration,
          });
          // Update momentum from waveform suggestion
          if (result.waveform.suggestedMomentum != null) {
            updateSuggestionAssignment(s.videoId, {
              suggestedMomentum: result.waveform.suggestedMomentum,
            });
          }
        })
        .catch(() => {
          activeDownloads.current.delete(s.videoId);
          setPredownloadStatus(s.videoId, "error");
        });
    }
  }, [
    currentIndex,
    visibleSuggestions,
    setPredownloadStatus,
    updateSuggestionAssignment,
  ]);

  // Pre-download first 2 items that would appear on refresh (unseen pool items).
  // Uses revealedCount as offset because handleGenerate shows allSuggestions[revealedCount..] on refresh.
  useEffect(() => {
    const store = useDiscoveryStore.getState();
    const refreshStart = revealedCount;

    for (let i = 0; i < 2; i++) {
      const poolIdx = refreshStart + i;
      if (poolIdx >= allSuggestions.length) break;

      const s = allSuggestions[poolIdx];
      // Skip if already visible (will be handled by the main window)
      if (visibleSuggestions.some((v) => v.videoId === s.videoId)) continue;
      // Skip if already predownloaded in pool cache
      if (store.poolPredownloads[s.videoId]) continue;
      // Skip if already active
      if (activeDownloads.current.has(s.videoId)) continue;
      // Respect concurrency limit
      if (activeDownloads.current.size >= 3) break;

      const downloadId = `predl_${Date.now()}_${dlCounter.current++}`;
      activeDownloads.current.add(s.videoId);

      commands
        .predownloadSuggestion(s.url, s.videoId, downloadId)
        .then((result) => {
          activeDownloads.current.delete(s.videoId);
          setPoolPredownload(s.videoId, {
            cachedPath: result.cachedPath,
            waveform: result.waveform,
            duration: result.duration,
            suggestedMomentum: result.waveform.suggestedMomentum ?? 0,
          });
        })
        .catch(() => {
          activeDownloads.current.delete(s.videoId);
        });
    }
  }, [revealedCount, allSuggestions, visibleSuggestions, setPoolPredownload]);
}
