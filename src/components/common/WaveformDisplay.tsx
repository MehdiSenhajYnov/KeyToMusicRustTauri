import { useRef, useEffect, useCallback, useState } from "react";
import type { WaveformData } from "../../types";

// Static placeholder waveform — gentle sine combination that looks like a muted audio contour
const PLACEHOLDER_POINTS = Array.from({ length: 50 }, (_, i) => {
  const t = i / 49;
  return (
    0.25 +
    0.12 * Math.sin(t * Math.PI * 5) +
    0.08 * Math.sin(t * Math.PI * 13) +
    0.04 * Math.sin(t * Math.PI * 21)
  );
});

interface WaveformDisplayProps {
  waveformData: WaveformData | null;
  momentum?: number;
  onMomentumChange?: (momentum: number) => void;
  onDragEnd?: (momentum: number) => void;
  isLoading?: boolean;
  playbackPosition?: number;
  suggestedMomentum?: number | null;
  showSuggestionLabel?: boolean;
  height?: number;
}

export function WaveformDisplay({
  waveformData,
  momentum = 0,
  onMomentumChange,
  onDragEnd,
  isLoading,
  playbackPosition,
  suggestedMomentum,
  showSuggestionLabel = true,
  height = 60,
}: WaveformDisplayProps) {
  const staticCanvasRef = useRef<HTMLCanvasElement>(null);
  const markersCanvasRef = useRef<HTMLCanvasElement>(null);
  const cursorCanvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);
  const [isNewSuggestion, setIsNewSuggestion] = useState(false);

  useEffect(() => {
    if (suggestedMomentum != null) {
      setIsNewSuggestion(true);
      const timer = setTimeout(() => setIsNewSuggestion(false), 2000);
      return () => clearTimeout(timer);
    } else {
      setIsNewSuggestion(false);
    }
  }, [suggestedMomentum]);

  const posToTime = useCallback(
    (clientX: number): number => {
      const canvas = staticCanvasRef.current;
      if (!canvas || !waveformData || waveformData.duration <= 0) return 0;
      const rect = canvas.getBoundingClientRect();
      const x = clientX - rect.left;
      const ratio = Math.max(0, Math.min(1, x / rect.width));
      return ratio * waveformData.duration;
    },
    [waveformData]
  );

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (!waveformData || waveformData.duration <= 0 || !onMomentumChange) return;
      isDragging.current = true;
      let lastTime = Math.round(posToTime(e.clientX) * 10) / 10;
      onMomentumChange(lastTime);

      const handleMouseMove = (e: MouseEvent) => {
        if (!isDragging.current) return;
        lastTime = Math.round(posToTime(e.clientX) * 10) / 10;
        onMomentumChange(lastTime);
      };

      const handleMouseUp = () => {
        isDragging.current = false;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
        document.body.style.cursor = "";
        onDragEnd?.(lastTime);
      };

      document.body.style.cursor = "grabbing";
      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [waveformData, posToTime, onMomentumChange, onDragEnd]
  );

  // Draw static waveform shape only (no markers)
  useEffect(() => {
    const canvas = staticCanvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;

    ctx.clearRect(0, 0, w, h);

    if (!waveformData || waveformData.points.length === 0) {
      // Draw placeholder ghost waveform
      ctx.beginPath();
      ctx.moveTo(0, h);
      for (let i = 0; i < PLACEHOLDER_POINTS.length; i++) {
        const x = (i / (PLACEHOLDER_POINTS.length - 1)) * w;
        const y = h - PLACEHOLDER_POINTS[i] * h * 0.9;
        ctx.lineTo(x, y);
      }
      ctx.lineTo(w, h);
      ctx.closePath();
      ctx.fillStyle = "rgba(99, 102, 241, 0.08)";
      ctx.fill();

      ctx.beginPath();
      for (let i = 0; i < PLACEHOLDER_POINTS.length; i++) {
        const x = (i / (PLACEHOLDER_POINTS.length - 1)) * w;
        const y = h - PLACEHOLDER_POINTS[i] * h * 0.9;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = "rgba(99, 102, 241, 0.15)";
      ctx.lineWidth = 1;
      ctx.stroke();
      return;
    }

    const points = waveformData.points;
    const total = points.length;

    // Draw filled area
    ctx.beginPath();
    ctx.moveTo(0, h);
    for (let i = 0; i < total; i++) {
      const x = (i / (total - 1)) * w;
      const y = h - points[i] * h * 0.9;
      ctx.lineTo(x, y);
    }
    ctx.lineTo((total - 1) / (total - 1) * w, h);
    ctx.closePath();
    ctx.fillStyle = "rgba(99, 102, 241, 0.25)";
    ctx.fill();

    // Draw contour line
    ctx.beginPath();
    for (let i = 0; i < total; i++) {
      const x = (i / (total - 1)) * w;
      const y = h - points[i] * h * 0.9;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = "rgba(99, 102, 241, 0.6)";
    ctx.lineWidth = 1;
    ctx.stroke();
  }, [waveformData, height]);

  // Draw momentum markers on the markers canvas (middle layer)
  useEffect(() => {
    const canvas = markersCanvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;

    ctx.clearRect(0, 0, w, h);

    if (!waveformData || waveformData.points.length === 0) return;

    const duration = waveformData.duration;

    // Draw suggested momentum marker (cyan dashed line)
    if (
      suggestedMomentum != null &&
      suggestedMomentum > 0 &&
      duration > 0 &&
      Math.abs(suggestedMomentum - momentum) > 0.3
    ) {
      const sx = (suggestedMomentum / duration) * w;
      const suggestedColor = "rgba(34, 211, 238, 0.85)";

      ctx.save();
      if (isNewSuggestion) {
        ctx.shadowColor = suggestedColor;
        ctx.shadowBlur = 8;
      }

      ctx.setLineDash([4, 2]);
      ctx.beginPath();
      ctx.moveTo(sx, 0);
      ctx.lineTo(sx, h);
      ctx.strokeStyle = suggestedColor;
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.setLineDash([]);

      ctx.shadowBlur = 0;

      // Label with background for contrast (optional, hidden in compact views)
      if (showSuggestionLabel) {
        ctx.font = "bold 11px Inter, sans-serif";
        const label = `Sugg: ${suggestedMomentum.toFixed(1)}s`;
        const labelW = ctx.measureText(label).width;
        const labelX = Math.min(sx + 4, w - labelW - 4);

        ctx.fillStyle = "rgba(0, 0, 0, 0.8)";
        ctx.fillRect(labelX - 2, 2, labelW + 4, 14);

        ctx.fillStyle = suggestedColor;
        ctx.fillText(label, labelX, 13);
      }
      ctx.restore();
    }

    // Draw momentum marker (solid yellow line)
    if (duration > 0 && momentum > 0) {
      const mx = (momentum / duration) * w;
      ctx.beginPath();
      ctx.moveTo(mx, 0);
      ctx.lineTo(mx, h);
      ctx.strokeStyle = "rgba(251, 191, 36, 0.9)";
      ctx.lineWidth = 2;
      ctx.stroke();
    }
  }, [waveformData, momentum, suggestedMomentum, height, isNewSuggestion, showSuggestionLabel]);

  // Draw playback cursor on overlay canvas (lightweight, runs on progress updates)
  useEffect(() => {
    const canvas = cursorCanvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;

    ctx.clearRect(0, 0, w, h);

    if (
      playbackPosition != null &&
      playbackPosition > 0 &&
      waveformData &&
      waveformData.duration > 0
    ) {
      const px = (playbackPosition / waveformData.duration) * w;
      ctx.beginPath();
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
      ctx.strokeStyle = "rgba(74, 222, 128, 0.8)";
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }
  }, [playbackPosition, waveformData]);

  if (isLoading) {
    return (
      <div
        className="w-full rounded overflow-hidden bg-bg-tertiary animate-pulse"
        style={{ height }}
      />
    );
  }

  return (
    <div
      ref={containerRef}
      className={`w-full relative rounded overflow-hidden ${waveformData ? "cursor-crosshair" : ""}`}
      style={{ height }}
    >
      <canvas
        ref={staticCanvasRef}
        className="w-full h-full"
        onMouseDown={handleMouseDown}
      />
      <canvas
        ref={markersCanvasRef}
        className="absolute inset-0 w-full h-full pointer-events-none"
      />
      <canvas
        ref={cursorCanvasRef}
        className="absolute inset-0 w-full h-full pointer-events-none"
      />
      {/* Suggestion badge is now external (MomentumSuggestionBadge in parent) */}
    </div>
  );
}
