import { useConfirmStore } from "../stores/confirmStore";

export function ConfirmDialog() {
  const { isOpen, message, close } = useConfirmStore();

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-[100]">
      <div className="bg-bg-secondary border border-border-color rounded-lg p-5 max-w-sm w-full mx-4 shadow-xl">
        <p className="text-text-primary text-sm mb-4">{message}</p>
        <div className="flex justify-end gap-2">
          <button
            onClick={() => close(false)}
            className="px-3 py-1.5 text-sm text-text-secondary hover:text-text-primary bg-bg-tertiary hover:bg-bg-hover rounded transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={() => close(true)}
            className="px-3 py-1.5 text-sm text-white bg-accent-primary hover:bg-accent-primary/80 rounded transition-colors"
            autoFocus
          >
            Confirm
          </button>
        </div>
      </div>
    </div>
  );
}
