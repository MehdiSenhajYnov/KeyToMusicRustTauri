import { useEffect, useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { applyKeyboardLayout, getKeyboardLayoutVersion, subscribeKeyboardLayout } from "../utils/keyMapping";
import { browserKeyboard, refreshKeyboardLayout } from "../utils/keyboardLayout";

export function useKeyboardLayoutVersion() {
  return useSyncExternalStore(subscribeKeyboardLayout, getKeyboardLayoutVersion);
}

export function useKeyboardLayout() {
  useKeyboardLayoutVersion();
  useEffect(() => {
    const refresh = () => { void refreshKeyboardLayout(); };
    const keyboard = browserKeyboard();
    window.addEventListener("focus", refresh);
    keyboard?.addEventListener("layoutchange", refresh);
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen<Record<string, string>>("keyboard_layout_changed", (event) => {
      if (!disposed) applyKeyboardLayout(event.payload);
    }).then((cleanup) => { if (disposed) cleanup(); else unlisten = cleanup; }).catch(console.error);
    refresh();
    return () => {
      disposed = true;
      unlisten?.();
      window.removeEventListener("focus", refresh);
      keyboard?.removeEventListener("layoutchange", refresh);
    };
  }, []);
}
