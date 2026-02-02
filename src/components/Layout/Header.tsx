import { useRef, useCallback } from "react";
import { useSettingsStore } from "../../stores/settingsStore";
import { useWheelSlider, getWheelActiveClass } from "../../hooks/useWheelSlider";

interface HeaderProps {
  onSettingsClick: () => void;
  onHelpClick: () => void;
}

export function Header({ onSettingsClick, onHelpClick }: HeaderProps) {
  const { config, setMasterVolume } = useSettingsStore();
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleVolumeChange = useCallback((volume: number) => {
    // Update store immediately for responsive UI
    useSettingsStore.setState((state) => ({
      config: { ...state.config, masterVolume: volume },
    }));
    // Debounce the backend call
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setMasterVolume(volume);
    }, 100);
  }, [setMasterVolume]);

  const { ref: masterVolWheelRef, isWheelActive: masterVolWheelActive } = useWheelSlider({
    value: Math.round(config.masterVolume * 100),
    min: 0, max: 100, step: 1,
    onChange: (v) => handleVolumeChange(v / 100),
  });

  return (
    <header className="h-12 bg-bg-secondary border-b border-border-color flex items-center px-4 gap-4 shrink-0">
      <div className="flex items-center gap-2">
        <span className="text-accent-primary font-bold text-lg">KTM</span>
        <span className="text-text-secondary text-sm hidden sm:inline">
          KeyToMusic
        </span>
      </div>

      <div className="flex-1" />

      <div className="flex items-center gap-2">
        <span className="text-text-muted text-xs">Vol</span>
        <input
          ref={masterVolWheelRef}
          type="range"
          min="0"
          max="100"
          value={Math.round(config.masterVolume * 100)}
          onChange={(e) => handleVolumeChange(Number(e.target.value) / 100)}
          className={`w-24 h-1 accent-accent-primary transition-all duration-200 ${getWheelActiveClass(masterVolWheelActive)}`}
        />
        <span className="text-text-secondary text-xs w-8">
          {Math.round(config.masterVolume * 100)}%
        </span>
      </div>

      <button
        onClick={onHelpClick}
        className="p-2 text-text-secondary hover:text-text-primary hover:bg-bg-hover rounded transition-colors"
        title="Keyboard Shortcuts"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
            d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01" />
          <circle cx="12" cy="12" r="10" strokeWidth={2} />
        </svg>
      </button>

      <button
        onClick={onSettingsClick}
        className="p-2 text-text-secondary hover:text-text-primary hover:bg-bg-hover rounded transition-colors"
        title="Settings"
      >
        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </button>
    </header>
  );
}
