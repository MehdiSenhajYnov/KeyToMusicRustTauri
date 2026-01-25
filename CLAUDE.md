# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

KeyToMusic is a Tauri-based desktop soundboard application designed for manga reading. It provides global keyboard detection to trigger assigned sounds without interrupting the reading experience. The app supports multi-track audio playback with crossfading, YouTube downloads, momentum (start position), and multiple loop modes.

**Platforms:** Windows 10/11, macOS 10.15+, Linux (Ubuntu, Fedora, Arch)

## Tech Stack

- **Framework:** Tauri 2.x (Rust backend + React frontend)
- **Frontend:** React 18+ with TypeScript, Tailwind CSS, Zustand (state management)
- **Backend:** Rust with rodio/cpal (audio), symphonia (fast seeking), platform-specific global key detection
- **External Tools:** yt-dlp (YouTube downloads)

## Project Structure

```
keytomusic/
├── src/                          # React/TypeScript frontend
│   ├── components/               # UI components (Layout, Tracks, Sounds, Keys, etc.)
│   │   ├── Errors/              # FileNotFoundModal
│   │   └── ConfirmDialog.tsx    # Custom confirm modal (replaces browser confirm())
│   ├── stores/                   # Zustand state management (profile, settings, error, export, toast, confirm)
│   ├── hooks/                    # Custom React hooks (useAudioEvents, useKeyDetection)
│   ├── types/                    # TypeScript type definitions
│   └── utils/                    # Frontend utilities (errorMessages, keyMapping, fileHelpers, tauriCommands)
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry point, logging init, event forwarding
│   │   ├── commands.rs           # Tauri commands exposed to frontend
│   │   ├── audio/                # Audio engine, tracks, crossfade, symphonia seeking
│   │   ├── keys/                 # Global keyboard detection & mapping (platform-specific)
│   │   │   ├── detector.rs       # Key detector with cooldown, shortcuts
│   │   │   ├── mapping.rs        # KeyEvent types, key code conversions
│   │   │   ├── chord.rs          # Multi-key chord detection (Trie-based)
│   │   │   ├── macos_listener.rs # macOS-only CGEventTap implementation
│   │   │   └── windows_listener.rs # Windows-only Raw Input API implementation
│   │   ├── youtube/              # YouTube downloader, cache, ffmpeg/yt-dlp managers
│   │   ├── import_export/        # .ktm file handling
│   │   └── storage/              # Profile & config persistence
│   └── Cargo.toml
├── data/                         # Runtime user data (profiles, cache, config, logs)
└── resources/                    # Static resources (icons, error.mp3)
```

## Core Architecture Concepts

### Audio System

**Multi-Track Architecture:** The app supports up to 20 independent audio tracks (OST, Ambiance, SFX). Each track can play one sound at a time. Playing a new sound on a track triggers a crossfade.

**Volume Hierarchy:**
```
final_volume = sound.volume × track.volume × master_volume
```

**Real-time Sound Volume:** Sound volume can be changed while playing via the `set_sound_volume` command, which updates the sink volume immediately without restarting playback.

**Audio Event Forwarding:** A dedicated polling thread in `main.rs` drains audio engine events every 100ms and emits them as Tauri events (`sound_started`, `sound_ended`, `playback_progress`) to the frontend.

**Duration Reading:** Audio file durations are computed via symphonia header reading (`n_frames / sample_rate`), providing instant results without decoding. Falls back to rodio sample-counting only when headers lack frame count info.

**Crossfade:** 500ms default duration with a custom curve that creates a brief silence gap between outgoing and incoming sounds (35%-65% of the duration). Crossfade only occurs between sounds on the same track.

**Momentum:** Each sound has a momentum property (start position in seconds). Sounds can start from 0:00 or from their momentum position. Auto-Momentum mode or the configured momentum modifier key (Shift/Ctrl/Alt, configurable in Settings) triggers momentum start.

**Playback via Symphonia:** All audio playback uses a custom `SymphoniaSource` that implements `rodio::Source`. This provides consistent format support (MP3, M4A/AAC, OGG, FLAC, WAV) and instant byte-level seeking for momentum (O(1) for CBR, O(log n) for VBR). The `isomp4` symphonia feature is required for M4A files from YouTube downloads.

