import { useEffect, useRef } from "react";
import { setKeyDetection } from "../utils/tauriCommands";
import { useSettingsStore } from "../stores/settingsStore";
import { isTextInput } from "../utils/inputHelpers";

export function useTextInputFocus() {
  const isDisabled = useRef(false);

  useEffect(() => {

    const handleFocusIn = (e: FocusEvent) => {
      // Don't override if user has manually disabled key detection
      if (isTextInput(e.target) && !isDisabled.current && useSettingsStore.getState().config.keyDetectionEnabled) {
        isDisabled.current = true;
        setKeyDetection(false).catch(console.error);
      }
    };

    const handleFocusOut = (e: FocusEvent) => {
      if (!isTextInput(e.target)) return;

      // Small delay to check if focus moved to another input
      setTimeout(() => {
        const active = document.activeElement;
        if (!isTextInput(active) && isDisabled.current) {
          isDisabled.current = false;
          setKeyDetection(true).catch(console.error);
        }
      }, 50);
    };

    document.addEventListener("focusin", handleFocusIn);
    document.addEventListener("focusout", handleFocusOut);

    return () => {
      document.removeEventListener("focusin", handleFocusIn);
      document.removeEventListener("focusout", handleFocusOut);
      // Always re-enable on cleanup
      if (isDisabled.current) {
        setKeyDetection(true).catch(console.error);
      }
    };
  }, []);
}
