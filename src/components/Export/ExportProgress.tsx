import { useExportStore } from "../../stores/exportStore";

export function ExportProgress() {
  const { isExporting, progress, cancelExport } = useExportStore();

  if (!isExporting || !progress) return null;

  const percentage =
    progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : 0;

  return (
    <div className="fixed bottom-4 right-4 z-50 bg-bg-secondary border border-border-color rounded-lg p-3 w-64 shadow-lg">
      <div className="flex items-center justify-between mb-2">
        <span className="text-text-primary text-sm font-medium">
          Exporting...
        </span>
        <div className="flex items-center gap-2">
          <span className="text-text-muted text-xs">
            {progress.current}/{progress.total}
          </span>
          <button
            onClick={cancelExport}
            className="text-text-muted hover:text-accent-error text-xs"
            title="Cancel export"
          >
            x
          </button>
        </div>
      </div>
      <div className="w-full h-1.5 bg-bg-tertiary rounded-full overflow-hidden">
        <div
          className="h-full bg-accent-primary rounded-full transition-all duration-200"
          style={{ width: `${percentage}%` }}
        />
      </div>
      {progress.filename && (
        <p className="text-text-muted text-xs mt-1 truncate">
          {progress.filename}
        </p>
      )}
    </div>
  );
}
