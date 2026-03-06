import { useProfileStore } from "../../stores/profileStore";
import { useConfirmStore } from "../../stores/confirmStore";
import { useHistoryStore, captureProfileState } from "../../stores/historyStore";
import { keyCodeToDisplay } from "../../utils/keyMapping";
import type { LoopMode } from "../../types";

interface MultiKeyDetailsProps {
  selectedKeys: Set<string>;
  onClose: () => void;
}

export function MultiKeyDetails({ selectedKeys, onClose }: MultiKeyDetailsProps) {
  const currentProfile = useProfileStore((s) => s.currentProfile);
  const saveCurrentProfile = useProfileStore((s) => s.saveCurrentProfile);
  const showConfirm = useConfirmStore((s) => s.confirm);

  if (!currentProfile) return null;

  const bindings = currentProfile.keyBindings.filter((kb) =>
    selectedKeys.has(kb.keyCode)
  );

  if (bindings.length === 0) return null;

  const count = selectedKeys.size;

  // Track: check if all same
  const trackIds = bindings.map((kb) => kb.trackId);
  const allSameTrack = trackIds.every((id) => id === trackIds[0]);
  const commonTrackId = allSameTrack ? trackIds[0] : null;

  // Loop mode: check if all same
  const loopModes = bindings.map((kb) => kb.loopMode);
  const allSameLoop = loopModes.every((m) => m === loopModes[0]);
  const commonLoopMode = allSameLoop ? loopModes[0] : null;

  const handleTrackChange = (newTrackId: string) => {
    const profile = useProfileStore.getState().currentProfile!;
    const before = captureProfileState(profile);

    // Direct state update to produce a single history entry
    useProfileStore.setState({
      currentProfile: {
        ...profile,
        keyBindings: profile.keyBindings.map((kb) =>
          selectedKeys.has(kb.keyCode) ? { ...kb, trackId: newTrackId } : kb
        ),
      },
    });

    const after = captureProfileState(useProfileStore.getState().currentProfile!);
    useHistoryStore.getState().pushState(
      `Change track for ${count} keys`,
      before,
      after
    );

    saveCurrentProfile();
  };

  const handleLoopModeChange = (newMode: LoopMode) => {
    const profile = useProfileStore.getState().currentProfile!;
    const before = captureProfileState(profile);

    useProfileStore.setState({
      currentProfile: {
        ...profile,
        keyBindings: profile.keyBindings.map((kb) =>
          selectedKeys.has(kb.keyCode) ? { ...kb, loopMode: newMode, currentIndex: 0 } : kb
        ),
      },
    });

    const after = captureProfileState(useProfileStore.getState().currentProfile!);
    useHistoryStore.getState().pushState(
      `Change loop mode for ${count} keys`,
      before,
      after
    );

    saveCurrentProfile();
  };

  const handleDeleteAll = async () => {
    if (!await showConfirm(`Delete ${count} key bindings?`)) return;

    const profile = useProfileStore.getState().currentProfile!;
    const before = captureProfileState(profile);

    const remainingBindings = profile.keyBindings.filter(
      (kb) => !selectedKeys.has(kb.keyCode)
    );
    const referencedSoundIds = new Set(
      remainingBindings.flatMap((kb) => kb.soundIds)
    );

    useProfileStore.setState({
      currentProfile: {
        ...profile,
        keyBindings: remainingBindings,
        sounds: profile.sounds.filter((s) => referencedSoundIds.has(s.id)),
      },
    });

    const after = captureProfileState(useProfileStore.getState().currentProfile!);
    useHistoryStore.getState().pushState(
      `Delete ${count} keys`,
      before,
      after
    );

    saveCurrentProfile();
    onClose();
  };

  return (
    <div className="p-4 space-y-3">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h3 className="text-text-primary text-sm font-semibold">
          {count} keys selected
        </h3>
        <div className="flex items-center gap-2">
          <button
            onClick={handleDeleteAll}
            className="text-accent-error hover:text-accent-error/80 text-xs"
          >
            Delete {count} keys
          </button>
          <button
            onClick={onClose}
            className="text-text-muted hover:text-text-primary text-sm"
          >
            Close
          </button>
        </div>
      </div>

      {/* Selected keys list */}
      <div className="flex flex-wrap gap-1">
        {[...selectedKeys].map((keyCode) => (
          <span
            key={keyCode}
            className="text-accent-primary font-mono text-xs font-bold bg-bg-tertiary px-1.5 py-0.5 rounded"
          >
            {keyCodeToDisplay(keyCode)}
          </span>
        ))}
      </div>

      {/* Track & Loop controls */}
      <div className="flex items-center gap-4 flex-wrap">
        <div className="flex items-center gap-2">
          <span className="text-text-muted text-xs">Track:</span>
          <select
            value={commonTrackId ?? "__mixed__"}
            onChange={(e) => {
              if (e.target.value !== "__mixed__") {
                handleTrackChange(e.target.value);
              }
            }}
            className="app-select app-select--compact text-sm"
          >
            {!allSameTrack && (
              <option value="__mixed__" disabled>Mixed</option>
            )}
            {currentProfile.tracks.map((t) => (
              <option key={t.id} value={t.id}>{t.name}</option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2">
          <span className="text-text-muted text-xs">Loop:</span>
          <select
            value={commonLoopMode ?? "__mixed__"}
            onChange={(e) => {
              if (e.target.value !== "__mixed__") {
                handleLoopModeChange(e.target.value as LoopMode);
              }
            }}
            className="app-select app-select--compact text-sm"
          >
            {!allSameLoop && (
              <option value="__mixed__" disabled>Mixed</option>
            )}
            <option value="off">Off</option>
            <option value="single">Single</option>
            <option value="random">Random</option>
            <option value="sequential">Sequential</option>
          </select>
        </div>
      </div>
    </div>
  );
}
