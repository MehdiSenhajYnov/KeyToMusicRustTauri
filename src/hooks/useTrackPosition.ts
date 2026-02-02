import { useState, useEffect, useRef } from "react";
import { useAudioStore } from "../stores/audioStore";

/**
 * Polls the mutable _positions record from audioStore using requestAnimationFrame.
 * Returns a local position state that updates at most once per animation frame,
 * isolating re-renders to the component that uses this hook.
 */
export function useTrackPosition(trackId: string | undefined): number {
  const [position, setPosition] = useState(0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (!trackId) {
      setPosition(0);
      return;
    }

    const tick = () => {
      const pos = useAudioStore.getState().getPosition(trackId);
      setPosition(pos);
      rafRef.current = requestAnimationFrame(tick);
    };

    rafRef.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(rafRef.current);
  }, [trackId]);

  return position;
}
