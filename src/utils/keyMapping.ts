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
