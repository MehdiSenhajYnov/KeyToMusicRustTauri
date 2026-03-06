import { useToastStore } from "../../stores/toastStore";
import type { ToastType } from "../../stores/toastStore";

const typeStyles: Record<ToastType, string> = {
  success: "border-accent-success text-accent-success",
  error: "border-accent-error text-accent-error",
  warning: "border-accent-warning text-accent-warning",
  info: "border-accent-primary text-accent-primary",
};

export function ToastContainer() {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-14 right-4 z-50 space-y-2 w-72">
      {toasts.map((toast) => (
        <div
          key={toast.id}
          className={`bg-bg-secondary border-l-4 rounded px-3 py-2 flex items-start gap-2 shadow-lg animate-[slideIn_0.2s_ease-out] ${typeStyles[toast.type]}`}
        >
          <p className="text-text-primary text-sm flex-1">{toast.message}</p>
          <button
            onClick={() => removeToast(toast.id)}
            className="text-text-muted hover:text-text-primary text-xs shrink-0"
          >
            x
          </button>
        </div>
      ))}
    </div>
  );
}
