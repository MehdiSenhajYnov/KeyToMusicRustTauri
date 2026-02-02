interface EmptyStateActionProps {
  icon: React.ReactNode;
  buttonText: string;
  description: string;
  onAction: () => void;
  secondaryHint?: string;
}

export function EmptyStateAction({
  icon,
  buttonText,
  description,
  onAction,
  secondaryHint,
}: EmptyStateActionProps) {
  return (
    <div className="flex-1 flex items-center justify-center animate-fadeIn">
      <div className="flex flex-col items-center gap-4 max-w-[400px] px-4">
        <div className="text-accent-primary/60">{icon}</div>
        <button
          onClick={onAction}
          className="px-6 py-3 bg-accent-primary text-white rounded-lg text-base font-medium hover:bg-accent-primary/80 transition-colors"
        >
          {buttonText}
        </button>
        <p className="text-text-muted text-sm text-center">{description}</p>
        {secondaryHint && (
          <p className="text-text-muted/60 text-xs text-center">{secondaryHint}</p>
        )}
      </div>
    </div>
  );
}
