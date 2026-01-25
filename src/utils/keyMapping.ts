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
  masterStopShortcut?: string[];
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
      { keys: config.masterStopShortcut, name: "Master Stop" },
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
 */
export function buildComboFromPressedKeys(pressedKeys: Set<string>): string {
  let hasCtrl = false;
  let hasShift = false;
  let hasAlt = false;
  let baseKey = "";

  for (const key of pressedKeys) {
    if (key === "ControlLeft" || key === "ControlRight") {
      hasCtrl = true;
    } else if (key === "ShiftLeft" || key === "ShiftRight") {
      hasShift = true;
    } else if (key === "AltLeft" || key === "AltRight") {
      hasAlt = true;
    } else if (!baseKey) {
      // Take the first non-modifier key
      baseKey = key;
    }
  }

  if (!baseKey) return "";

  let combo = "";
  if (hasCtrl) combo += "Ctrl+";
  if (hasShift) combo += "Shift+";
  if (hasAlt) combo += "Alt+";
  combo += baseKey;

  return combo;
}
