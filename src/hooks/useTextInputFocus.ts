import { useEffect } from "react";
import { acquireKeyDetectionSuspension } from "../utils/keyDetectionSuspension";
import { isTextInput } from "../utils/inputHelpers";

export function useTextInputFocus() {
  useEffect(() => {
    let release: (() => void) | undefined;
    let timer: ReturnType<typeof setTimeout>;
    const sync = () => {
      if (isTextInput(document.activeElement) && document.hasFocus()) {
        release ??= acquireKeyDetectionSuspension();
      } else {
        release?.();
        release = undefined;
      }
    };
    const deferSync = () => { clearTimeout(timer); timer = setTimeout(sync, 0); };
    document.addEventListener("focusin", sync);
    document.addEventListener("focusout", deferSync);
    window.addEventListener("blur", deferSync);
    window.addEventListener("focus", sync);
    sync();
    return () => {
      clearTimeout(timer);
      document.removeEventListener("focusin", sync);
      document.removeEventListener("focusout", deferSync);
      window.removeEventListener("blur", deferSync);
      window.removeEventListener("focus", sync);
      release?.();
    };
  }, []);
}
