import { useDialog } from "../hooks/useDialog";
import { useConfirmStore } from "../stores/confirmStore";

export function ConfirmDialog() {
  const { isOpen, message, close } = useConfirmStore();

  const dialogRef = useDialog(() => close(false), isOpen);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-[100]">
      <div ref={dialogRef} role="alertdialog" aria-modal="true" aria-describedby="confirmation-message" tabIndex={-1} className="bg-bg-secondary border border-border-color rounded-lg p-5 max-w-sm w-full mx-4 shadow-xl">
        <p id="confirmation-message" className="text-text-primary text-sm mb-4">{message}</p>
        <div className="flex justify-end gap-2">
          <button
            autoFocus
            onClick={() => close(false)}
            className="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary bg-bg-tertiary hover:bg-bg-hover rounded transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => close(true)}
            className="px-3 py-1.5 text-sm text-white bg-accent-primary hover:bg-accent-primary/80 rounded transition-colors"
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}
