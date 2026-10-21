import { expect, it, vi } from "vitest";
import { selectLoopSound } from "../src/utils/loopPlayback";
import { useAudioStore } from "../src/stores/audioStore";
import { useConfirmStore } from "../src/stores/confirmStore";
import type { Sound } from "../src/types";
const sound: Sound = { id: "one", name: "One", duration: 10, momentum: 0, volume: 1, source: { type: "local", path: "one.wav" } };
it.each([-1, NaN, Infinity, 99999])("recovers an invalid single-loop index %s", (index) => {
  expect(selectLoopSound(["missing", "one"], new Map([[sound.id, sound]]), "single", index)).toEqual({ sound, nextIndex: 0 });
});
it("does not clear a newly playing sound on a stale ended event", () => {
  useAudioStore.getState().setSoundStarted("track", "new");
  useAudioStore.getState().setSoundEnded("track", "old");
  expect(useAudioStore.getState().playingTracks.get("track")?.soundId).toBe("new");
});
it("settles replaced confirmations without timers closing the new dialog", async () => {
  vi.useFakeTimers();
  const first = useConfirmStore.getState().confirm("First");
  const second = useConfirmStore.getState().confirm("Second");
  expect(await first).toBe(false);
  await vi.advanceTimersByTimeAsync(60000);
  expect(useConfirmStore.getState().message).toBe("Second");
  expect(useConfirmStore.getState().isOpen).toBe(true);
  useConfirmStore.getState().close(true);
  expect(await second).toBe(true);
  vi.useRealTimers();
});

import { moveKeyBinding, mergeKeyBindings } from "../src/utils/keyBindings";
import { selectMoodBindings } from "../src/utils/moodHelpers";
import type { KeyBinding } from "../src/types";
const binding = (keyCode: string, trackId: string, soundIds: string[]): KeyBinding => ({keyCode,trackId,soundIds,loopMode:"single",currentIndex:0});
it("moving a key preserves other tracks and merges only the matching track", () => {
  const bindings = [binding("KeyA","ost",["one"]),binding("KeyA","sfx",["two"]),binding("KeyB","ost",["three"])];
  const moved = moveKeyBinding(bindings,"KeyA","ost","KeyB");
  expect(moved).toContainEqual(bindings[1]);
  expect(moved.find(b=>b.keyCode==="KeyB")?.soundIds).toEqual(["three","one"]);
  expect(moved).toHaveLength(2);
});
it("merging tracks does not create duplicate key-track pairs or sound references", () => {
  expect(mergeKeyBindings([binding("KeyA","ost",["one"]),binding("KeyA","ost",["one","two"])]))
    .toEqual([binding("KeyA","ost",["one","two"])]);
});
it("Mood AI chooses the strongest eligible intensity per track", () => {
  const low = {...binding("KeyA","ost",["one"]),mood:"epic" as const,moodIntensity:1 as const};
  const high = {...binding("KeyB","ost",["two"]),mood:"epic" as const,moodIntensity:3 as const};
  expect(selectMoodBindings([low,high],"epic",3)).toEqual([high]);
  expect(selectMoodBindings([low,high],"epic",2)).toEqual([low]);
});
