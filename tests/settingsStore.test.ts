import { afterEach, expect, it, vi } from "vitest";
vi.mock("../src/utils/tauriCommands", () => ({ updateConfig: vi.fn(), saveProfile: vi.fn() }));
afterEach(() => vi.resetModules());
it("an older settings failure does not roll back unrelated newer edits", async () => {
  const {useSettingsStore: store} = await import("../src/stores/settingsStore");
  const commands = await import("../src/utils/tauriCommands");
  let reject!: (error: Error) => void;
  vi.mocked(commands.updateConfig).mockImplementationOnce(()=>new Promise((_resolve,fail)=>{reject=fail;})).mockResolvedValue(undefined);
  const first = store.getState().updateConfig({autoMomentum:true});
  await Promise.resolve();
  const second = store.getState().updateConfig({crossfadeDuration:900});
  reject(new Error("write failed"));
  await Promise.all([first,second]);
  expect(store.getState().config.autoMomentum).toBe(false);
  expect(store.getState().config.crossfadeDuration).toBe(900);
});
it("two failed edits restore the last confirmed value, not another failed value", async () => {
  const {useSettingsStore: store} = await import("../src/stores/settingsStore");
  const commands = await import("../src/utils/tauriCommands");
  vi.mocked(commands.updateConfig).mockRejectedValue(new Error("Disk full"));
  await Promise.all([store.getState().updateConfig({crossfadeDuration:800}),store.getState().updateConfig({crossfadeDuration:900})]);
  expect(store.getState().config.crossfadeDuration).toBe(500);
});
