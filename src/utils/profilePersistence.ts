import type { Profile } from "../types";

/** Serialize snapshots so a slow write can never replace a newer edit. */
export function createProfilePersistence(
  write: (profile: Profile) => Promise<void>,
  onStatus: (status: "pending" | "saving" | "saved" | "error", error?: unknown) => void,
  delay = 300
) {
  const pending = new Map<string, Profile>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let running: Promise<boolean> | undefined;

  async function drain(): Promise<boolean> {
    while (pending.size) {
      const profile = pending.values().next().value as Profile;
      onStatus("saving");
      try {
        await write(profile);
        if (pending.get(profile.id) === profile) pending.delete(profile.id);
      } catch (error) {
        onStatus("error", error);
        return false; // Keep the latest snapshot available for retry.
      }
    }
    onStatus("saved");
    return true;
  }

  function flush(): Promise<boolean> {
    clearTimeout(timer);
    if (!running) running = drain().finally(() => { running = undefined; });
    return running;
  }

  return {
    enqueue(profile: Profile) {
      pending.set(profile.id, profile);
      onStatus("pending");
      clearTimeout(timer);
      timer = setTimeout(() => { void flush(); }, delay);
    },
    flush,
  };
}
