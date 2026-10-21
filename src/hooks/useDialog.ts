import { useEffect, useRef } from "react";
import { acquireKeyDetectionSuspension } from "../utils/keyDetectionSuspension";

const dialogs: HTMLElement[] = [];

/** Keep focus and shortcuts inside the topmost dialog, then restore focus. */
export function useDialog(onClose: () => void, open = true) {
  const ref = useRef<HTMLDivElement>(null);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    const dialog = ref.current;
    if (!open || !dialog) return;
    const previous = document.activeElement;
    const release = acquireKeyDetectionSuspension();
    dialogs.push(dialog);
    const focusable = () => Array.from(dialog.querySelectorAll<HTMLElement>(
      'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex="0"]'
    )).filter((element) => element.getClientRects().length > 0);
    if (!dialog.contains(document.activeElement)) (focusable()[0] ?? dialog).focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (dialogs[dialogs.length - 1] !== dialog || event.defaultPrevented) return;
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopImmediatePropagation();
        closeRef.current();
      } else if (event.key === "Tab") {
        const elements = focusable();
        const index = elements.indexOf(document.activeElement as HTMLElement);
        if (elements.length === 0 || index < 0 || (event.shiftKey ? index === 0 : index === elements.length - 1)) {
          event.preventDefault();
          (elements[event.shiftKey ? elements.length - 1 : 0] ?? dialog).focus();
        }
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      dialogs.splice(dialogs.indexOf(dialog), 1);
      release();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, [open]);
  return ref;
}
