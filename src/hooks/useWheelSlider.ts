import { useRef, useEffect, useState } from "react";

interface WheelSliderOptions {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
  hoverDelayMs?: number; // Default 400ms - UX best practice to prevent accidental changes
}

/**
 * Returns classes for wheel-enabled slider visual feedback.
 * No scale (prevents layout shift) - just glow + brightness.
 */
export function getWheelActiveClass(isActive: boolean): string {
  return isActive
    ? "shadow-[0_0_10px_rgba(99,102,241,0.6)] brightness-110"
    : "";
}

export function useWheelSlider(options: WheelSliderOptions) {
  const ref = useRef<HTMLInputElement>(null);
  const optionsRef = useRef(options);
  const [isWheelActive, setIsWheelActive] = useState(false);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isActiveRef = useRef(false);

  optionsRef.current = options;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const hoverDelay = options.hoverDelayMs ?? 400;

    const handleMouseEnter = () => {
      // Start hover timer
      hoverTimerRef.current = setTimeout(() => {
        isActiveRef.current = true;
        setIsWheelActive(true);
      }, hoverDelay);
    };

    const handleMouseLeave = () => {
      // Cancel timer and deactivate immediately
      if (hoverTimerRef.current) {
        clearTimeout(hoverTimerRef.current);
        hoverTimerRef.current = null;
      }
      isActiveRef.current = false;
      setIsWheelActive(false);
    };

    const handleWheel = (e: WheelEvent) => {
      // Only handle wheel if activated by hover delay
      if (!isActiveRef.current) return;

      e.preventDefault();
      const { value, min, max, step, onChange } = optionsRef.current;
      const direction = e.deltaY < 0 ? 1 : -1;
      const raw = value + direction * step;
      // Round to step precision to avoid floating point drift
      const precision = Math.max(0, -Math.floor(Math.log10(step)));
      const rounded = parseFloat(raw.toFixed(precision));
      const clamped = Math.min(max, Math.max(min, rounded));
      onChange(clamped);
    };

    el.addEventListener("mouseenter", handleMouseEnter);
    el.addEventListener("mouseleave", handleMouseLeave);
    el.addEventListener("wheel", handleWheel, { passive: false });

    return () => {
      if (hoverTimerRef.current) clearTimeout(hoverTimerRef.current);
      el.removeEventListener("mouseenter", handleMouseEnter);
      el.removeEventListener("mouseleave", handleMouseLeave);
      el.removeEventListener("wheel", handleWheel);
    };
  }, [options.hoverDelayMs]);

  return { ref, isWheelActive };
}
