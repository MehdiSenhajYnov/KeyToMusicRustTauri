# Phase 4.5 - Bug Fixes & Améliorations

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-23

---

## 4.5.1 Corrections UI

- [x] **4.5.1.1** Fix toggle switch ball positioning
  - [x] Added `left-0.5` for explicit positioning
  - [x] Changed off-state to `translate-x-0` instead of negative translate
  **✅ Complété** - GlobalToggles.tsx

- [x] **4.5.1.2** Fix Key Detection toggle not working
  - [x] Added `if (!config.keyDetectionEnabled) return;` guard in `handleKeyPress`
  **✅ Complété** - useKeyDetection.ts

- [x] **4.5.1.3** Fix Now Playing always showing "nothing playing"
  - [x] Added audio event polling thread in main.rs (100ms interval)
  - [x] Thread drains AudioEngine events and emits Tauri events (sound_started, sound_ended, playback_progress)
  **✅ Complété** - main.rs

- [x] **4.5.1.4** Add key deletion functionality
  - [x] Added "Delete Key" button in SoundDetails panel
  - [x] Uses `removeKeyBinding` with confirmation dialog
  **✅ Complété** - SoundDetails.tsx

## 4.5.2 Stop All & Key Detection

- [x] **4.5.2.1** Fix Stop All not working when app is focused
  - [x] Added browser keyboard handler with pressed keys tracking (useRef<Set<string>>)
  - [x] On keydown: checks if all StopAllShortcut keys are pressed
  - [x] On keyup: removes key from set
  **✅ Complété** - useKeyDetection.ts

## 4.5.3 Loop Mode & Sound Selection

- [x] **4.5.3.1** Change loop mode "off" to random selection
  - [x] When multiple sounds on same key with mode "off", picks random sound (avoids repeat)
  - [x] Sound stops when finished (no auto-play next)
  - [x] Updated currentIndex tracking to include "off" mode
  **✅ Complété** - useKeyDetection.ts

## 4.5.4 Key Binding Names

- [x] **4.5.4.1** Add custom name field to KeyBinding
  - [x] Added `name?: string` to TypeScript KeyBinding interface
  - [x] Added `#[serde(default)] pub name: Option<String>` to Rust KeyBinding struct
  - [x] KeyGrid displays `kb.name || firstSound?.name` and total sound count
  - [x] SoundDetails has editable name input with debounced save
  **✅ Complété** - types/index.ts, types.rs, KeyGrid.tsx, SoundDetails.tsx

## 4.5.5 AddSoundModal Improvements

- [x] **4.5.5.1** Add per-file momentum editors
  - [x] FileEntry state with path, momentum, duration per file
  - [x] Per-file momentum editors (number input + range slider + play/stop button)
  - [x] Duration auto-fetched via `getAudioDuration` (symphonia headers)
  - [x] Playing one preview auto-stops any other playing preview
  **✅ Complété** - AddSoundModal.tsx

- [x] **4.5.5.2** Fix multiple sounds only adding one
  - [x] Grouped sounds by key before creating bindings
  - [x] Single binding per key with all sound IDs (not one binding per sound)
  **✅ Complété** - AddSoundModal.tsx

## 4.5.6 Duration Computation

- [x] **4.5.6.1** Replace rodio duration with symphonia header reading
  - [x] Uses symphonia to probe format and read `n_frames` from track params
  - [x] Returns `n_frames / sample_rate` — instant without decoding
  - [x] Falls back to rodio sample-counting if headers lack frame count
  **✅ Complété** - audio/buffer.rs

## 4.5.7 Now Playing Seekable Slider

- [x] **4.5.7.1** Add interactive seek slider
  - [x] Drag-then-release pattern (onChange sets local state, onMouseUp triggers seek)
  - [x] Stop button (■) per active track
  - [x] `updateProgress()` called before async `playSound` to prevent slider jump-back
  **✅ Complété** - NowPlaying.tsx

## 4.5.8 Real-time Sound Volume

- [x] **4.5.8.1** Add SetSoundVolume command through full stack
  - [x] Added `SetSoundVolume { track_id, sound_id, volume }` to AudioCommand enum
  - [x] Handler updates sound_volumes map and recalculates sink volume
  - [x] Added `set_sound_volume` Tauri command in commands.rs
  - [x] Added `setSoundVolume` wrapper in tauriCommands.ts
  - [x] SoundDetails volume slider calls `commands.setSoundVolume` on change
  **✅ Complété** - engine.rs, commands.rs, main.rs, tauriCommands.ts, SoundDetails.tsx