**Audio Device Selection:** Users can select a specific output device from Settings, or follow the system default (None). The device list is provided via `cpal` host enumeration. The selected device is persisted in `config.json` as `audioDevice`.

**Seamless Device Switching:** When the audio device changes (either via user selection in Settings or OS default device change), playback resumes automatically on the new device. The engine captures each playing track's state (file_path, position, volumes), rebuilds the OutputStream, then immediately resumes all tracks at their captured positions with no crossfade. This produces a brief gap (<50ms) but no `SoundEnded` events are emitted, so the frontend sees uninterrupted playback. The `AudioTrack` struct stores `file_path: Option<String>` to enable this resume capability.

**Device Polling:** When following the system default (audioDevice = None), the audio thread polls for default device changes every 3 seconds via `cpal::default_host().default_output_device()`. If the device name changes, the seamless switch is triggered automatically.

### Key Detection

Uses platform-specific global keyboard capture (works both when app is focused AND in background):
- **Windows:** Uses Raw Input API (`src-tauri/src/keys/windows_listener.rs`)
- **macOS:** Uses custom CGEventTap implementation (`src-tauri/src/keys/macos_listener.rs`)
- **Linux:** Uses `rdev` crate

**Windows Raw Input API Implementation:** The standard `rdev` crate and `SetWindowsHookEx` with `WH_KEYBOARD_LL` don't receive keyboard events when the Tauri/WebView2 window is focused. The Raw Input API bypasses this limitation:
- Creates a hidden message-only window (`HWND_MESSAGE`)
- Registers for raw keyboard input with `RIDEV_INPUTSINK` flag (receives input even when not in foreground)
- Processes `WM_INPUT` messages to capture all keyboard events globally
- Maps Windows virtual key codes to Web KeyboardEvent.code strings
- Works consistently regardless of window focus state

**macOS CGEventTap Implementation:** On macOS 13+, rdev crashes because it calls `TSMGetInputSourceProperty` from a background thread, which Apple now enforces must run on the main dispatch queue. The custom implementation uses CoreGraphics CGEventTap directly:
- Creates an event tap at HID level for hardware-level key capture
- Runs in a CFRunLoop on a dedicated Rust thread
- Maps hardware keycodes (0x00-0x7E) directly to Web KeyboardEvent.code strings
- Still captures keys globally even when app is in background

**Cooldown:** Global 1500ms cooldown (configurable) applies to ALL key presses to prevent accidental spam.

**Global Shortcuts:** Master Stop, Auto-Momentum toggle, and Key Detection toggle shortcuts all work regardless of key detection state (both foreground and background). They are checked before the `enabled` guard in `detector.rs`. Only sound-triggering key presses are blocked when detection is disabled.

**Master Stop:** Configurable key combination (default: Ctrl+Shift+S) stops all sounds on all tracks. Works both in background (via rdev) and in foreground (via browser keyboard handler with pressed keys tracking).

**Frontend Key Detection Guard:** The `handleKeyPress` function checks `config.keyDetectionEnabled` before processing any key event, ensuring the toggle actually disables detection.

**Auto-disable:** Key detection is automatically disabled when text input fields are focused in the UI.

**Keyboard Layout Support (AZERTY etc.):** Uses `charToKeyCode(e.key) || e.code` pattern for layout-independent key codes. A dynamic `layoutMap` records actual characters from keydown events via `recordKeyLayout()`. The `keyCodeToDisplay()` function checks layoutMap first for correct display, falling back to QWERTY map.

**Sticky Modifier Fix:** `pressedKeysRef` is cleared on window `blur` event to prevent phantom modifier keys (e.g., Alt stuck after Alt+Tab) from triggering shortcuts incorrectly.

### Modal Dialogs (ConfirmDialog)

**Why custom dialogs:** Browser `confirm()` doesn't work on macOS WKWebView (returns immediately without user input). A custom React modal with Zustand store replaces all confirmation dialogs.

**Pattern:**
```typescript
// Usage anywhere in the app:
const confirmed = await useConfirmStore.getState().confirm("Delete this item?");
if (confirmed) { /* proceed */ }
```

**Store (`src/stores/confirmStore.ts`):**
- `confirm(message): Promise<boolean>` - Shows modal, returns promise
- `close(result: boolean)` - Resolves promise and hides modal

**Component (`src/components/ConfirmDialog.tsx`):** Modal with Cancel/Confirm buttons, dark theme, uses `autoFocus` on Confirm button.

