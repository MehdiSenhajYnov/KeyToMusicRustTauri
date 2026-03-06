import { useEffect } from "react";
import { useProfileStore } from "../stores/profileStore";
import { useHistoryStore } from "../stores/historyStore";
import { useToastStore } from "../stores/toastStore";
import { isTextInput } from "../utils/inputHelpers";

/**
 * Hook to handle Ctrl+Z (Undo) and Ctrl+Y (Redo) keyboard shortcuts.
 * Also Cmd+Z and Cmd+Shift+Z on macOS.
 */
export function useUndoRedo() {
  const undo = useProfileStore((s) => s.undo);
  const redo = useProfileStore((s) => s.redo);
  const saveCurrentProfile = useProfileStore((s) => s.saveCurrentProfile);
  const getUndoActionName = useHistoryStore((s) => s.getUndoActionName);
  const getRedoActionName = useHistoryStore((s) => s.getRedoActionName);
  const addToast = useToastStore((s) => s.addToast);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if focused on text input (but not sliders/checkboxes)
      if (isTextInput(e.target)) {
        return;
      }

      const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
      const isCtrlOrCmd = isMac ? e.metaKey : e.ctrlKey;

      if (!isCtrlOrCmd) return;

      // Undo: Ctrl+Z / Cmd+Z
      if (e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        const actionName = getUndoActionName();
        if (undo()) {
          addToast(actionName ? `Undo: ${actionName}` : "Action undone", "info");
          saveCurrentProfile();
        }
        return;
      }

      // Redo: Ctrl+Y / Cmd+Shift+Z
      if (e.key === "y" || (e.key === "z" && e.shiftKey)) {
        e.preventDefault();
        const actionName = getRedoActionName();
        if (redo()) {
          addToast(actionName ? `Redo: ${actionName}` : "Action redone", "info");
          saveCurrentProfile();
        }
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [undo, redo, getUndoActionName, getRedoActionName, addToast, saveCurrentProfile]);
}
