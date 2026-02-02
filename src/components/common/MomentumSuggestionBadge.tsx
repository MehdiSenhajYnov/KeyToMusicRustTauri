interface MomentumSuggestionBadgeProps {
  suggestedMomentum: number;
  currentMomentum: number;
  onApply: () => void;
  size?: "sm" | "md";
}

export function MomentumSuggestionBadge({
  suggestedMomentum,
  currentMomentum,
  onApply,
  size = "md",
}: MomentumSuggestionBadgeProps) {
  if (Math.abs(suggestedMomentum - currentMomentum) <= 0.3) return null;

  const isSm = size === "sm";

  return (
    <button
      onClick={onApply}
      className={`flex items-center gap-1 rounded-full
        bg-cyan-500/20 hover:bg-cyan-500/30
        border border-cyan-500/40 hover:border-cyan-500/60
        text-cyan-400 hover:text-cyan-300
        transition-all duration-200 ease-out
        font-medium group
        focus:outline-none focus:ring-2 focus:ring-cyan-500/50 focus:ring-offset-1 focus:ring-offset-transparent
        ${isSm ? "px-1.5 py-0.5 text-[9px] gap-0.5" : "px-2 py-0.5 text-[10px] gap-1.5"}`}
      title={`Momentum suggested: ${suggestedMomentum.toFixed(1)}s\nClick to apply`}
    >
      <svg
        className={`animate-pulse ${isSm ? "w-2.5 h-2.5" : "w-3 h-3"}`}
        fill="currentColor"
        viewBox="0 0 20 20"
      >
        <path d="M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.54 1.118l-2.8-2.034a1 1 0 00-1.175 0l-2.8 2.034c-.784.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.364-1.118L2.98 8.72c-.783-.57-.38-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z" />
      </svg>
      <span>{suggestedMomentum.toFixed(1)}s</span>
      <svg
        className={`opacity-60 group-hover:opacity-100 transition-opacity ${isSm ? "w-2 h-2" : "w-2.5 h-2.5"}`}
        fill="currentColor"
        viewBox="0 0 20 20"
      >
        <path
          fillRule="evenodd"
          d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
          clipRule="evenodd"
        />
      </svg>
    </button>
  );
}
