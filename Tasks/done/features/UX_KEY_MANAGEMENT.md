# Phase 4.6 - UX Enhancements & Key Management

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-23

---

## 4.6.1 Resizable Panel Divider

- [x] **4.6.1.1** Add resizable divider bar above SoundDetails panel
  - [x] `panelHeight` state (default 256px) with `isResizing` ref
  - [x] Divider bar: `h-1.5 cursor-ns-resize` with hover highlight
  - [x] Mouse/touch drag handlers (mousedown/mousemove/mouseup)
  - [x] Body cursor override during drag, constraints (min 120px, max container-100px)
  **✅ Complété** - MainContent.tsx

## 4.6.2 Track Management

- [x] **4.6.2.1** Change track of existing key binding
  - [x] Added track dropdown selector in SoundDetails panel
  - [x] `handleTrackChange` updates binding's trackId with auto-save
  **✅ Complété** - SoundDetails.tsx

- [x] **4.6.2.2** Rename tracks with double-click
  - [x] `editingTrackId` and `editingName` state in TrackView
  - [x] Double-click on track name enters edit mode (input with autoFocus)
  - [x] Confirm on blur or Enter, cancel on Escape
  **✅ Complété** - TrackView.tsx

## 4.6.3 Profile Switch & Shortcuts

- [x] **4.6.3.1** Stop sounds on profile switch
  - [x] Added `stopAllSounds()` call before loading new profile
  **✅ Complété** - profileStore.ts

- [x] **4.6.3.2** Configurable shortcuts for Auto-Momentum and Key Detection toggles
  - [x] Added `autoMomentumShortcut` and `keyDetectionShortcut` fields to AppConfig (TS + Rust)
  - [x] Added `ToggleKeyDetection` and `ToggleAutoMomentum` variants to KeyEvent enum
  - [x] Key detection shortcut checked BEFORE enabled guard (works when disabled)
  - [x] Rust: added setter methods and shortcut checks in detector.rs
  - [x] Main.rs: handles new KeyEvent variants, emits Tauri events
  - [x] Commands.rs: syncs shortcuts to key detector on config update
  - [x] SettingsModal: unified ShortcutTarget capture with Change/Clear buttons
  - [x] useKeyDetection: listens for toggle events, browser handler checks shortcuts
  **✅ Complété** - types/index.ts, types.rs, mapping.rs, detector.rs, main.rs, commands.rs, settingsStore.ts, SettingsModal.tsx, useKeyDetection.ts

## 4.6.4 AZERTY Layout Support

- [x] **4.6.4.1** Fix AZERTY key display (showing QWERTY letters)
  - [x] Dynamic `layoutMap: Map<string, string>` populated from keydown events
  - [x] `recordKeyLayout(code, key)` records actual character for physical keys
  - [x] `keyCodeToDisplay` checks layoutMap first, falls back to QWERTY map
  - [x] Shortcut capture uses `charToKeyCode(e.key) || e.code` for layout-independent codes
  - [x] Browser handler and settings both call `recordKeyLayout` on keydown
  **✅ Complété** - keyMapping.ts, useKeyDetection.ts, SettingsModal.tsx

## 4.6.5 Key Reassignment

- [x] **4.6.5.1** Change key of entire binding
  - [x] "Change Key" button next to key display in SoundDetails header
  - [x] Capture mode: user presses new key, binding moves to that key
  - [x] Conflict handling: asks to merge if target key already has sounds
  - [x] `onKeyChanged` callback updates parent's selectedKey state
  **✅ Complété** - SoundDetails.tsx, MainContent.tsx

- [x] **4.6.5.2** Move individual sound to different key
  - [x] "Move" button per sound in SoundDetails
  - [x] Capture mode: user presses target key, sound moves there
  - [x] If target has binding: adds sound to it; if not: creates new binding
  - [x] If source binding becomes empty after move: auto-removes it
  **✅ Complété** - SoundDetails.tsx

## 4.6.6 Global Shortcuts Consistency

- [x] **4.6.6.1** All global shortcuts work regardless of key detection state
  - [x] Moved Stop All and auto-momentum shortcut checks before the `enabled` guard in detector.rs
  - [x] All three shortcuts (key detection, Stop All, auto-momentum) now fire even when detection is off, both in foreground and background
  **✅ Complété** - detector.rs

- [x] **4.6.6.2** Fix sticky modifier keys (Alt/Ctrl stuck after window switch)
  - [x] Added `blur` event listener that clears `pressedKeysRef` when window loses focus
  - [x] Prevents phantom modifier keys from triggering shortcuts after Alt+Tab
  **✅ Complété** - useKeyDetection.ts
