import { useSettingsStore } from "../../stores/settingsStore";

export function GlobalToggles() {
  const { config, toggleAutoMomentum, toggleKeyDetection } = useSettingsStore();

  return (
    <div className="space-y-2">
      <Toggle
        label="Auto-Momentum"
        enabled={config.autoMomentum}
        onToggle={toggleAutoMomentum}
      />
      <Toggle
        label="Key Detection"
        enabled={config.keyDetectionEnabled}
        onToggle={toggleKeyDetection}
      />
    </div>
  );
}

function Toggle({
  label,
  enabled,
  onToggle,
}: {
  label: string;
  enabled: boolean;
  onToggle: () => void;
}) {
  return (
    <button
      onClick={onToggle}
      className="w-full flex items-center justify-between px-2 py-1.5 rounded hover:bg-bg-hover transition-colors"
    >
      <span className="text-text-secondary text-sm">{label}</span>
      <span
        className={`w-8 h-4 rounded-full relative transition-colors ${
          enabled ? "bg-accent-success" : "bg-bg-tertiary"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-3 h-3 rounded-full bg-white transition-transform ${
            enabled ? "translate-x-4" : "translate-x-0"
          }`}
        />
      </span>
    </button>
  );
}
