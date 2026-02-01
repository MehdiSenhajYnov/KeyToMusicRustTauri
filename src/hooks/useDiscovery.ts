import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useDiscoveryStore } from "../stores/discoveryStore";

export function useDiscovery() {
  const setProgress = useDiscoveryStore((s) => s.setProgress);
  const setGenerating = useDiscoveryStore((s) => s.setGenerating);
  const setError = useDiscoveryStore((s) => s.setError);

  useEffect(() => {
    const unlisteners: Promise<() => void>[] = [];

    unlisteners.push(
      listen<Record<string, never>>("discovery_started", () => {
        setGenerating(true);
      })
    );

    unlisteners.push(
      listen<{ current: number; total: number; seedName: string }>(
        "discovery_progress",
        (event) => {
          setProgress(event.payload);
        }
      )
    );

    unlisteners.push(
      listen<{ count: number }>("discovery_complete", () => {
        setGenerating(false);
        setProgress(null);
      })
    );

    unlisteners.push(
      listen<{ message: string }>("discovery_error", (event) => {
        setError(event.payload.message);
      })
    );

    return () => {
      for (const p of unlisteners) {
        p.then((f) => f());
      }
    };
  }, [setProgress, setGenerating, setError]);
}
