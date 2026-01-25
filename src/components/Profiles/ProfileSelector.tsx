import { useState, useRef } from "react";
import { useProfileStore } from "../../stores/profileStore";
import { useSettingsStore } from "../../stores/settingsStore";
import { useToastStore } from "../../stores/toastStore";
import { useConfirmStore } from "../../stores/confirmStore";

export function ProfileSelector() {
  const { profiles, currentProfile, loadProfile, createProfile, deleteProfile, loadProfiles, renameProfile, duplicateProfile } =
    useProfileStore();
  const { updateConfig } = useSettingsStore();
  const addToast = useToastStore((s) => s.addToast);
  const showConfirm = useConfirmStore((s) => s.confirm);
  const [isCreating, setIsCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState("");
  const renameInputRef = useRef<HTMLInputElement>(null);

  const handleSelect = async (id: string) => {
    await loadProfile(id);
    await updateConfig({ currentProfileId: id });
  };

  const handleCreate = async () => {
    if (!newName.trim()) return;
    const profile = await createProfile(newName.trim());
    if (profile) {
      await loadProfile(profile.id);
      await updateConfig({ currentProfileId: profile.id });
      addToast(`Profile "${newName.trim()}" created`, "success");
    }
    setNewName("");
    setIsCreating(false);
  };

  const handleDelete = async (id: string, name: string) => {
    if (!await showConfirm(`Delete profile "${name}"?`)) return;
    await deleteProfile(id);
    await updateConfig({ currentProfileId: null });
    await loadProfiles();
    addToast(`Profile "${name}" deleted`, "info");
  };

  const handleRenameStart = (id: string, name: string) => {
    setEditingProfileId(id);
    setEditingName(name);
    setTimeout(() => renameInputRef.current?.select(), 0);
  };

  const handleRenameConfirm = async () => {
    if (!editingProfileId || !editingName.trim()) {
      setEditingProfileId(null);
      return;
    }
    await renameProfile(editingProfileId, editingName.trim());
    setEditingProfileId(null);
  };

  const handleDuplicate = async (id: string, name: string) => {
    const newProfile = await duplicateProfile(id);
    if (newProfile) {
      await loadProfile(newProfile.id);
      await updateConfig({ currentProfileId: newProfile.id });
      addToast(`Profile "${name}" duplicated`, "success");
    }
  };

  return (
    <div className="p-3 space-y-2">
      <h3 className="text-text-muted text-xs font-semibold uppercase tracking-wider">
        Profiles
      </h3>

      <div className="space-y-1">
        {profiles.map((p) => (
          <div
            key={p.id}
            className={`group flex items-center gap-1 px-2 py-1.5 rounded text-sm cursor-pointer transition-colors ${
              currentProfile?.id === p.id
                ? "bg-accent-primary/20 text-accent-primary"
                : "text-text-secondary hover:bg-bg-hover hover:text-text-primary"
            }`}
            onClick={() => handleSelect(p.id)}
          >
            {editingProfileId === p.id ? (
              <input
                ref={renameInputRef}
                type="text"
                value={editingName}
                onChange={(e) => setEditingName(e.target.value)}
                onBlur={handleRenameConfirm}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleRenameConfirm();
                  if (e.key === "Escape") setEditingProfileId(null);
                }}
                onClick={(e) => e.stopPropagation()}
                className="flex-1 min-w-0 bg-bg-tertiary border border-border-focus rounded px-1 py-0 text-sm text-text-primary outline-none"
                autoFocus
              />
            ) : (
              <span
                className="flex-1 truncate"
                onDoubleClick={(e) => {
                  e.stopPropagation();
                  handleRenameStart(p.id, p.name);
                }}
                title="Double-click to rename"
              >
                {p.name}
              </span>
            )}
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleDuplicate(p.id, p.name);
              }}
              className="opacity-0 group-hover:opacity-100 text-text-muted hover:text-accent-primary text-xs p-0.5"
              title="Duplicate"
            >
              ⎘
            </button>
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleDelete(p.id, p.name);
              }}
              className="opacity-0 group-hover:opacity-100 text-text-muted hover:text-accent-error text-xs p-0.5"
              title="Delete"
            >
              x
            </button>
          </div>
        ))}
      </div>

      {isCreating ? (
        <div className="flex gap-1">
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreate();
              if (e.key === "Escape") setIsCreating(false);
            }}
            placeholder="Profile name"
            className="flex-1 bg-bg-tertiary border border-border-color rounded px-2 py-1 text-sm text-text-primary focus:border-border-focus outline-none"
            autoFocus
          />
          <button
            onClick={handleCreate}
            className="text-accent-success text-sm px-1"
          >
            +
          </button>
        </div>
      ) : (
        <button
          onClick={() => setIsCreating(true)}
          className="w-full text-left text-text-muted hover:text-accent-primary text-sm px-2 py-1 rounded hover:bg-bg-hover transition-colors"
        >
          + New Profile
        </button>
      )}
    </div>
  );
}
