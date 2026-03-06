import { useState } from "react";

interface WarningTooltipProps {
  message: string;
  className?: string;
}

export function WarningTooltip({ message, className = "" }: WarningTooltipProps) {
  const [showTooltip, setShowTooltip] = useState(false);

  return (
    <div
      className={`relative inline-flex ${className}`}
      onMouseEnter={() => setShowTooltip(true)}
      onMouseLeave={() => setShowTooltip(false)}
    >
      {/* Warning Icon */}
      <svg
        className="w-4 h-4 text-amber-500 cursor-help"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={2}
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
        />
      </svg>

      {/* Tooltip */}
      {showTooltip && (
        <div className="absolute z-50 bottom-full left-1/2 -translate-x-1/2 mb-2 px-3 py-2 text-xs text-text-primary bg-bg-tertiary border border-border-color rounded shadow-lg whitespace-normal max-w-[250px] min-w-[200px]">
          {message}
          {/* Arrow */}
          <div className="absolute top-full left-1/2 -translate-x-1/2 -mt-px">
            <div className="border-4 border-transparent border-t-border-color" />
          </div>
        </div>
      )}
    </div>
  );
}
