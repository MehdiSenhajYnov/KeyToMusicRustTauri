import { useEffect } from "react";
import { useSettingsStore } from "../../stores/settingsStore";
import { formatShortcut } from "../../utils/keyMapping";

interface KeyboardShortcutsModalProps {
  onClose: () => void;
}

const isMac = navigator.platform.toUpperCase().indexOf("MAC") >= 0;
const mod = isMac ? "Cmd" : "Ctrl";

function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-text-secondary text-xs font-semibold uppercase tracking-wide border-b border-border-color pb-1 mb-3">
      {children}
    </h3>
  );
}

function Kbd({ children }: { children: string }) {
  return (
    <kbd className="bg-bg-tertiary text-text-primary text-xs font-mono px-1.5 py-0.5 rounded border border-border-color">
      {children}
    </kbd>
  );
}

function KeyCombo({ keys }: { keys: string[] }) {
  return (
    <span className="flex items-center gap-1">
      {keys.map((k, i) => (
        <span key={i} className="flex items-center gap-1">
          {i > 0 && <span className="text-text-muted text-xs">+</span>}
          <Kbd>{k}</Kbd>
        </span>
      ))}
    </span>
  );
}

function NotSet() {
  return <span className="text-text-muted text-xs italic">(not set)</span>;
}

function ShortcutRow({ label, keys }: { label: string; keys: string[] | null }) {
  return (
    <div className="flex items-center justify-between py-1">
      <span className="text-text-secondary text-sm">{label}</span>
      {keys ? <KeyCombo keys={keys} /> : <NotSet />}
    </div>
  );
}

function momentumModifierLabel(modifier: string): string[] | null {
  if (modifier === "None") return null;
  return [modifier, "key"];
}

export function KeyboardShortcutsModal({ onClose }: KeyboardShortcutsModalProps) {
  const config = useSettingsStore((s) => s.config);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const masterStop = config.masterStopShortcut.length > 0
    ? formatShortcut(config.masterStopShortcut).split("+")
    : null;
  const autoMomentum = config.autoMomentumShortcut.length > 0
    ? formatShortcut(config.autoMomentumShortcut).split("+")
    : null;
  const keyDetection = config.keyDetectionShortcut.length > 0
    ? formatShortcut(config.keyDetectionShortcut).split("+")
    : null;
  const momentumMod = momentumModifierLabel(config.momentumModifier);

  return (
    <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50" onClick={onClose}>
      <div
        className="bg-bg-secondary border border-border-color rounded-lg w-[520px] max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-border-color shrink-0">
          <h2 className="text-text-primary font-semibold">Keyboard Shortcuts</h2>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-lg leading-none"
          >
            x
          </button>
        </div>

        {/* Scrollable content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-5">
          {/* Global Shortcuts */}
          <section>
            <SectionHeader>Global Shortcuts</SectionHeader>
            <ShortcutRow label="Master Stop" keys={masterStop} />
            <ShortcutRow label="Toggle Auto-Momentum" keys={autoMomentum} />
            <ShortcutRow label="Toggle Key Detection" keys={keyDetection} />
            <ShortcutRow label="Momentum Modifier" keys={momentumMod} />
          </section>

          {/* General */}
          <section>
            <SectionHeader>General</SectionHeader>
            <ShortcutRow label="Undo" keys={[mod, "Z"]} />
            <ShortcutRow label="Redo" keys={isMac ? [mod, "Shift", "Z"] : [mod, "Y"]} />
            <ShortcutRow label="Keyboard Shortcuts" keys={["?"]} />
          </section>

          {/* Key Grid */}
          <section>
            <SectionHeader>Key Grid</SectionHeader>
            <ShortcutRow label="Select all" keys={[mod, "A"]} />
            <ShortcutRow label="Multi-select" keys={[mod, "Click"]} />
            <ShortcutRow label="Range select" keys={["Shift", "Click"]} />
            <ShortcutRow label="Deselect" keys={["Click selected key"]} />
          </section>

          {/* Discovery */}
          <section>
            <SectionHeader>Discovery</SectionHeader>
            <ShortcutRow label="Previous suggestion" keys={["\u2190"]} />
            <ShortcutRow label="Next suggestion" keys={["\u2192"]} />
          </section>

          {/* Modals */}
          <section>
            <SectionHeader>Modals</SectionHeader>
            <ShortcutRow label="Close / Cancel" keys={["Esc"]} />
            <ShortcutRow label="Submit YouTube URL" keys={["Enter"]} />
          </section>

          {/* Mouse Interactions */}
          <section>
            <SectionHeader>Mouse Interactions</SectionHeader>
            <ShortcutRow label="Adjust sliders" keys={["Mouse wheel"]} />
            <ShortcutRow label="Set momentum" keys={["Drag on waveform"]} />
            <ShortcutRow label="Resize panel" keys={["Drag divider bar"]} />
            <ShortcutRow label="Add files" keys={["Drag & drop audio"]} />
          </section>
        </div>

        {/* Footer */}
        <div className="flex justify-center p-3 border-t border-border-color shrink-0">
          <span className="text-text-muted text-xs">
            Tip: press <Kbd>?</Kbd> to toggle this modal
          </span>
        </div>
      </div>
    </div>
  );
}
