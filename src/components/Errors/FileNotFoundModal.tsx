import { useState } from "react";
import { useErrorStore } from "../../stores/errorStore";
import { useProfileStore } from "../../stores/profileStore";
import * as commands from "../../utils/tauriCommands";

export function FileNotFoundModal() {
  const { missingQueue, dismissCurrent, clearAll } = useErrorStore();
  const { removeSound, updateSound, saveCurrentProfile } = useProfileStore();
  const [isLocating, setIsLocating] = useState(false);
  const [isRedownloading, setIsRedownloading] = useState(false);

  if (missingQueue.length === 0) return null;

  const current = missingQueue[0];
  const remaining = missingQueue.length - 1;

  const handleLocateFile = async () => {
    setIsLocating(true);
    try {
      const newPath = await commands.pickAudioFile();
      if (newPath) {
        updateSound(current.soundId, {
          source: { type: "local", path: newPath },
        });
        saveCurrentProfile();
        dismissCurrent();
      }
    } catch (e) {
      console.error("Failed to pick audio file:", e);
    } finally {
      setIsLocating(false);
    }
  };

  const handleRedownload = async () => {
    if (!current.youtubeUrl) return;
    setIsRedownloading(true);
    try {
      const downloadId = `redownload-${current.soundId}-${Date.now()}`;
      const sound = await commands.addSoundFromYoutube(current.youtubeUrl, downloadId);
      // Update the existing sound's cached path
      updateSound(current.soundId, {
        source: {
          type: "youtube",
          url: current.youtubeUrl,
          cachedPath: sound.source.type === "youtube" ? sound.source.cachedPath : "",
        },
        duration: sound.duration,
      });
      saveCurrentProfile();
      dismissCurrent();
    } catch (e) {
      console.error("Failed to re-download:", e);
    } finally {
      setIsRedownloading(false);
    }
  };

  const handleRemove = () => {
    removeSound(current.soundId);
    saveCurrentProfile();
    dismissCurrent();
  };

  const handleSkip = () => {
    dismissCurrent();
  };

  const handleSkipAll = () => {
    clearAll();
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-bg-secondary rounded-lg shadow-xl w-[440px] max-w-[90vw] p-6">
        <h2 className="text-lg font-semibold text-text-primary mb-2">
          Sound File Not Found
        </h2>

        <div className="mb-4">
          <p className="text-sm text-text-secondary mb-1">
            <span className="font-medium text-text-primary">{current.soundName}</span>
          </p>
          <p className="text-xs text-text-tertiary break-all font-mono bg-bg-primary rounded px-2 py-1">
            {current.path}
          </p>
          {remaining > 0 && (
            <p className="text-xs text-text-tertiary mt-2">
              +{remaining} more missing {remaining === 1 ? "sound" : "sounds"}
            </p>
          )}
        </div>

        <div className="flex flex-wrap gap-2">
          {current.sourceType === "local" ? (
            <button
              onClick={handleLocateFile}
              disabled={isLocating}
              className="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded transition-colors"
            >
              {isLocating ? "Locating..." : "Locate File"}
            </button>
          ) : (
            <button
              onClick={handleRedownload}
              disabled={isRedownloading}
              className="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded transition-colors"
            >
              {isRedownloading ? "Downloading..." : "Re-download"}
            </button>
          )}

          <button
            onClick={handleRemove}
            className="px-3 py-1.5 text-sm bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded transition-colors"
          >
            Remove
          </button>

          <button
            onClick={handleSkip}
            className="px-3 py-1.5 text-sm bg-bg-primary hover:bg-bg-primary/80 text-text-secondary rounded transition-colors"
          >
            Skip
          </button>

          {remaining > 0 && (
            <button
              onClick={handleSkipAll}
              className="px-3 py-1.5 text-sm bg-bg-primary hover:bg-bg-primary/80 text-text-tertiary rounded transition-colors ml-auto"
            >
              Skip All
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
