import { useEffect } from "react";
import { setKeyDetectionTyping } from "../utils/tauriCommands";
import { isTextInput } from "../utils/inputHelpers";

export function useTextInputFocus() {
  useEffect(() => {
    // Typing mutes sound keys only: global shortcuts with Ctrl/Alt/Cmd keep
    // working while a field has focus.
    let typing = false;
    let timer: ReturnType<typeof setTimeout>;
    const setTyping = (next: boolean) => {
      if (typing === next) return;
      typing = next;
      void setKeyDetectionTyping(next).catch(console.error);
    };
    const sync = () => setTyping(isTextInput(document.activeElement) && document.hasFocus());
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
      setTyping(false);
    };
  }, []);
}
