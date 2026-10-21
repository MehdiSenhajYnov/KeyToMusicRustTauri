import { afterEach, expect, it, vi } from "vitest";
import { createProfilePersistence } from "../src/utils/profilePersistence";
import type { Profile } from "../src/types";

const profile = (id: string, name: string): Profile => ({ id, name, createdAt: "", updatedAt: "", sounds: [], tracks: [], keyBindings: [] });
afterEach(() => vi.useRealTimers());

it("serializes saves and retains edits made during a slow write", async () => {
  vi.useFakeTimers();
  let finish!: () => void;
  const write = vi.fn().mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; })).mockResolvedValue(undefined);
  const queue = createProfilePersistence(write, vi.fn());
  queue.enqueue(profile("a", "first"));
  const saved = queue.flush();
  queue.enqueue(profile("a", "latest"));
  queue.enqueue(profile("b", "other profile"));
  expect(write).toHaveBeenCalledTimes(1);
  finish();
  expect(await saved).toBe(true);
  expect(write.mock.calls.map(([p]) => p.name)).toEqual(["first", "latest", "other profile"]);
});

it("keeps failed changes available for retry", async () => {
  vi.useFakeTimers();
  const write = vi.fn().mockRejectedValueOnce(new Error("Disk full")).mockResolvedValue(undefined);
  const status = vi.fn();
  const queue = createProfilePersistence(write, status);
  queue.enqueue(profile("a", "valuable edit"));
  expect(await queue.flush()).toBe(false);
  expect(status).toHaveBeenLastCalledWith("error", expect.any(Error));
  expect(await queue.flush()).toBe(true);
  expect(write).toHaveBeenCalledTimes(2);
  expect(write.mock.calls[1][0].name).toBe("valuable edit");
});
