const KEY_DISPLAY_MAP: Record<string, string> = {
  // Letters
  KeyA: "A", KeyB: "B", KeyC: "C", KeyD: "D", KeyE: "E",
  KeyF: "F", KeyG: "G", KeyH: "H", KeyI: "I", KeyJ: "J",
  KeyK: "K", KeyL: "L", KeyM: "M", KeyN: "N", KeyO: "O",
  KeyP: "P", KeyQ: "Q", KeyR: "R", KeyS: "S", KeyT: "T",
  KeyU: "U", KeyV: "V", KeyW: "W", KeyX: "X", KeyY: "Y", KeyZ: "Z",
  // Digits
  Digit0: "0", Digit1: "1", Digit2: "2", Digit3: "3", Digit4: "4",
  Digit5: "5", Digit6: "6", Digit7: "7", Digit8: "8", Digit9: "9",
  // Numpad
  Numpad0: "Num0", Numpad1: "Num1", Numpad2: "Num2", Numpad3: "Num3",
  Numpad4: "Num4", Numpad5: "Num5", Numpad6: "Num6", Numpad7: "Num7",
  Numpad8: "Num8", Numpad9: "Num9",
  NumpadMultiply: "Num*", NumpadAdd: "Num+", NumpadSubtract: "Num-",
  NumpadDecimal: "Num.", NumpadDivide: "Num/", NumpadEnter: "NumEnter",
  // Function keys
  F1: "F1", F2: "F2", F3: "F3", F4: "F4", F5: "F5", F6: "F6",
  F7: "F7", F8: "F8", F9: "F9", F10: "F10", F11: "F11", F12: "F12",
  // Arrows
  ArrowUp: "\u2191", ArrowDown: "\u2193", ArrowLeft: "\u2190", ArrowRight: "\u2192",
  // Special
  Space: "Space", Enter: "Enter", Tab: "Tab", Escape: "Esc",
  Backspace: "Bksp", Delete: "Del", Insert: "Ins",
  Home: "Home", End: "End", PageUp: "PgUp", PageDown: "PgDn",
  CapsLock: "Caps", NumLock: "NumLk", ScrollLock: "ScrLk",
  PrintScreen: "PrtSc", Pause: "Pause",
  // Modifiers
  ShiftLeft: "LShift", ShiftRight: "RShift",
  ControlLeft: "LCtrl", ControlRight: "RCtrl",
  AltLeft: "LAlt", AltRight: "RAlt",
  MetaLeft: "LMeta", MetaRight: "RMeta",
  // Punctuation
  Semicolon: ";", Comma: ",", Period: ".", Slash: "/",
  Backslash: "\\", Quote: "'", Backquote: "`",
  BracketLeft: "[", BracketRight: "]", Minus: "-", Equal: "=",
};

// Dynamic layout map: records the actual character for each physical key code
// Populated at runtime from keydown events, so it adapts to any keyboard layout
const layoutMap: Map<string, string> = new Map();

export function recordKeyLayout(code: string, key: string): void {
  // Only record single-character keys (letters, digits, punctuation)
  if (key.length === 1 && /^Key[A-Z]$|^Digit[0-9]$/.test(code)) {
    layoutMap.set(code, key.toUpperCase());
  }
}

export function keyCodeToDisplay(code: string): string {
  // Handle combined key codes (e.g., "Ctrl+Shift+KeyA")
  if (code.includes("+")) {
    const parts = code.split("+");
    return parts.map((part) => {
      // Simplify modifier display
      if (part === "Ctrl") return "Ctrl";
      if (part === "Shift") return "Shift";
      if (part === "Alt") return "Alt";
      // Use the learned layout map first (handles AZERTY, QWERTZ, etc.)
      const layoutChar = layoutMap.get(part);
      if (layoutChar) return layoutChar;
      return KEY_DISPLAY_MAP[part] || part;
    }).join("+");
  }
  // Use the learned layout map first (handles AZERTY, QWERTZ, etc.)
  const layoutChar = layoutMap.get(code);
  if (layoutChar) return layoutChar;
  return KEY_DISPLAY_MAP[code] || code;
}

