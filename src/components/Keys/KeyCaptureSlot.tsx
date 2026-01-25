import { useState, useRef, useEffect, useCallback } from "react";
import {
  keyCodeToDisplay,
  charToKeyCode,
  recordKeyLayout,
  buildComboFromPressedKeys,
  checkShortcutConflicts,
  ShortcutConflict,
  ShortcutConflictConfig,
} from "../../utils/keyMapping";

interface KeyCaptureSlotProps {
  /** Current key code value (e.g., "KeyA", "Ctrl+Shift+KeyB") */
  value: string;
  /** Called when a valid key is captured */
  onChange: (keyCode: string) => void;
  /** Called when the slot is removed */
  onRemove?: () => void;
  /** Placeholder text when empty */
  placeholder?: string;
  /** Whether the slot can be removed */
  removable?: boolean;
  /** Configuration for conflict checking */
  conflictConfig?: ShortcutConflictConfig;
  /** Index for display (1-based) */
  index?: number;
  /** Whether this is disabled */
  disabled?: boolean;
}

export function KeyCaptureSlot({
  value,
  onChange,
  onRemove,
  placeholder = "Click to assign",
  removable = true,
  conflictConfig,
  index,
  disabled = false,
}: KeyCaptureSlotProps) {
  const [isCapturing, setIsCapturing] = useState(false);
  const [capturedDisplay, setCapturedDisplay] = useState("");
  const [conflict, setConflict] = useState<ShortcutConflict | null>(null);
  const pressedKeysRef = useRef<Set<string>>(new Set());
  const slotRef = useRef<HTMLDivElement>(null);

  // Reset conflict when value changes externally
  useEffect(() => {
    setConflict(null);
  }, [value]);

  const startCapture = useCallback(() => {
    if (disabled) return;
    setIsCapturing(true);
    setCapturedDisplay("");
    setConflict(null);
    pressedKeysRef.current.clear();
  }, [disabled]);

  const cancelCapture = useCallback(() => {
    setIsCapturing(false);
    setCapturedDisplay("");
    setConflict(null);
    pressedKeysRef.current.clear();
  }, []);

  const finishCapture = useCallback(
    (combo: string) => {
      // Check for conflicts
      const conflictResult = checkShortcutConflicts(combo, conflictConfig);

      if (conflictResult?.type === "error") {
        // Show error, don't accept the key
        setConflict(conflictResult);
        setCapturedDisplay(keyCodeToDisplay(combo));
        // Keep capturing mode open so user can try again
        pressedKeysRef.current.clear();
        return;
      }

      if (conflictResult?.type === "warning") {
        // Show warning but accept the key
        setConflict(conflictResult);
      }

      // Accept the key
      onChange(combo);
      setIsCapturing(false);
      setCapturedDisplay("");
      pressedKeysRef.current.clear();
    },
    [onChange, conflictConfig]
  );

  // Handle keyboard events
  useEffect(() => {
    if (!isCapturing) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      // Escape to cancel
      if (e.code === "Escape") {
        cancelCapture();
        return;
      }

      // Get key code (layout-aware)
      const code = charToKeyCode(e.key) || e.code;
      recordKeyLayout(code, e.key);

      // Add to pressed keys
      pressedKeysRef.current.add(code);

      // Build and display current combo
      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (combo) {
        setCapturedDisplay(keyCodeToDisplay(combo));
        // Clear any previous conflict when typing new keys
        setConflict(null);
      }
    };

    const handleKeyUp = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();

      const code = charToKeyCode(e.key) || e.code;

      // When a key is released, finalize the capture if we have a valid combo
      const combo = buildComboFromPressedKeys(pressedKeysRef.current);
      if (combo) {
        finishCapture(combo);
      }

      // Remove from pressed keys
      pressedKeysRef.current.delete(code);
    };

    // Handle click outside to cancel
    const handleClickOutside = (e: MouseEvent) => {
      if (slotRef.current && !slotRef.current.contains(e.target as Node)) {
        cancelCapture();
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("keyup", handleKeyUp, true);
    document.addEventListener("mousedown", handleClickOutside);

    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("keyup", handleKeyUp, true);
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isCapturing, cancelCapture, finishCapture]);

  // Display value
  const displayValue = value ? keyCodeToDisplay(value) : "";

  return (
    <div ref={slotRef} className="flex flex-col gap-1">
      <div className="flex items-center gap-2">
        {/* Index badge */}
        {index !== undefined && (
          <span className="text-xs text-text-muted w-4 text-right">
            {index}.
          </span>
        )}

        {/* Main slot button */}
        <button
          type="button"
          onClick={isCapturing ? undefined : startCapture}
          disabled={disabled}
          className={`
            flex-1 px-3 py-2 rounded text-sm text-left transition-all
            ${
              disabled
                ? "bg-bg-tertiary text-text-muted cursor-not-allowed"
                : isCapturing
                ? "bg-accent-primary/20 border-2 border-accent-primary text-text-primary ring-2 ring-accent-primary/30"
                : value
                ? "bg-bg-tertiary text-text-primary hover:bg-bg-hover border border-border-color"
                : "bg-bg-tertiary text-text-muted hover:bg-bg-hover border border-dashed border-border-color"
            }
            ${conflict?.type === "error" ? "border-accent-error" : ""}
          `}
        >
          {isCapturing ? (
            <span className="flex items-center gap-2">
              {capturedDisplay ? (
                <>
                  <span className="font-mono font-medium">{capturedDisplay}</span>
                  <span className="text-text-muted text-xs">Release to confirm</span>
                </>
              ) : (
                <span className="text-text-muted animate-pulse">
                  Press a key...
                </span>
              )}
            </span>
          ) : displayValue ? (
            <span className="font-mono font-medium">{displayValue}</span>
          ) : (
            <span>{placeholder}</span>
          )}
        </button>

        {/* Remove button */}
        {removable && onRemove && !isCapturing && (
          <button
            type="button"
            onClick={onRemove}
            className="p-1.5 text-text-muted hover:text-accent-error rounded transition-colors"
            title="Remove"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}

        {/* Cancel button during capture */}
        {isCapturing && (
          <button
            type="button"
            onClick={cancelCapture}
            className="px-2 py-1 text-xs text-text-muted hover:text-text-primary rounded transition-colors"
          >
            Esc
          </button>
        )}
      </div>

      {/* Conflict message */}
      {conflict && (
        <div
          className={`text-xs px-2 py-1 rounded ${
            conflict.type === "error"
              ? "bg-accent-error/10 text-accent-error"
              : "bg-accent-warning/10 text-accent-warning"
          }`}
        >
          {conflict.type === "error" ? (
            <>
              <span className="font-medium">Cannot use:</span> {conflict.message}
            </>
          ) : (
            <>
              <span className="font-medium">Warning:</span> {conflict.message}
            </>
          )}
        </div>
      )}
    </div>
  );
}
