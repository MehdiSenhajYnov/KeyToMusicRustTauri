import { invoke } from "@tauri-apps/api/core";
import { applyKeyboardLayout } from "./keyMapping";

type KeyboardAPI = EventTarget & { getLayoutMap: () => Promise<Map<string, string>> };
export function browserKeyboard(): KeyboardAPI | undefined {
  return (navigator as Navigator & { keyboard?: KeyboardAPI }).keyboard;
}

let refresh: Promise<void> | undefined;
export function refreshKeyboardLayout(): Promise<void> {
  if (refresh) return refresh;
  refresh = (async () => {
    try {
      const layout = await browserKeyboard()?.getLayoutMap();
      if (layout?.size) {
        applyKeyboardLayout(Object.fromEntries(layout));
        return;
      }
    } catch { /* WebKitGTK does not expose this API; use the native keymap. */ }
    try {
      const labels = await invoke<Record<string, string>>("get_keyboard_layout");
      applyKeyboardLayout(labels ?? {});
    } catch { /* Keep cached labels when no native bridge is available. */ }
  })().finally(() => { refresh = undefined; });
  return refresh;
}