const CHAR_TO_KEYCODE: Record<string, string> = {
  a: "KeyA", b: "KeyB", c: "KeyC", d: "KeyD", e: "KeyE",
  f: "KeyF", g: "KeyG", h: "KeyH", i: "KeyI", j: "KeyJ",
  k: "KeyK", l: "KeyL", m: "KeyM", n: "KeyN", o: "KeyO",
  p: "KeyP", q: "KeyQ", r: "KeyR", s: "KeyS", t: "KeyT",
  u: "KeyU", v: "KeyV", w: "KeyW", x: "KeyX", y: "KeyY", z: "KeyZ",
  "0": "Digit0", "1": "Digit1", "2": "Digit2", "3": "Digit3", "4": "Digit4",
  "5": "Digit5", "6": "Digit6", "7": "Digit7", "8": "Digit8", "9": "Digit9",
};

export function parseKeyCombination(keys: string): string[] {
  return keys
    .toLowerCase()
    .split("")
    .map((ch) => CHAR_TO_KEYCODE[ch])
    .filter((code): code is string => code !== undefined);
}

export function charToKeyCode(char: string): string | null {
  return CHAR_TO_KEYCODE[char.toLowerCase()] || null;
}

/**
 * Get the correct key code for a keyboard event.
 * Prefers e.code for Numpad keys (to distinguish from Digit keys),
 * otherwise uses charToKeyCode for layout-aware mapping.
 */
export function getKeyCode(e: KeyboardEvent): string {
  // For Numpad keys, always use e.code to distinguish from digit row
  if (e.code.startsWith("Numpad")) {
    return e.code;
  }
  // For other keys, try character-based mapping for layout awareness
  return charToKeyCode(e.key) || e.code;
}

export function isValidKeyCode(code: string): boolean {
  return code in KEY_DISPLAY_MAP;
}

export function formatShortcut(keys: string[]): string {
  return keys.map(keyCodeToDisplay).join("+");
}

/**
 * Build a combined key code from modifier flags and base key.
 * Order: Ctrl > Shift > Alt > Key (consistent with backend)
 */
export function buildKeyCombo(
  baseKey: string,
  modifiers: { ctrl?: boolean; shift?: boolean; alt?: boolean }
): string {
  let combo = "";
  if (modifiers.ctrl) combo += "Ctrl+";
  if (modifiers.shift) combo += "Shift+";
  if (modifiers.alt) combo += "Alt+";
  combo += baseKey;
  return combo;
}

/**
 * Parse a combined key code into its components.
 */
export function parseKeyCombo(combo: string): {
  baseKey: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
} {
  const parts = combo.split("+");
  const baseKey = parts[parts.length - 1];
  const modifiers = parts.slice(0, -1);
  return {
    baseKey,
    ctrl: modifiers.includes("Ctrl"),
    shift: modifiers.includes("Shift"),
    alt: modifiers.includes("Alt"),
  };
}

/**
 * Check if a key combo conflicts with common system shortcuts.
 * Returns a warning message if conflict detected, null otherwise.
 * @deprecated Use checkShortcutConflicts instead for full validation
 */
export function checkKeyComboConflict(combo: string): string | null {
  const { baseKey, ctrl, alt } = parseKeyCombo(combo);

  // Common system shortcuts to warn about
  const systemShortcuts: Record<string, string> = {
    "Ctrl+KeyC": "Copy",
    "Ctrl+KeyV": "Paste",
    "Ctrl+KeyX": "Cut",
    "Ctrl+KeyZ": "Undo",
    "Ctrl+KeyY": "Redo",
    "Ctrl+KeyA": "Select All",
    "Ctrl+KeyS": "Save",
    "Ctrl+KeyW": "Close Window",
    "Ctrl+KeyQ": "Quit App",
    "Ctrl+KeyN": "New Window",
    "Ctrl+KeyT": "New Tab",
    "Alt+F4": "Close Window",
  };

  if (systemShortcuts[combo]) {
    return `This shortcut conflicts with "${systemShortcuts[combo]}"`;
  }

  // Warn about Ctrl+number (browser tab switching)
  if (ctrl && /^Digit[1-9]$/.test(baseKey)) {
    return "This shortcut may conflict with browser tab switching";
  }

  // Warn about Alt+letter (menu access on Windows)
  if (alt && /^Key[A-Z]$/.test(baseKey)) {
    return "This shortcut may conflict with menu access on Windows";
  }

  return null;
}