### Sound Assignment & Loop Modes

Each key binding associates:
- A keyboard key (e.g., "KeyA", "F5")
- A track ID
- A list of sound IDs (can assign multiple sounds to one key)
- A loop mode
- An optional custom name (defaults to first sound's name in the UI)

**Key Reassignment:** The SoundDetails panel supports:
- **Change entire binding's key:** "Change Key" button captures a new key and moves all sounds (merges if target key exists).
- **Move individual sound:** "Move" button per sound captures a target key and moves just that sound (creates binding if needed, removes source binding if empty).

**Track Management:** Track of an existing binding can be changed via dropdown in SoundDetails. Tracks can be renamed by double-clicking their name in TrackView.

**Loop Modes:**
- `off`: Play a random sound from the list (no repeat), stop when finished
- `single`: Loop the same sound continuously (respects momentum on loop)
- `random`: Play a random sound from the list (avoid repeating the same), auto-play next on end
- `sequential`: Cycle through sounds in order, auto-play next on end

### YouTube Integration

**Concurrent Downloads:** Multiple YouTube downloads can run simultaneously. Each download is identified by a unique `download_id` passed to `add_sound_from_youtube`. Progress events include this ID so the frontend can track each download independently. The UI shows individual progress bars per download and the URL input remains available during downloads.

**yt-dlp:** Downloads YouTube audio as M4A (best audio quality) and stores in `data/cache/`. Uses video ID as filename (`{video_id}.m4a`) for predictable paths.

**ffmpeg Auto-Install:** YouTube provides DASH fragmented MP4 audio which requires ffmpeg to remux into proper M4A. The app auto-downloads ffmpeg from `yt-dlp/FFmpeg-Builds` GitHub releases on first YouTube download. ffmpeg is stored in the app's `bin/` directory alongside yt-dlp, and yt-dlp auto-detects it via `--ffmpeg-location`.

**Canonical URL Cache:** Cache lookups use canonical URLs (`https://www.youtube.com/watch?v={id}`) to avoid duplicate downloads when URLs contain extra parameters (list=, pp=, etc.).

**Title Extraction:** Uses `--write-info-json` flag to get the video title from yt-dlp's JSON output. The info.json file is cleaned up after reading.

**Retry Logic:** Transient network errors (connection, timeout, incomplete reads) are automatically retried up to 3 times with a 2-second delay between attempts. Non-retryable errors (private video, unavailable, geo-blocked) fail immediately.

**Cache Cleanup:** At startup, after `save_profile`, and after `delete_profile`, the app scans all profile JSONs to collect referenced `cachedPath` values. Cache entries (and their files) not referenced by any profile are automatically deleted. This scan-based approach avoids stale `usedBy` tracking.

### Profiles & Persistence

**Profiles:** Each profile (saved as `data/profiles/{uuid}.json`) contains:
- Sounds (with local path or YouTube URL + cached path)
- Tracks configuration
- Key bindings

**Profile Switch:** All playing sounds are stopped (`stopAllSounds()`) before loading a new profile.

**Global Config:** `data/config.json` stores app-wide settings (master volume, auto-momentum, key detection enabled, master stop shortcut, auto-momentum shortcut, key detection shortcut, crossfade duration, key cooldown, audio device).

**Auto-save:** Configuration saves automatically on changes (with 1-second debounce), on app close, and every 5 minutes.

### Import/Export

**Export Format:** `.ktm` files (ZIP archives containing `profile.json`, `metadata.json`, and a `sounds/` folder with audio files).

**Export UX:** Export uses a global Zustand store (`exportStore.ts`) so progress persists beyond the Settings modal lifecycle. A floating progress bar (`ExportProgress.tsx`) shows current/total files and filename, with a cancel button. Export emits `export_progress` events from Rust to frontend via Tauri.

**Export Safety:**
- **Temp file pattern:** Export writes to `{output}.tmp` first, then renames to final path on success. This prevents corrupt files if the process is interrupted.
- **Tracking file:** Before export starts, the temp path is written to `data/export_in_progress.txt`. On success, this file is deleted. On next startup, `cleanup_interrupted_export()` checks for this file and deletes the orphaned temp file.
- **Cancellation:** Uses `AtomicBool` static (`EXPORT_CANCELLED`) checked between each file copy in the export loop. On cancel: drops zip writer, deletes temp file and tracking file, returns "Export cancelled" error.
- **Window close interception:** `onCloseRequested` handler warns the user if export is in progress. If they confirm close, `cleanupExportTemp()` is called before closing.

**Tauri 2 Permissions:** The `capabilities/default.json` must include `core:window:allow-destroy` and `core:window:allow-close` for `onCloseRequested` to work correctly (these are NOT included in `core:window:default`).

**Import Process:** Extracts the .ktm file, assigns new UUID to avoid conflicts, copies audio files to `data/imported_sounds/{new_id}/`, and updates file paths.

**Legacy Import:** Converts save files from the old KeyToMusic (Unity-based) version into new profiles. The legacy format uses a `Sounds` array with numeric Windows virtual key codes (`Key`), character labels (`UserKeyChar`), and `SoundInfos` containing `uniqueId`, `soundPath`, `soundName`, and `soundMomentum`. Conversion maps VK codes to web `KeyCode` strings (65-90→KeyA-KeyZ, 48-57→Digit0-Digit9, 112-123→F1-F12, plus OEM keys), creates `SoundSource::Local` entries (normalizing `/` to `\` on Windows), assigns all bindings to a default "OST" track, and sets volume to 1.0, loop mode to "off", duration to 0.0 (computed on load). The UI button is in the Settings modal Import/Export section.

## Development Commands

Since this is a new project, here are the setup and common commands:

### Initial Setup
```bash
# Create Tauri project with React + TypeScript template
npm create tauri-app@latest keytomusic -- --template react-ts

# Install dependencies
npm install zustand
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p

# Add Rust dependencies to src-tauri/Cargo.toml:
# tauri, serde, serde_json, rodio, rdev, tokio, uuid, walkdir, sanitize-filename
```

### Development
```bash
# Run in development mode (hot reload)
npm run tauri dev

# Build frontend only
npm run build

# Format Rust code
cargo fmt --manifest-path src-tauri/Cargo.toml

# Lint Rust code
cargo clippy --manifest-path src-tauri/Cargo.toml

# Run Rust tests
cargo test --manifest-path src-tauri/Cargo.toml
```

### Build
```bash
# Build production app (creates installer)
npm run tauri build
```

## Key Implementation Notes

### Tauri Commands Pattern

All backend functionality is exposed via Tauri commands in `src-tauri/src/commands.rs`:

```rust
#[tauri::command]
fn play_sound(track_id: String, sound_id: String, file_path: String, start_position: f64, sound_volume: f32) -> Result<(), String>;

#[tauri::command]
fn stop_sound(track_id: String) -> Result<(), String>;

#[tauri::command]
fn stop_all_sounds() -> Result<(), String>;

#[tauri::command]
fn set_sound_volume(track_id: String, sound_id: String, volume: f32) -> Result<(), String>;

#[tauri::command]
async fn get_audio_duration(path: String) -> Result<f64, String>;

#[tauri::command]
async fn preload_profile_sounds(sounds: Vec<SoundPreloadEntry>) -> Result<HashMap<String, f64>, String>;

#[tauri::command]
fn list_audio_devices() -> Vec<String>;

#[tauri::command]
fn set_audio_device(device: Option<String>) -> Result<(), String>;

#[tauri::command]
async fn add_sound_from_youtube(url: String, download_id: String) -> Result<Sound, String>;

// Legacy import commands
#[tauri::command]
async fn pick_legacy_file() -> Result<Option<String>, String>;

#[tauri::command]
async fn import_legacy_save(path: String) -> Result<Profile, String>;

// Error handling commands
#[tauri::command]
fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo>;

#[tauri::command]
async fn pick_audio_file() -> Result<Option<String>, String>;

#[tauri::command]
async fn pick_audio_files() -> Result<Vec<String>, String>;

#[tauri::command]
fn get_logs_folder() -> Result<String, String>;

#[tauri::command]
fn get_data_folder() -> Result<String, String>;

#[tauri::command]
fn open_folder(path: String) -> Result<(), String>;
// ... etc
```

Frontend invokes these via:
```typescript
import { invoke } from '@tauri-apps/api';
await invoke('play_sound', { trackId, soundId, startPosition });
```

### Backend → Frontend Events

The backend emits events for real-time updates:

```rust
// Rust side
app_handle.emit_all("sound_started", payload).ok();
```

```typescript
// React side
import { listen } from '@tauri-apps/api/event';
useEffect(() => {
  const unlisten = listen('sound_started', (event) => {
    // Handle event
  });
  return () => { unlisten.then(f => f()); };
}, []);
```

Event types: `sound_started`, `sound_ended`, `playback_progress`, `key_pressed`, `master_stop_triggered`, `youtube_download_progress` (with `downloadId`, `status`, `progress`), `sound_not_found` (with `soundId`, `path`, `trackId`), `audio_error` (with `message`), `export_progress`, `toggle_key_detection`, `toggle_auto_momentum`.

### Error Handling

**Error Sound:** When a sound file is not found during playback, the audio engine plays a short error sound (`resources/sounds/error.mp3`) at 50% master volume via a fire-and-forget sink (`sink.detach()`). The error sound path is resolved from the Tauri resource directory at startup via `SetErrorSoundPath`.

**Sound Not Found Flow:**
1. `play_sound` command checks file existence
2. If missing: calls `play_error_sound()`, emits `sound_not_found` event with `{soundId, path, trackId}`, returns Err
3. Frontend `useAudioEvents` hook receives event, adds entry to `errorStore.missingQueue`
4. `FileNotFoundModal` displays entries one at a time from the queue

**FileNotFoundModal (`src/components/Errors/FileNotFoundModal.tsx`):** Queue-based modal showing sound name, file path, and remaining error count. Actions differ by source type:
- **Local sounds:** "Locate File" (native file picker via `pick_audio_file`) / "Remove" / "Skip"
- **YouTube sounds:** "Re-download" (calls `addSoundFromYoutube`) / "Remove" / "Skip"
- "Skip All" dismisses remaining queue entries

**Profile Verification:** On profile load, `verifyProfileSounds(profile)` checks all sound file paths. Missing files are added to the error store queue, triggering the FileNotFoundModal.

**Error Messages (`src/utils/errorMessages.ts`):** Maps raw error strings to user-friendly messages via regex patterns. Used by toast notifications for `audio_error` events.

**Toast Notifications:** Non-blocking errors (audio device issues, playback failures) are shown as toast notifications via `useToastStore`. The `useKeyDetection` hook shows toasts for non-file-not-found playback errors.

### Logging

**Infrastructure:** Uses `tracing` + `tracing-subscriber` + `tracing-appender` crates. Daily rolling log files are written to `{app_data}/logs/keytomusic.log`. Configurable via `RUST_LOG` env var (default: `info`). The non-blocking writer guard is held for the program's lifetime.

**Logged Events:**
- App startup (info)
- Error sound load success/failure (info/warn)
- Sound not found (warn with file path, track ID, sound ID)
- Audio errors (error)
- Config/storage issues (warn)

**Open Logs:** Settings modal includes an "Open Logs Folder" button that calls `get_logs_folder()` and opens the directory via `@tauri-apps/plugin-shell`.

### Thread Safety

The audio engine runs in a separate thread. Use Tokio channels or Arc<Mutex<>> for state sharing between Tauri command handlers and the audio thread.

### Data Paths

Use platform-specific app data directories:
- Windows: `C:\Users\{user}\AppData\Roaming\KeyToMusic\`
- macOS: `/Users/{user}/Library/Application Support/KeyToMusic/`
- Linux: `/home/{user}/.local/share/keytomusic/`

### Recommended Development Order

1. **Foundations:** Types, storage, basic Tauri commands
2. **Audio Engine:** Basic playback, tracks, crossfade, buffering
3. **Key Detection:** Global keyboard capture with cooldown
4. **UI:** Layout, profiles, tracks view, sound management
5. **YouTube:** Download and cache system
6. **Import/Export:** .ktm file handling
7. **Polish:** Error handling, testing, optimization

## UI Design Considerations

- **Dark Theme:** Default color palette with indigo/violet accents
- **Minimum Window Size:** 800x600 pixels
- **Layout:** Header (logo, master volume, settings, window controls) + Sidebar (profiles, controls, now playing) + Main Content (track view, key assignments, sound details)
- **Key Assignment Grid:** Visual representation of assigned keys with custom name (or first sound name) and total sound count. Keys can be deleted via the SoundDetails panel.
- **Now Playing:** Seekable progress slider (drag-then-release pattern) with stop button per track. Updates position in real-time via `playback_progress` events.
- **AddSoundModal:** Files are added via native file picker ("Add Files" button using `pickAudioFiles`) or drag & drop. No manual path text input. Per-file momentum editors with number input, slider, and play/stop preview. Multiple sounds are grouped by key before creating bindings. Key assignment uses cycling: if fewer keys than sounds are provided (e.g., "ab" with 5 sounds), keys cycle (a,b,a,b,a). A single key assigns all sounds to that key.
- **Resizable SoundDetails Panel:** Divider bar between KeyGrid and SoundDetails with drag-to-resize (min 120px, default 256px). Uses mousedown/mousemove/mouseup pattern with body cursor override.
- **Drag & Drop:** Support for adding multiple audio files at once with bulk assignment to specified keys. Dropping files while AddSoundModal is open appends them to the existing file list (uses `processedFilesRef` to distinguish mount vs. subsequent prop changes, React StrictMode safe).

## Implemented Features (Phase 8)

### Profile Duplication ✅
- Duplicate an existing profile via button in ProfileSelector
- Creates a copy with new UUID, appends "(Copy)" to name
- Backend: `duplicate_profile(id, new_name)` command in `storage/profile.rs`
- Frontend: Duplicate button (SVG icon) in ProfileSelector, calls `profileStore.duplicateProfile()`

### Combined Key Shortcuts (Modifiers) ✅
- **Backend:** `detector.rs` emits combined codes ("Ctrl+Shift+KeyA")
- **Frontend detection:** `useKeyDetection.ts` matches combined codes with fallback to base key
- **Frontend UI:** AddSoundModal uses `KeyCaptureSlot` for key capture with full modifier support
- Modifier order: Ctrl > Shift > Alt > Key
- Shift+X on existing "X" binding still triggers momentum (backward compatible)
- `keyMapping.ts` has helpers: `buildKeyCombo()`, `parseKeyCombo()`, `checkShortcutConflicts()`

### Undo/Redo System ✅
- Ctrl+Z for Undo, Ctrl+Y / Cmd+Shift+Z for Redo
- Implemented via `historyStore.ts` with past/future stacks
- Undoable: sound add/delete/modify, binding add/delete/modify, track add/delete
- Non-undoable: profile creation/deletion, YouTube downloads, duration preloads, playback index
- Maximum 50 history entries to limit memory usage
- Toast feedback: "Undo: {actionName}" / "Redo: {actionName}"
- History cleared on profile switch
- `useUndoRedo.ts` hook integrated in `App.tsx`

### Multi-Key Chords ✅ (Phase 8.4)

Support pressing multiple non-modifier keys simultaneously (like piano chords), using a combo detection system inspired by fighting games (Street Fighter, Tekken).

**How it works:**
- Uses a Trie (prefix tree) structure for optimal detection
- Trigger immediately when combo reaches a "leaf" (no further extensions possible)
- Timer only when extensions exist (conditional latency)

**Example with bindings: A, A+Z, A+Z+E**
```
A pressed → Extensions possible (A+Z, A+Z+E) → Start 30ms timer
Z pressed → Extensions possible (A+Z+E) → Continue timer
E pressed → Leaf node (no A+Z+E+*) → TRIGGER IMMEDIATELY "A+Z+E"
```

**Latency optimization:**
- 0ms if key is a leaf (no extensions in profile)
- 0ms if current combo is a leaf (trigger immediately)
- 30-50ms only when extensions are possible

**Configuration:**
- `config.chordWindowMs`: 20-100ms (configurable in Settings, default: 30ms)

**Format:** Modifiers first (Ctrl > Shift > Alt), then base keys sorted alphabetically.
- `"KeyZ+KeyA"` → normalized to `"KeyA+KeyZ"`

**Key files:**
- `src-tauri/src/keys/chord.rs` - ComboTrie and ChordDetector
- `src-tauri/src/keys/detector.rs` - Integration with global key detection
- `src/utils/keyMapping.ts` - `normalizeCombo()`, `buildComboFromPressedKeys()`

### Configurable Momentum Modifier ✅ (Phase 8.5)

Users can choose which modifier key triggers momentum playback:
- **Options:** Shift (default), Ctrl, Alt, or Disabled
- **Config field:** `momentumModifier: "Shift" | "Ctrl" | "Alt" | "None"`
- **UI:** Dropdown in Settings under "Key Detection" section
- **Rule:** Exact binding match takes priority (e.g., "Ctrl+A" binding triggers normally, not "A" with momentum)
- **Use case:** Solves Numpad+Shift hardware limitation (Shift+Numpad4 sends ArrowLeft on most keyboards)

**Conflict detection:** Warns users when shortcuts conflict with momentum modifier + bound keys:
- When changing momentum modifier: checks if existing shortcuts would override momentum
- When setting a shortcut: checks if it uses the momentum modifier + a bound key
- Shows toast warnings for immediate feedback on changes
- Persistent warning icons with tooltips:
  - Next to conflicting shortcuts in Settings
  - Next to Momentum Modifier dropdown if conflicts exist
  - On affected keys in KeyGrid (visible after closing Settings)

**Key files:**
- `src/types/index.ts` - `MomentumModifier` type
- `src/stores/settingsStore.ts` - `setMomentumModifier()` action
- `src-tauri/src/types.rs` - `MomentumModifier` enum
- `src-tauri/src/commands.rs` - `update_config` handles momentum modifier persistence
- `src/hooks/useKeyDetection.ts` - `hasMomentumModifier()` check
- `src/components/Settings/SettingsModal.tsx` - Dropdown UI with conflict detection (organized in sections with scrolling)
- `src/components/Keys/KeyGrid.tsx` - Warning icons on conflicting keys
- `src/components/common/WarningTooltip.tsx` - Reusable warning icon with tooltip
- `src/utils/keyMapping.ts` - `findMomentumConflicts()`, `getKeyMomentumConflict()` utilities

## Known Limitations

### Numpad + Shift
When NumLock is ON and Shift is pressed with a numpad key, the OS sends the alternate key (ArrowLeft, End, etc.) instead of "Shift+Numpad4". This is standard keyboard hardware behavior, not a bug. **Workaround:** Go to Settings > Key Detection > Momentum Modifier and select "Ctrl" or "Alt" instead of "Shift".

## Technical Limits

- **Max Tracks:** 20
- **Momentum Seeking:** Instant via symphonia byte-level seek (no pre-loading needed)
- **Cooldown Range:** 0ms - 5000ms
- **Crossfade Range:** 100ms - 2000ms
- **Supported Audio Formats:** MP3, WAV, OGG, FLAC, AAC
- **Audio Thread:** Dynamic timeout (200ms idle, 16ms when playing) to reduce CPU usage

## Dependencies (Cargo.toml excerpt)

```toml
[dependencies]
tauri = { version = "2", features = ["shell-open"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rodio = "0.19"         # Audio playback
cpal = "0.15"          # Audio device enumeration
symphonia = { version = "0.5", features = ["mp3", "flac", "ogg", "wav", "pcm", "aac", "isomp4"] }  # Fast seeking + M4A
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4"] }
walkdir = "2"          # File traversal
sanitize-filename = "0.5"
zip = { version = "2", default-features = false, features = ["deflate"] }  # ffmpeg ZIP extraction
tracing = "0.1"        # Structured logging
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }  # Log formatting
tracing-appender = "0.2"  # Daily rolling log files

# Platform-specific key detection:
[target."cfg(target_os = \"linux\")".dependencies]
rdev = { git = "https://github.com/fufesou/rdev" }  # Global key detection (Linux only)

[target."cfg(target_os = \"windows\")".dependencies]
windows = { version = "0.58", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_UI_Input", "Win32_Devices_HumanInterfaceDevice", "Win32_Graphics_Gdi"] }  # Raw Input API

# macOS uses native CGEventTap via CoreGraphics FFI (no external crate)
```

### Frontend Dependencies (npm)

```json
"@tauri-apps/plugin-shell": "^2.0.0"  // Opening folders (logs, data)
```

## External Requirements

- **yt-dlp:** Auto-downloaded to `{app_data}/bin/yt-dlp.exe` on first YouTube download. No user installation needed.
- **ffmpeg:** Auto-downloaded to `{app_data}/bin/ffmpeg.exe` on first YouTube download (required for M4A remuxing). No user installation needed.

## Reference

See `KeyToMusic_Technical_Specification.md` for comprehensive implementation details, data models, UI mockups, and pseudocode algorithms.
