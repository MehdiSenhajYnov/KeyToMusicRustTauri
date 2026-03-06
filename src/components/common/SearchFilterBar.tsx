import { useState, useRef, useCallback, forwardRef, useImperativeHandle } from "react";
import type { KeyGridFilter, Track, LoopMode, BaseMood } from "../../types";
import { MOOD_CATEGORIES } from "../../utils/moodHelpers";

interface FilterChip {
  type: "track" | "loop" | "status" | "mood";
  value: string;
  label: string;
}

interface SearchFilterBarProps {
  totalCount: number;
  matchCount: number;
  onFilterChange: (filter: KeyGridFilter | null) => void;
  tracks: Track[];
}

export interface SearchFilterBarHandle {
  focus: () => void;
}

const LOOP_MODES: LoopMode[] = ["off", "random", "single", "sequential"];
const STATUS_VALUES = ["playing", "stopped"];

export const SearchFilterBar = forwardRef<SearchFilterBarHandle, SearchFilterBarProps>(
  function SearchFilterBar({ totalCount, matchCount, onFilterChange }, ref) {
    const [inputValue, setInputValue] = useState("");
    const [chips, setChips] = useState<FilterChip[]>([]);
    const [isActive, setIsActive] = useState(false);
    const inputRef = useRef<HTMLInputElement>(null);

    useImperativeHandle(ref, () => ({
      focus: () => {
        setIsActive(true);
        inputRef.current?.focus();
      },
    }));

    const buildFilter = useCallback(
      (text: string, currentChips: FilterChip[]): KeyGridFilter | null => {
        const trackChip = currentChips.find((c) => c.type === "track");
        const loopChip = currentChips.find((c) => c.type === "loop");
        const statusChip = currentChips.find((c) => c.type === "status");
        const moodChip = currentChips.find((c) => c.type === "mood");

        if (!text && currentChips.length === 0) return null;

        return {
          searchText: text,
          trackName: trackChip?.value ?? null,
          loopMode: (loopChip?.value as LoopMode) ?? null,
          status: (statusChip?.value as "playing" | "stopped") ?? null,
          mood: (moodChip?.value as BaseMood) ?? null,
          intensity: null, // TODO: parse m:mood:intensity format
        };
      },
      []
    );

    const handleInputChange = (rawValue: string) => {
      // Parse tokens to detect prefixes
      const tokens = rawValue.split(/\s+/);
      const newChips = [...chips];
      const remaining: string[] = [];

      for (const token of tokens) {
        if (!token) continue;

        const lower = token.toLowerCase();

        if (lower.startsWith("t:") && token.length > 2) {
          const val = token.slice(2);
          // Remove existing track chip
          const idx = newChips.findIndex((c) => c.type === "track");
          if (idx !== -1) newChips.splice(idx, 1);
          newChips.push({ type: "track", value: val, label: `t:${val}` });
        } else if (lower.startsWith("l:") && token.length > 2) {
          const val = token.slice(2).toLowerCase();
          if (LOOP_MODES.includes(val as LoopMode)) {
            const idx = newChips.findIndex((c) => c.type === "loop");
            if (idx !== -1) newChips.splice(idx, 1);
            newChips.push({ type: "loop", value: val, label: `l:${val}` });
          } else {
            remaining.push(token);
          }
        } else if (lower.startsWith("s:") && token.length > 2) {
          const val = token.slice(2).toLowerCase();
          if (STATUS_VALUES.includes(val)) {
            const idx = newChips.findIndex((c) => c.type === "status");
            if (idx !== -1) newChips.splice(idx, 1);
            newChips.push({ type: "status", value: val, label: `s:${val}` });
          } else {
            remaining.push(token);
          }
        } else if (lower.startsWith("m:") && token.length > 2) {
          const val = token.slice(2).toLowerCase();
          if ((MOOD_CATEGORIES as readonly string[]).includes(val)) {
            const idx = newChips.findIndex((c) => c.type === "mood");
            if (idx !== -1) newChips.splice(idx, 1);
            newChips.push({ type: "mood", value: val, label: `m:${val}` });
          } else {
            remaining.push(token);
          }
        } else {
          remaining.push(token);
        }
      }

      const newText = remaining.join(" ");

      // Only update chips state if array contents actually changed
      const chipsChanged = newChips.length !== chips.length ||
        newChips.some((c, i) => c.type !== chips[i].type || c.value !== chips[i].value);
      if (chipsChanged) setChips(newChips);

      setInputValue(newText);
      onFilterChange(buildFilter(newText, chipsChanged ? newChips : chips));
    };

    const removeChip = (index: number) => {
      const newChips = chips.filter((_, i) => i !== index);
      setChips(newChips);
      onFilterChange(buildFilter(inputValue, newChips));
      inputRef.current?.focus();
    };

    const clearAll = () => {
      setInputValue("");
      setChips([]);
      setIsActive(false);
      onFilterChange(null);
      inputRef.current?.blur();
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        clearAll();
      }
      // Backspace on empty input removes last chip
      if (e.key === "Backspace" && !inputValue && chips.length > 0) {
        removeChip(chips.length - 1);
      }
    };

    const hasFilter = inputValue || chips.length > 0;

    // Don't render the full bar when inactive and no filter
    if (!isActive && !hasFilter) {
      return (
        <button
          onClick={() => {
            setIsActive(true);
            // Focus after state update
            setTimeout(() => inputRef.current?.focus(), 0);
          }}
          className="flex items-center gap-1.5 px-2 py-1 text-text-muted text-xs rounded hover:bg-bg-tertiary transition-colors"
          aria-label="Search key bindings"
        >
          <svg
            className="w-3.5 h-3.5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <span className="hidden sm:inline">Ctrl+F</span>
        </button>
      );
    }

    return (
      <div className="flex-1 max-w-md min-w-0">
        <div className="flex items-center gap-1.5 bg-bg-tertiary border border-border-color rounded px-2 py-1 focus-within:border-border-focus transition-colors">
          {/* Search icon */}
          <svg
            className="w-3.5 h-3.5 text-text-muted shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>

          {/* Input */}
          <input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => handleInputChange(e.target.value)}
            onKeyDown={handleKeyDown}
            onBlur={() => {
              if (!hasFilter) setIsActive(false);
            }}
            placeholder="Search keys, sounds, tracks..."
            className="flex-1 bg-transparent text-sm text-text-primary placeholder-text-muted outline-none min-w-0"
            aria-label="Search key bindings"
          />

          {/* Counter */}
          {hasFilter && (
            <span className="text-text-muted text-xs whitespace-nowrap" role="status">
              {matchCount}/{totalCount}
            </span>
          )}

          {/* Clear button */}
          {hasFilter && (
            <button
              onClick={clearAll}
              className="text-text-muted hover:text-text-primary p-0.5 shrink-0"
              aria-label="Clear search"
            >
              <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          )}
        </div>

        {/* Filter chips */}
        {chips.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-1">
            {chips.map((chip, i) => (
              <span
                key={`${chip.type}-${chip.value}`}
                className="inline-flex items-center gap-1 bg-accent-primary/20 text-accent-primary text-xs rounded-full px-2 py-0.5"
              >
                {chip.label}
                <button
                  onClick={() => removeChip(i)}
                  className="hover:text-accent-primary/70"
                  aria-label={`Remove filter: ${chip.label}`}
                >
                  <svg className="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                  </svg>
                </button>
              </span>
            ))}
          </div>
        )}
      </div>
    );
  }
);
