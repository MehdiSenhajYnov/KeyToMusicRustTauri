import { useRef, useEffect } from "react";

interface WheelSliderOptions {
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}

export function useWheelSlider(options: WheelSliderOptions) {
  const ref = useRef<HTMLInputElement>(null);
  const optionsRef = useRef(options);
  optionsRef.current = options;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const handler = (e: WheelEvent) => {
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

    el.addEventListener("wheel", handler, { passive: false });
    return () => el.removeEventListener("wheel", handler);
  }, []);

  return ref;
}