/**
 * Shortcut conflict result
 */
export interface ShortcutConflict {
  type: "error" | "warning";
  message: string;
  conflictWith: string;
}

/**
 * Configuration for shortcut conflict checking
 */
export interface ShortcutConflictConfig {
  stopAllShortcut?: string[];
  autoMomentumShortcut?: string[];
  keyDetectionShortcut?: string[];
}

/**
 * Convert array of key codes to a combined key code string.
 * ["ControlLeft", "ShiftLeft", "KeyS"] -> "Ctrl+Shift+KeyS"
 */
export function keysArrayToCombo(keys: string[]): string {
  if (keys.length === 0) return "";

  let hasCtrl = false;
  let hasShift = false;
  let hasAlt = false;
  let baseKey = "";

  for (const key of keys) {
    if (key === "ControlLeft" || key === "ControlRight") {
      hasCtrl = true;
    } else if (key === "ShiftLeft" || key === "ShiftRight") {
      hasShift = true;
    } else if (key === "AltLeft" || key === "AltRight") {
      hasAlt = true;
    } else {
      baseKey = key;
    }
  }

  let combo = "";
  if (hasCtrl) combo += "Ctrl+";
  if (hasShift) combo += "Shift+";
  if (hasAlt) combo += "Alt+";
  combo += baseKey;

  return combo;
}

/**
 * Check if a key combo conflicts with reserved shortcuts.
 * Includes app shortcuts (Undo/Redo), user-configured global shortcuts,
 * system shortcuts, and optionally existing profile bindings.
 */
export function checkShortcutConflicts(
  combo: string,
  config?: ShortcutConflictConfig
): ShortcutConflict | null {
  const { baseKey, ctrl, alt } = parseKeyCombo(combo);

  // 1. App undo/redo shortcuts (hardcoded, always blocked)
  const appShortcuts: Record<string, string> = {
    "Ctrl+KeyZ": "Undo",
    "Ctrl+KeyY": "Redo",
  };

  if (appShortcuts[combo]) {
    return {
      type: "error",
      message: `Reserved for ${appShortcuts[combo]}`,
      conflictWith: appShortcuts[combo],
    };
  }

  // 2. User-configured global shortcuts
  if (config) {
    const globalShortcuts = [
      { keys: config.stopAllShortcut, name: "Stop All" },
      { keys: config.autoMomentumShortcut, name: "Auto-Momentum" },
      { keys: config.keyDetectionShortcut, name: "Key Detection" },
    ];

    for (const shortcut of globalShortcuts) {
      if (shortcut.keys && shortcut.keys.length > 0) {
        const shortcutCombo = keysArrayToCombo(shortcut.keys);
        if (shortcutCombo === combo) {
          return {
            type: "error",
            message: `Used for ${shortcut.name}`,
            conflictWith: shortcut.name,
          };
        }
      }
    }

  }

  // 3. System shortcuts (blocked)
  const systemShortcuts: Record<string, string> = {
    "Ctrl+KeyC": "Copy",
    "Ctrl+KeyV": "Paste",
    "Ctrl+KeyX": "Cut",
    "Ctrl+KeyA": "Select All",
    "Ctrl+KeyS": "Save",
    "Ctrl+KeyW": "Close Window",
    "Ctrl+KeyQ": "Quit",
    "Ctrl+KeyN": "New Window",
    "Ctrl+KeyT": "New Tab",
    "Alt+F4": "Close Window",
  };

  if (systemShortcuts[combo]) {
    return {
      type: "error",
      message: `System shortcut (${systemShortcuts[combo]})`,
      conflictWith: systemShortcuts[combo],
    };
  }

  // 4. Warnings (allowed but inform user)
  if (ctrl && /^Digit[1-9]$/.test(baseKey)) {
    return {
      type: "warning",
      message: "May conflict with browser tabs",
      conflictWith: "Browser tabs",
    };
  }

  if (alt && /^Key[A-Z]$/.test(baseKey)) {
    return {
      type: "warning",
      message: "May conflict with Windows menus",
      conflictWith: "Windows menus",
    };
  }

  return null;
}

