import { useState, useEffect, useRef, useCallback } from "react";
import { formatDuration } from "../../utils/fileHelpers";
import { useWheelSlider } from "../../hooks/useWheelSlider";

interface SearchResultPreviewProps {
  streamUrl: string;
  duration: number;
  onClose: () => void;
}

export function SearchResultPreview({ streamUrl, duration, onClose }: SearchResultPreviewProps) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [isBuffering, setIsBuffering] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Create audio element on mount
  useEffect(() => {
    let cancelled = false;

    const audio = new Audio();
    // Do NOT set crossOrigin — YouTube CDN doesn't return CORS headers.
    // Without it, <audio> uses "no-cors" mode which allows playback.
    audio.preload = "auto";
    audio.src = streamUrl;
    audioRef.current = audio;

    const onTimeUpdate = () => setCurrentTime(audio.currentTime);
    const onPlaying = () => {
      if (cancelled) return;
      setIsPlaying(true);
      setIsBuffering(false);
    };
    const onPause = () => {
      if (cancelled) return;
      setIsPlaying(false);
    };
    const onEnded = () => {
      if (cancelled) return;
      setIsPlaying(false);
      setCurrentTime(0);
    };
    const onWaiting = () => {
      if (cancelled) return;
      setIsBuffering(true);
    };
    const onCanPlay = () => {
      if (cancelled) return;
      setIsBuffering(false);
    };
    const onError = () => {
      if (cancelled) return;
      const code = audio.error?.code;
      console.warn("[preview] Audio error:", code, audio.error?.message);
      if (code === MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED) {
        setError("Format not supported");
      } else if (code === MediaError.MEDIA_ERR_NETWORK) {
        setError("Network error");
      } else {
        setError("Preview unavailable");
      }
      setIsPlaying(false);
      setIsBuffering(false);
    };

    audio.addEventListener("timeupdate", onTimeUpdate);
    audio.addEventListener("playing", onPlaying);
    audio.addEventListener("pause", onPause);
    audio.addEventListener("ended", onEnded);
    audio.addEventListener("waiting", onWaiting);
    audio.addEventListener("canplay", onCanPlay);
    audio.addEventListener("error", onError);

    audio.play().catch((e) => {
      if (cancelled) return;
      // AbortError is harmless — React strict mode double-mount causes
      // first mount's play() to be interrupted by cleanup's pause()
      if (e.name === "AbortError") return;
      console.warn("[preview] play() rejected:", e);
      setIsBuffering(false);
      setError("Preview unavailable");
    });

    return () => {
      cancelled = true;
      audio.pause();
      audio.removeEventListener("timeupdate", onTimeUpdate);
      audio.removeEventListener("playing", onPlaying);
      audio.removeEventListener("pause", onPause);
      audio.removeEventListener("ended", onEnded);
      audio.removeEventListener("waiting", onWaiting);
      audio.removeEventListener("canplay", onCanPlay);
      audio.removeEventListener("error", onError);
      audio.src = "";
    };
  }, [streamUrl]);

  const handlePlayPause = useCallback(() => {
    const audio = audioRef.current;
    if (!audio) return;
    if (isPlaying) {
      audio.pause();
    } else {
      audio.play().catch(() => setError("Preview unavailable"));
    }
  }, [isPlaying]);

  const handleSeek = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const audio = audioRef.current;
    if (!audio) return;
    const time = Number(e.target.value);
    audio.currentTime = time;
    setCurrentTime(time);
  }, []);

  const handleClose = useCallback(() => {
    const audio = audioRef.current;
    if (audio) {
      audio.pause();
      audio.src = "";
    }
    onClose();
  }, [onClose]);

  const handleRetry = useCallback(() => {
    const audio = audioRef.current;
    if (!audio) return;
    setError(null);
    setIsBuffering(true);
    audio.load();
    audio.play().catch((e) => {
      if (e.name === "AbortError") return;
      setIsBuffering(false);
      setError("Preview unavailable");
    });
  }, []);

  const effectiveDuration = duration || (audioRef.current?.duration || 0);

  const seekWheelRef = useWheelSlider({
    value: currentTime, min: 0, max: effectiveDuration || 1, step: 0.5,
    onChange: (v) => {
      const audio = audioRef.current;
      if (!audio) return;
      audio.currentTime = v;
      setCurrentTime(v);
    },
  });

  if (error) {
    return (
      <div className="flex items-center gap-2 bg-bg-secondary rounded px-2 py-1.5">
        <span className="text-accent-error text-xs flex-1">{error}</span>
        <button
          onClick={handleRetry}
          className="text-accent-primary text-xs hover:underline shrink-0"
        >
          Retry
        </button>
        <button
          onClick={handleClose}
          className="text-text-muted hover:text-text-primary text-sm shrink-0 leading-none"
        >
          &times;
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 bg-bg-secondary rounded px-2 py-1.5">
      {/* Play/Pause or Loading spinner */}
      {isBuffering ? (
        <div className="w-5 h-5 flex items-center justify-center shrink-0">
          <div className="w-3 h-3 border-2 border-accent-primary border-t-transparent rounded-full animate-spin" />
        </div>
      ) : (
        <button
          onClick={handlePlayPause}
          className="w-5 h-5 flex items-center justify-center text-accent-primary hover:text-accent-primary/80 shrink-0"
          title={isPlaying ? "Pause" : "Play"}
        >
          {isPlaying ? "\u23F8" : "\u25B6"}
        </button>
      )}

      {/* Seek bar */}
      <input
        ref={seekWheelRef}
        type="range"
        min="0"
        max={effectiveDuration || 1}
        step="0.1"
        value={currentTime}
        onChange={handleSeek}
        disabled={isBuffering}
        className="flex-1 h-1 accent-accent-primary disabled:opacity-30"
      />

      {/* Timer */}
      <span className="text-text-muted text-[10px] whitespace-nowrap shrink-0 tabular-nums">
        {formatDuration(currentTime)} / {formatDuration(effectiveDuration)}
      </span>

      {/* Close button */}
      <button
        onClick={handleClose}
        className="text-text-muted hover:text-text-primary text-sm shrink-0 leading-none"
        title="Close preview"
      >
        &times;
      </button>
    </div>
  );
}
