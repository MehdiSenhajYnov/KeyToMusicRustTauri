import { beforeEach, afterEach, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
let storage: Map<string, string>;
beforeEach(() => {
  vi.resetModules();
  invoke.mockReset();
  storage = new Map();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => storage.get(key) ?? null,
    setItem: (key: string, value: string) => storage.set(key, value),
  });
  vi.stubGlobal("navigator", {});
});
afterEach(() => vi.unstubAllGlobals());

it("loads the native AZERTY map before any keypress and keeps physical bindings", async () => {
  invoke.mockResolvedValue({ KeyA: "q", KeyQ: "a", KeyW: "z", KeyZ: "w" });
  const { refreshKeyboardLayout } = await import("../src/utils/keyboardLayout");
  await refreshKeyboardLayout();
  const mapping = await import("../src/utils/keyMapping");
  expect(mapping.keyCodeToDisplay("KeyA")).toBe("Q");
  expect(mapping.keyCodeToDisplay("KeyQ")).toBe("A");
  expect(mapping.keyCodeToDisplay("Ctrl+KeyW")).toBe("Ctrl+Z");
  expect(mapping.getKeyCode({ code: "KeyA", key: "q" } as KeyboardEvent)).toBe("KeyA");
});

it("applies an entire layout in one notification and refreshes all labels", async () => {
  const mapping = await import("../src/utils/keyMapping");
  const changes: string[][] = [];
  const unsubscribe = mapping.subscribeKeyboardLayout(() => changes.push([
    mapping.keyCodeToDisplay("KeyA"), mapping.keyCodeToDisplay("KeyQ"),
  ]));
  mapping.applyKeyboardLayout({ KeyA: "q", KeyQ: "a" });
  mapping.applyKeyboardLayout({ KeyA: "a", KeyQ: "q" });
  expect(changes).toEqual([["Q", "A"], ["A", "Q"]]);
  unsubscribe();
});

it("does not let typing overwrite an authoritative map", async () => {
  const mapping = await import("../src/utils/keyMapping");
  mapping.applyKeyboardLayout({ KeyA: "q", KeyQ: "a", Digit1: "&" });
  mapping.recordKeyLayout("KeyA", "a");
  mapping.recordKeyLayout("Digit1", "1", { shiftKey: true, altKey: false, ctrlKey: false, metaKey: false });
  expect(mapping.keyCodeToDisplay("KeyA")).toBe("Q");
  expect(mapping.keyCodeToDisplay("Digit1")).toBe("&");
});

it("restores cached labels after restart when native detection is unavailable", async () => {
  let mapping = await import("../src/utils/keyMapping");
  mapping.applyKeyboardLayout({ KeyA: "q", KeyQ: "a" });
  vi.resetModules();
  invoke.mockRejectedValue(Error("No native bridge"));
  const { refreshKeyboardLayout } = await import("../src/utils/keyboardLayout");
  await refreshKeyboardLayout();
  mapping = await import("../src/utils/keyMapping");
  expect(mapping.keyCodeToDisplay("KeyQ")).toBe("A");
  expect(mapping.keyCodeToDisplay("KeyA")).toBe("Q");
});

it("uses the browser map when supported", async () => {
  vi.stubGlobal("navigator", { keyboard: { getLayoutMap: async () => new Map([["KeyY", "z"], ["KeyZ", "y"]]) } });
  const { refreshKeyboardLayout } = await import("../src/utils/keyboardLayout");
  await refreshKeyboardLayout();
  const { keyCodeToDisplay } = await import("../src/utils/keyMapping");
  expect(keyCodeToDisplay("KeyY")).toBe("Z");
  expect(invoke).not.toHaveBeenCalled();
});

it("ignores shifted and AltGr characters when learning fallback labels", async () => {
  const { recordKeyLayout, keyCodeToDisplay } = await import("../src/utils/keyMapping");
  recordKeyLayout("Digit1", "&");
  recordKeyLayout("Digit1", "1", { shiftKey: true, altKey: false, ctrlKey: false, metaKey: false });
  recordKeyLayout("Digit1", "~", { shiftKey: false, altKey: true, ctrlKey: false, metaKey: false });
  expect(keyCodeToDisplay("Digit1")).toBe("&");
});

it("preserves numpad and special-key labels", async () => {
  const { applyKeyboardLayout, keyCodeToDisplay } = await import("../src/utils/keyMapping");
  applyKeyboardLayout({ KeyQ: "a", Numpad1: "1", Enter: "\n" });
  expect(keyCodeToDisplay("Numpad1")).toBe("Num1");
  expect(keyCodeToDisplay("Enter")).toBe("Enter");
});
