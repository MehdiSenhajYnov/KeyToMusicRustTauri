import { stopAllSounds } from "../../utils/tauriCommands";

export function StopAllButton() {
  const handleStop = async () => {
    try {
      await stopAllSounds();
    } catch (e) {
      console.error("Failed to stop all sounds:", e);
    }
  };

  return (
    <button
      onClick={handleStop}
      className="w-full py-2 bg-accent-error/20 hover:bg-accent-error/40 text-accent-error font-semibold text-sm rounded transition-colors active:scale-95"
    >
      Stop All
    </button>
  );
}
