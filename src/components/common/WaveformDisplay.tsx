import { useRef, useEffect, useCallback } from "react";
import type { WaveformData } from "../../types";

interface WaveformDisplayProps {
  waveformData: WaveformData | null;
  momentum: number;
  onMomentumChange: (momentum: number) => void;
  onDragEnd?: (momentum: number) => void;
  isLoading?: boolean;
  playbackPosition?: number;
  suggestedMomentum?: number | null;
  onAcceptSuggestion?: () => void;
  height?: number;
}

export function WaveformDisplay({
  waveformData,
  momentum,
  onMomentumChange,
  onDragEnd,
  isLoading,
  playbackPosition,
  suggestedMomentum,
  onAcceptSuggestion,
  height = 60,
}: WaveformDisplayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const isDragging = useRef(false);

  const posToTime = useCallback(
    (clientX: number): number => {
      const canvas = canvasRef.current;
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
      if (!waveformData || waveformData.duration <= 0) return;
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

  // Draw waveform
  useEffect(() => {
    const canvas = canvasRef.current;
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

    // Clear
    ctx.clearRect(0, 0, w, h);

    if (!waveformData || waveformData.points.length === 0) {
      return;
    }

    const points = waveformData.points;
    const duration = waveformData.duration;

    // Draw filled area
    ctx.beginPath();
    ctx.moveTo(0, h);
    for (let i = 0; i < points.length; i++) {
      const x = (i / (points.length - 1)) * w;
      const y = h - points[i] * h * 0.9;
      ctx.lineTo(x, y);
    }
    ctx.lineTo(w, h);
    ctx.closePath();
    ctx.fillStyle = "rgba(99, 102, 241, 0.25)";
    ctx.fill();

    // Draw contour line
    ctx.beginPath();
    for (let i = 0; i < points.length; i++) {
      const x = (i / (points.length - 1)) * w;
      const y = h - points[i] * h * 0.9;
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.strokeStyle = "rgba(99, 102, 241, 0.6)";
    ctx.lineWidth = 1;
    ctx.stroke();

    // Draw suggested momentum marker (dashed line)
    if (
      suggestedMomentum != null &&
      suggestedMomentum > 0 &&
      duration > 0 &&
      Math.abs(suggestedMomentum - momentum) > 1
    ) {
      const sx = (suggestedMomentum / duration) * w;
      ctx.setLineDash([3, 3]);
      ctx.beginPath();
      ctx.moveTo(sx, 0);
      ctx.lineTo(sx, h);
      ctx.strokeStyle = "rgba(255, 255, 255, 0.3)";
      ctx.lineWidth = 1;
      ctx.stroke();
      ctx.setLineDash([]);

      // Small label
      ctx.fillStyle = "rgba(255, 255, 255, 0.5)";
      ctx.font = "9px sans-serif";
      const label = `${suggestedMomentum.toFixed(1)}s`;
      const labelW = ctx.measureText(label).width;
      const labelX = Math.min(sx + 2, w - labelW - 2);
      ctx.fillText(label, labelX, 10);
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

    // Draw playback cursor (thin green line)
    if (playbackPosition != null && playbackPosition > 0 && duration > 0) {
      const px = (playbackPosition / duration) * w;
      ctx.beginPath();
      ctx.moveTo(px, 0);
      ctx.lineTo(px, h);
      ctx.strokeStyle = "rgba(74, 222, 128, 0.8)";
      ctx.lineWidth = 1.5;
      ctx.stroke();
    }
  }, [waveformData, momentum, playbackPosition, suggestedMomentum, height]);

  if (isLoading) {
    return (
      <div
        className="w-full rounded overflow-hidden bg-bg-tertiary animate-pulse"
        style={{ height }}
      />
    );
  }

  if (!waveformData) {
    return null;
  }

  return (
    <div
      ref={containerRef}
      className="w-full relative cursor-crosshair rounded overflow-hidden"
      style={{ height }}
    >
      <canvas
        ref={canvasRef}
        className="w-full h-full"
        onMouseDown={handleMouseDown}
      />
      {/* Click hint for suggested momentum */}
      {suggestedMomentum != null &&
        suggestedMomentum > 0 &&
        onAcceptSuggestion &&
        waveformData.duration > 0 &&
        Math.abs(suggestedMomentum - momentum) > 1 && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              onAcceptSuggestion();
            }}
            className="absolute top-0.5 text-[9px] text-white/50 hover:text-white/80 bg-black/30 rounded px-1 transition-colors"
            style={{
              left: `${Math.min(
                (suggestedMomentum / waveformData.duration) * 100,
                85
              )}%`,
            }}
            title="Use suggested momentum"
          >
            Use
          </button>
        )}
    </div>
  );
}
