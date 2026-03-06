import { useState, useEffect } from "react";
import { useProfileStore } from "../../stores/profileStore";
import * as commands from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { formatDuration } from "../../utils/fileHelpers";

export function DislikedVideosPanel() {
  const profile = useProfileStore((s) => s.currentProfile);
  const addToast = useToastStore((s) => s.addToast);
  const [disliked, setDisliked] = useState<commands.DislikedVideoInfo[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!profile) return;
    setLoading(true);
    commands.listDislikedVideos(profile.id)
      .then(setDisliked)
      .catch(() => addToast("Failed to load disliked videos", "error"))
      .finally(() => setLoading(false));
  }, [profile?.id, addToast]);

  const handleUndislike = async (videoId: string) => {
    if (!profile) return;
    try {
      await commands.undislikeDiscovery(profile.id, videoId);
      setDisliked((prev) => prev.filter((v) => v.videoId !== videoId));
      addToast("Video removed from dislikes", "success");
    } catch {
      addToast("Failed to remove dislike", "error");
    }
  };

  if (!profile) return null;

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-sm text-text-muted">
          {loading ? "Loading..." : `${disliked.length} video(s) blacklisted`}
        </span>
      </div>

      {!loading && disliked.length === 0 && (
        <p className="text-text-muted text-xs italic">
          No disliked videos. Use the thumbs-down button in Discovery to permanently hide suggestions.
        </p>
      )}

      {!loading && disliked.length > 0 && (
        <div className="space-y-1.5 max-h-48 overflow-y-auto">
          {disliked.map((video) => (
            <div
              key={video.videoId}
              className="flex items-center justify-between gap-2 px-2 py-1.5 bg-bg-tertiary rounded border border-border-color"
            >
              <div className="flex-1 min-w-0">
                <div className="text-xs font-medium text-text-primary truncate">
                  {video.title}
                </div>
                <div className="text-[10px] text-text-muted">
                  {video.channel}{video.channel && video.duration > 0 ? " \u00B7 " : ""}{video.duration > 0 ? formatDuration(video.duration) : ""}
                </div>
              </div>
              <button
                onClick={() => handleUndislike(video.videoId)}
                className="shrink-0 px-2 py-0.5 text-[10px] bg-accent-primary/20 text-accent-primary rounded hover:bg-accent-primary/30 transition-colors"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