/**
 * Build a combined key code from pressed keys Set.
 * Used during key capture to create the combo string.
 * Supports multi-key chords: multiple base keys are sorted alphabetically.
 * Format: Ctrl+Shift+Alt+KeyA+KeyB+KeyZ (modifiers first, then sorted base keys)
 */
export function buildComboFromPressedKeys(pressedKeys: Set<string>): string {
  let hasCtrl = false;
  let hasShift = false;
  let hasAlt = false;
  const baseKeys: string[] = [];

  for (const key of pressedKeys) {
    if (key === "ControlLeft" || key === "ControlRight") {
      hasCtrl = true;
    } else if (key === "ShiftLeft" || key === "ShiftRight") {
      hasShift = true;
    } else if (key === "AltLeft" || key === "AltRight") {
      hasAlt = true;
    } else {
      baseKeys.push(key);
    }
  }

  if (baseKeys.length === 0) return "";

  // Sort base keys alphabetically for consistent ordering
  baseKeys.sort();

  const parts: string[] = [];
  if (hasCtrl) parts.push("Ctrl");
  if (hasShift) parts.push("Shift");
  if (hasAlt) parts.push("Alt");
  parts.push(...baseKeys);

  return parts.join("+");
}

/**
 * Normalize a combo string to canonical form.
 * Ensures modifiers are in order Ctrl > Shift > Alt and base keys are sorted alphabetically.
 */
export function normalizeCombo(combo: string): string {
  const parts = combo.split("+");

  let hasCtrl = false;
  let hasShift = false;
  let hasAlt = false;
  const baseKeys: string[] = [];

  for (const part of parts) {
    if (part === "Ctrl") {
      hasCtrl = true;
    } else if (part === "Shift") {
      hasShift = true;
    } else if (part === "Alt") {
      hasAlt = true;
    } else {
      baseKeys.push(part);
    }
  }

  // Sort base keys alphabetically
  baseKeys.sort();

  const normalized: string[] = [];
  if (hasCtrl) normalized.push("Ctrl");
  if (hasShift) normalized.push("Shift");
  if (hasAlt) normalized.push("Alt");
  normalized.push(...baseKeys);

  return normalized.join("+");
}

/**
 * Check if a combo is a multi-key chord (has multiple base keys).
 */
export function isMultiKeyChord(combo: string): boolean {
  const parts = combo.split("+");
  const baseKeys = parts.filter(p => p !== "Ctrl" && p !== "Shift" && p !== "Alt");
  return baseKeys.length > 1;
}

/**
 * Momentum modifier type (re-exported from types for backward compatibility)
 */
import type { MomentumModifier } from "../types";
export type MomentumModifierType = MomentumModifier;

/**
 * Check if a shortcut key array contains a specific modifier.
 */
export function shortcutHasModifier(keys: string[], modifier: MomentumModifierType): boolean {
  if (modifier === "None") return false;
  return keys.some((k) => {
    switch (modifier) {
      case "Shift":
        return k === "ShiftLeft" || k === "ShiftRight";
      case "Ctrl":
        return k === "ControlLeft" || k === "ControlRight";
      case "Alt":
        return k === "AltLeft" || k === "AltRight";
      default:
        return false;
    }
  });
}

/**
 * Get the base key (non-modifier) from a shortcut key array.
 */
export function getShortcutBaseKey(keys: string[]): string | null {
  const baseKey = keys.find(
    (k) =>
      !k.includes("Shift") &&
      !k.includes("Control") &&
      !k.includes("Alt")
  );
  return baseKey ?? null;
}

/**
 * Get all base keys from profile bindings.
 */
