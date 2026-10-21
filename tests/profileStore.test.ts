import { beforeEach, afterEach, expect, it, vi } from "vitest";
import type { Profile } from "../src/types";

vi.mock("../src/utils/tauriCommands", () => ({
  saveProfile: vi.fn().mockResolvedValue(undefined),
  loadProfile: vi.fn(), listProfiles: vi.fn().mockResolvedValue([]),
  deleteProfile: vi.fn().mockResolvedValue(undefined),
  stopAllSounds: vi.fn().mockResolvedValue(undefined),
  verifyProfileSounds: vi.fn().mockResolvedValue([]),
  preloadProfileSounds: vi.fn().mockResolvedValue({}),
  getWaveformsBatch: vi.fn().mockResolvedValue({}),
  updateConfig: vi.fn().mockResolvedValue(undefined),
}));
const profile = (id: string): Profile => ({ id, name: id, createdAt: "", updatedAt: "", tracks: [], sounds: [], keyBindings: [] });
beforeEach(() => { vi.resetModules(); vi.clearAllMocks(); vi.useFakeTimers(); });
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers(); });

it("persists edits before switching and selects the new profile once", async () => {
  const { useProfileStore: store } = await import("../src/stores/profileStore");
  const commands = await import("../src/utils/tauriCommands");
  store.setState({ currentProfile: profile("a"), isLoading: false });
  store.getState().addTrack({ id: "t", name: "OST", volume: 0.2 });
  vi.mocked(commands.loadProfile).mockResolvedValue(profile("b"));
  expect(await store.getState().loadProfile("b")).toBe(true);
  expect(commands.saveProfile).toHaveBeenCalledWith(expect.objectContaining({ id: "a", tracks: [expect.objectContaining({ volume: 0.2 })] }));
  expect(commands.updateConfig).toHaveBeenCalledWith({ currentProfileId: "b" });
  await store.getState().loadProfile("b");
  expect(commands.loadProfile).toHaveBeenCalledTimes(1);
});

it("does not clear the current selection when deleting an inactive profile", async () => {
  const { useProfileStore: store } = await import("../src/stores/profileStore");
  const commands = await import("../src/utils/tauriCommands");
  store.setState({ currentProfile: profile("a"), isLoading: false });
  expect(await store.getState().deleteProfile("b")).toBe(true);
  expect(store.getState().currentProfile?.id).toBe("a");
  expect(commands.updateConfig).not.toHaveBeenCalled();
});

it("does not silently discard unassigned imported sounds", async () => {
  const { useProfileStore: store } = await import("../src/stores/profileStore");
  const commands = await import("../src/utils/tauriCommands");
  const loaded = profile("b");
  loaded.sounds = [{ id: "unassigned", name: "Keep me", duration: 1, momentum: 0, volume: 1, source: { type: "local", path: "/tmp/sound.wav" } }];
  vi.mocked(commands.loadProfile).mockResolvedValue(loaded);
  await store.getState().loadProfile("b");
  expect(store.getState().currentProfile?.sounds).toHaveLength(1);
});

it("keeps the current profile and reports failure when opening fails", async () => {
  const { useProfileStore: store } = await import("../src/stores/profileStore");
  const commands = await import("../src/utils/tauriCommands");
  store.setState({ currentProfile: profile("a"), isLoading: false });
  vi.mocked(commands.loadProfile).mockRejectedValue(new Error("Corrupt file"));
  expect(await store.getState().loadProfile("b")).toBe(false);
  expect(store.getState().currentProfile?.id).toBe("a");
  expect(commands.stopAllSounds).not.toHaveBeenCalled();
});

it("ignores an old load that finishes after a newer selection", async () => {
  const { useProfileStore: store } = await import("../src/stores/profileStore");
  const commands = await import("../src/utils/tauriCommands");
  let finish!: (p: Profile) => void;
  vi.mocked(commands.loadProfile).mockImplementationOnce(() => new Promise((resolve) => { finish = resolve; })).mockResolvedValue(profile("c"));
  const first = store.getState().loadProfile("b");
  await vi.waitFor(() => expect(commands.loadProfile).toHaveBeenCalled());
  const second = store.getState().loadProfile("c");
  expect(await second).toBe(true);
  finish(profile("b"));
  expect(await first).toBe(false);
  expect(store.getState().currentProfile?.id).toBe("c");
});