export function getProfileBaseKeys(bindings: { keyCode: string }[]): Set<string> {
  const baseKeys = new Set<string>();
  for (const binding of bindings) {
    const parts = binding.keyCode.split("+");
    const baseKey = parts[parts.length - 1];
    baseKeys.add(baseKey);
  }
  return baseKeys;
}

/**
 * Information about a momentum/shortcut conflict.
 */
export interface MomentumConflict {
  shortcutName: string;
  shortcutKeys: string[];
  boundKey: string;
}

/**
 * Check if shortcuts conflict with momentum modifier and profile bindings.
 * Returns list of conflicting shortcuts.
 */
export function findMomentumConflicts(
  momentumModifier: MomentumModifierType,
  shortcuts: { name: string; keys: string[] }[],
  profileBindings: { keyCode: string }[]
): MomentumConflict[] {
  if (momentumModifier === "None") return [];

  const profileBaseKeys = getProfileBaseKeys(profileBindings);
  const conflicts: MomentumConflict[] = [];

  for (const shortcut of shortcuts) {
    if (shortcut.keys.length === 0) continue;

    const baseKey = getShortcutBaseKey(shortcut.keys);
    if (
      baseKey &&
      shortcutHasModifier(shortcut.keys, momentumModifier) &&
      profileBaseKeys.has(baseKey)
    ) {
      conflicts.push({
        shortcutName: shortcut.name,
        shortcutKeys: shortcut.keys,
        boundKey: baseKey,
      });
    }
  }

  return conflicts;
}

/**
 * Check if a specific key binding is affected by a momentum conflict.
 */
export function getKeyMomentumConflict(
  keyCode: string,
  momentumModifier: MomentumModifierType,
  shortcuts: { name: string; keys: string[] }[]
): MomentumConflict | null {
  if (momentumModifier === "None") return null;

  // Get the base key from the binding
  const parts = keyCode.split("+");
  const baseKey = parts[parts.length - 1];

  for (const shortcut of shortcuts) {
    if (shortcut.keys.length === 0) continue;

    const shortcutBaseKey = getShortcutBaseKey(shortcut.keys);
    if (
      shortcutBaseKey === baseKey &&
      shortcutHasModifier(shortcut.keys, momentumModifier)
    ) {
      return {
        shortcutName: shortcut.name,
        shortcutKeys: shortcut.keys,
        boundKey: baseKey,
      };
    }
  }

  return null;
}

/**
 * Build the shortcuts array used for conflict detection.
 */
import type { AppConfig } from "../types";
export function buildShortcutsList(config: AppConfig) {
  return [
    { name: "Stop All", keys: config.stopAllShortcut },
    { name: "Auto-Momentum", keys: config.autoMomentumShortcut },
    { name: "Key Detection", keys: config.keyDetectionShortcut },
  ];
}

/**
 * Preferred order for auto-assigning keys to discovery suggestions.
 */
const AUTO_KEY_ORDER = [
  "KeyA","KeyB","KeyC","KeyD","KeyE","KeyF","KeyG","KeyH","KeyI","KeyJ",
  "KeyK","KeyL","KeyM","KeyN","KeyO","KeyP","KeyQ","KeyR","KeyS","KeyT",
  "KeyU","KeyV","KeyW","KeyX","KeyY","KeyZ",
  "Digit1","Digit2","Digit3","Digit4","Digit5","Digit6","Digit7","Digit8","Digit9","Digit0",
  "F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12",
  "Numpad0","Numpad1","Numpad2","Numpad3","Numpad4",
  "Numpad5","Numpad6","Numpad7","Numpad8","Numpad9",
];

/**
 * Find the next available key code that is not in `usedKeys` or `alreadySuggested`.
 * Returns "" if all keys are taken.
 */
export function findNextAvailableKey(
  usedKeys: Set<string>,
  alreadySuggested?: Set<string>
): string {
  for (const key of AUTO_KEY_ORDER) {
    if (!usedKeys.has(key) && (!alreadySuggested || !alreadySuggested.has(key))) {
      return key;
    }
  }
  return "";
}
