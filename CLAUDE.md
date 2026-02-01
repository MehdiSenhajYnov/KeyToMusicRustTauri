# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

KeyToMusic is a Tauri-based desktop soundboard application designed for manga reading. It provides global keyboard detection to trigger assigned sounds without interrupting the reading experience. The app supports multi-track audio playback with crossfading, YouTube downloads, momentum (start position), multiple loop modes, waveform visualization, and an intelligent discovery system for finding new sounds.

**Platforms:** Windows 10/11, macOS 10.15+, Linux (Ubuntu, Fedora, Arch)

## Tech Stack

- **Framework:** Tauri 2.x (Rust backend + React frontend)
- **Frontend:** React 18+ with TypeScript, Tailwind CSS, Zustand (state management)
- **Backend:** Rust with rodio/cpal (audio), symphonia (fast seeking + waveform analysis), platform-specific global key detection
- **External Tools:** yt-dlp (YouTube downloads, search, playlist/Mix fetching)

## Project Structure

```
keytomusic/
├── src/                          # React/TypeScript frontend
│   ├── components/               # UI components (Layout, Tracks, Sounds, Keys, etc.)
│   │   ├── Discovery/           # DiscoveryPanel (YouTube Mix recommendations)
│   │   ├── Errors/              # FileNotFoundModal
│   │   ├── common/              # WaveformDisplay, WarningTooltip
│   │   └── ConfirmDialog.tsx    # Custom confirm modal (replaces browser confirm())
│   ├── stores/                   # Zustand state management
│   │   ├── profileStore.ts      # Profile CRUD, sounds, bindings, tracks, undo/redo
│   │   ├── audioStore.ts        # Playing tracks state (trackId → soundId + position)
│   │   ├── settingsStore.ts     # AppConfig management
│   │   ├── discoveryStore.ts    # Discovery suggestions, pre-download, carousel state
│   │   ├── historyStore.ts      # Undo/redo stacks (max 50 entries)
│   │   ├── errorStore.ts        # Missing sound file queue
│   │   ├── exportStore.ts       # Export progress tracking
│   │   ├── toastStore.ts        # Toast notifications
│   │   └── confirmStore.ts      # Custom confirm dialog state
│   ├── hooks/                    # Custom React hooks
│   │   ├── useAudioEvents.ts    # Listens to sound_started/ended/progress events
│   │   ├── useKeyDetection.ts   # Key press handling + momentum logic
│   │   ├── useDiscovery.ts      # Discovery event listeners (progress, partial, complete)
│   │   ├── useDiscoveryPredownload.ts  # Smart pre-download orchestration
│   │   ├── useUndoRedo.ts       # Ctrl+Z / Ctrl+Y keyboard handler
│   │   └── useTextInputFocus.ts # Auto-disable key detection on text inputs
│   ├── types/                    # TypeScript type definitions
│   │   └── index.ts             # All types (Sound, Profile, AppConfig, WaveformData, etc.)
│   └── utils/                    # Frontend utilities
│       ├── tauriCommands.ts     # Type-safe wrappers for all Tauri invoke() calls
│       ├── keyMapping.ts        # Key code display, combos, shortcuts, conflicts
│       ├── profileAnalysis.ts   # Profile mode detection + smart auto-assignment
│       ├── errorMessages.ts     # Raw error → user-friendly message mapping
│       ├── fileHelpers.ts       # Audio file validation, path utilities
│       ├── soundHelpers.ts      # getSoundFilePath, findLeastUsedTrack utilities
│       └── inputHelpers.ts      # Text input detection for auto-disable
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # Tauri entry point, logging init, event forwarding
│   │   ├── commands.rs           # All Tauri commands exposed to frontend
│   │   ├── state.rs              # AppState (config, audio engine, key detector, caches)
│   │   ├── types.rs              # Shared Rust types (Sound, Profile, AppConfig, etc.)
│   │   ├── audio/                # Audio engine, tracks, crossfade, symphonia seeking
│   │   │   ├── engine.rs         # AudioEngine with command channel, track management
│   │   │   ├── track.rs          # AudioTrack (sink, crossfade, playback state)
│   │   │   ├── crossfade.rs      # Crossfade curve logic
│   │   │   ├── symphonia_source.rs # Custom rodio::Source with byte-level seeking
│   │   │   ├── analysis.rs       # Waveform computation, momentum detection, WaveformCache
│   │   │   └── buffer.rs         # Duration reading via symphonia headers
│   │   ├── keys/                 # Global keyboard detection & mapping (platform-specific)
│   │   │   ├── detector.rs       # Key detector with cooldown, shortcuts, chord integration
│   │   │   ├── mapping.rs        # KeyEvent types, key code conversions
│   │   │   ├── chord.rs          # Multi-key chord detection (Trie-based)
│   │   │   ├── macos_listener.rs # macOS-only CGEventTap implementation
│   │   │   └── windows_listener.rs # Windows-only Raw Input API implementation
│   │   ├── discovery/            # YouTube Mix recommendation engine
│   │   │   ├── engine.rs         # DiscoveryEngine: seed extraction, aggregation, scoring
│   │   │   ├── mix_fetcher.rs    # Fetches YouTube Mix playlists via yt-dlp
│   │   │   └── cache.rs          # Per-profile discovery cache with seed hash
│   │   ├── youtube/              # YouTube downloader, cache, search, ffmpeg/yt-dlp managers
│   │   │   ├── downloader.rs    # Core download logic, retry, canonical URLs
│   │   │   ├── cache.rs         # YouTubeCache: file-to-profile mapping, cleanup
│   │   │   ├── search.rs        # YouTube search + playlist fetch via yt-dlp
│   │   │   ├── yt_dlp_manager.rs # Auto-download and manage yt-dlp binary
│   │   │   └── ffmpeg_manager.rs # Auto-download and manage ffmpeg binary
│   │   ├── import_export/        # .ktm file handling
│   │   └── storage/              # Profile & config persistence
│   └── Cargo.toml
├── data/                         # Runtime user data (profiles, cache, config, logs)
│   ├── profiles/                 # Profile JSON files ({uuid}.json)
│   ├── cache/                    # YouTube audio cache + waveforms.json
│   ├── discovery/                # Discovery cache per profile ({profile_id}.json)
│   ├── bin/                      # Auto-downloaded yt-dlp and ffmpeg binaries
│   ├── imported_sounds/          # Sounds from .ktm imports
│   ├── logs/                     # Daily rolling log files
│   └── config.json               # Global app configuration
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

**Audio Event Forwarding:** A dedicated polling thread in `main.rs` drains audio engine events every 100ms and emits them as Tauri events (`sound_started`, `sound_ended`, `playback_progress`) to the frontend. The audio engine itself only generates `playback_progress` events every 250ms to reduce overhead.

**Duration Reading:** Audio file durations are computed via symphonia header reading (`n_frames / sample_rate`), providing instant results without decoding. Falls back to rodio sample-counting only when headers lack frame count info. Durations are computed in batch on profile load via `preload_profile_sounds` and applied in a single batched state update.

**Crossfade:** 500ms default duration with a custom curve that creates a brief silence gap between outgoing and incoming sounds (35%-65% of the duration). Crossfade only occurs between sounds on the same track.

**Momentum:** Each sound has a momentum property (start position in seconds). Sounds can start from 0:00 or from their momentum position. Auto-Momentum mode or the configured momentum modifier key (Shift/Ctrl/Alt, configurable in Settings) triggers momentum start.

**Playback via Symphonia:** All audio playback uses a custom `SymphoniaSource` that implements `rodio::Source`. This provides consistent format support (MP3, M4A/AAC, OGG, FLAC, WAV) and instant byte-level seeking for momentum (O(1) for CBR, O(log n) for VBR). The `isomp4` symphonia feature is required for M4A files from YouTube downloads.

**Audio Device Selection:** Users can select a specific output device from Settings, or follow the system default (None). The device list is provided via `cpal` host enumeration. The selected device is persisted in `config.json` as `audioDevice`.

**Seamless Device Switching:** When the audio device changes (either via user selection in Settings or OS default device change), playback resumes automatically on the new device. The engine captures each playing track's state (file_path, position, volumes), rebuilds the OutputStream, then immediately resumes all tracks at their captured positions with no crossfade. This produces a brief gap (<50ms) but no `SoundEnded` events are emitted, so the frontend sees uninterrupted playback. The `AudioTrack` struct stores `file_path: Option<String>` to enable this resume capability.

**Device Polling:** When following the system default (audioDevice = None), the audio thread polls for default device changes every 3 seconds via `cpal::default_host().default_output_device()`. If the device name changes, the seamless switch is triggered automatically.

### Waveform Analysis

**Purpose:** Visual audio energy waveform (RMS-based) for momentum editing. Shows audio structure (intro, buildup, climax) so users can visually set the start point instead of blind scrubbing.

**Backend (`src-tauri/src/audio/analysis.rs`):**

- **`WaveformData`** struct: `points: Vec<f32>` (10-2000 RMS samples normalized 0.0-1.0), `duration`, `sample_rate`, `suggested_momentum: Option<f64>`
- **`compute_waveform_sampled()`** - Fast method (~40x faster than full decode). Seeks to N positions across the file, decodes a small window at each, computes RMS. Works on seekable formats (M4A, MP3, FLAC). Falls back to full decode if seeking unsupported.
- **`compute_waveform()`** - Full decode fallback. Decodes entire file to mono, computes RMS per segment, normalizes and smooths with 3-point moving average.
- **`detect_momentum_point()`** - Analyzes waveform to auto-suggest optimal start position. Skips first 5%, finds first significant amplitude rise (gradient > 0.05) after a quiet section (average < 0.15 over 5% window). Returns timestamp in seconds.

**WaveformCache (`analysis.rs`):**
- In-memory LRU cache (max 50 entries) with disk persistence to `data/cache/waveforms.json`
- File-modification-time invalidation: if source audio file is newer than cached entry, recompute
- Atomic disk writes (`.json.tmp` → `.json`)
- Initialized at startup via `WaveformCache::new_with_disk(50, cache_path)`

**Frontend (`src/components/common/WaveformDisplay.tsx`):**
- Dual-canvas architecture: static canvas (waveform bars + momentum markers) + cursor canvas (playback position line)
- Static canvas only redraws when waveform data, momentum, or suggested momentum change
- Cursor canvas redraws on playback progress ticks (decoupled from expensive waveform rendering)
- Draggable momentum marker, visual suggested momentum indicator
- "Accept suggestion" button to apply auto-detected momentum

**Tauri Commands:**
- `get_waveform(path, num_points)` → `WaveformData` - Single file waveform
- `get_waveforms_batch(entries: Vec<{path, num_points}>)` → `HashMap<String, WaveformData>` - Batch waveform for multi-file imports

### Discovery System

**Purpose:** Intelligent sound recommendation engine that analyzes YouTube sounds in a profile and suggests related videos via YouTube Mix (Radio) playlists. Transforms manual sound hunting into one-click additions.

**Architecture:**
```
Seeds (existing YouTube sounds) → YouTube Mix fetch per seed → Cross-seed aggregation → Scoring by occurrence count → Top 30 suggestions → Pre-download + waveform analysis → One-click add to profile
```

**Backend (`src-tauri/src/discovery/`):**

- **`engine.rs`** - `DiscoveryEngine` with `generate_suggestions()`:
  - Extracts video IDs from profile's YouTube sounds as seeds (max 15)
  - Fetches YouTube Mix for each seed concurrently (`buffer_unordered(10)`)
  - Aggregates results into occurrence map: videos found in more seed mixes rank higher
  - Filters: duration 30-900s, excludes existing profile sounds
  - Returns top 30 `DiscoverySuggestion` structs
  - Supports streaming partial results via callback after each seed
  - Cancelable via `AtomicBool` flag

- **`mix_fetcher.rs`** - `fetch_mix(video_id)`:
  - Uses yt-dlp: `yt-dlp "youtube.com/watch?v={id}&list=RD{id}" --flat-playlist --dump-json --no-download`
  - Parses JSON output line-by-line for title, duration, channel, thumbnail
  - 15-second socket timeout per seed
  - Returns empty Vec on error (best-effort, non-blocking)

- **`cache.rs`** - `DiscoveryCache`:
  - Persists to `data/discovery/{profile_id}.json`
  - Stores `seed_hash` to detect when profile's YouTube sounds change
  - Tracks `dismissed_ids` (user-rejected suggestions)
  - Atomic write pattern, cleaned up on profile deletion

**Frontend (`src/stores/discoveryStore.ts`):**

- **`EnrichedSuggestion`** extends `DiscoverySuggestion` with:
  - Pre-download tracking: `predownloadStatus` ("idle"/"downloading"/"ready"/"error"), `cachedPath`, `downloadProgress`
  - Waveform data (computed after pre-download)
  - Auto-assignment: `suggestedKey`, `suggestedTrackId`, `suggestedMomentum`
  - Preview state: `isPreviewPlaying`

- **Pagination:** Reveals 10 suggestions initially, +5 per scroll
- **Carousel navigation:** `goToNext()`, `goToPrev()`, `goToIndex()`

**Frontend Hooks:**
- **`useDiscovery.ts`** - Listens for `discovery_started`, `discovery_progress`, `discovery_partial`, `discovery_complete`, `discovery_error` events
- **`useDiscoveryPredownload.ts`** - Pre-downloads suggestions around current carousel position. Asymmetric window: [current-2, ..., current+3]. Max 3 concurrent downloads. Updates waveform cache after completion.

**Frontend Component (`src/components/Discovery/DiscoveryPanel.tsx`):**
- Carousel UI for browsing suggestions with navigation arrows
- Auto-triggers discovery on profile load if profile has YouTube sounds + no cached results
- Streaming display: shows partial results as they arrive during generation
- Preview playback of pre-downloaded suggestions
- Dismiss suggestions (persisted in cache)
- One-click add to profile with auto-assigned key/track/momentum

**Smart Auto-Assignment (`src/utils/profileAnalysis.ts`):**
- `analyzeProfile()` - Detects profile mode: "single-sound" (avg ≤ 2 sounds/binding) or "multi-sound"
- `computeAutoAssign()` - Context-aware key/track assignment:
  - **Single-sound mode:** Next available key from ordered list, assign to least-used track
  - **Multi-sound mode:** Find binding with most seed video ID matches (similar suggestions cluster together), use that binding's track

**Tauri Commands:**
- `start_discovery(profile_id)` → `Vec<DiscoverySuggestion>` - Generate suggestions (async, emits events)
- `get_discovery_suggestions(profile_id)` → `Option<Vec<DiscoverySuggestion>>` - Load cached
- `dismiss_discovery(profile_id, video_id)` → `()` - Mark suggestion dismissed
- `cancel_discovery()` → `()` - Abort ongoing generation
- `predownload_suggestion(url, video_id, download_id)` → `PredownloadResult` - Download + duration + waveform in one call

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

**Cooldown:** Global 200ms cooldown (configurable, 0-5000ms range) applies to ALL key presses to prevent accidental spam.

**Global Shortcuts:** Master Stop, Auto-Momentum toggle, and Key Detection toggle shortcuts all work regardless of key detection state (both foreground and background). They are checked before the `enabled` guard in `detector.rs`. Only sound-triggering key presses are blocked when detection is disabled.

**Master Stop:** Configurable key combination (default: Ctrl+Shift+S) stops all sounds on all tracks. Works both in background (via rdev) and in foreground (via browser keyboard handler with pressed keys tracking).

**Frontend Key Detection Guard:** The `handleKeyPress` function checks `config.keyDetectionEnabled` before processing any key event, ensuring the toggle actually disables detection.

**Auto-disable:** Key detection is automatically disabled (via `useTextInputFocus` hook calling `setKeyDetection(false)` on the backend) when text-like input fields are focused in the UI. Non-text inputs like `<input type="range">` (sliders) and `<input type="checkbox">` are excluded so that key detection continues working when interacting with volume sliders, momentum sliders, etc. The `isTextInput()` helper checks the input's `type` attribute to distinguish text inputs from non-text controls.

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

**YouTube Search:** Integrated search via `search_youtube(query, max_results)` command. Uses `yt-dlp ytsearch{N}:{query}` to search YouTube directly from the app.

**Playlist Support:** `fetch_playlist(url)` command fetches YouTube playlist metadata (title, entries with video IDs, durations, channels). Used both for user playlist imports and internally by the discovery system for Mix fetching. Configurable via `playlistImportEnabled` in config.

**ffmpeg Auto-Install:** YouTube provides DASH fragmented MP4 audio which requires ffmpeg to remux into proper M4A. The app auto-downloads ffmpeg from `yt-dlp/FFmpeg-Builds` GitHub releases on first YouTube download. ffmpeg is stored in the app's `bin/` directory alongside yt-dlp, and yt-dlp auto-detects it via `--ffmpeg-location`. Binary presence can be checked via `check_yt_dlp_installed()` and `check_ffmpeg_installed()` commands.

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

**Global Config (`data/config.json`):**
```typescript
{
  masterVolume: number,              // 0.0-1.0
  autoMomentum: boolean,
  keyDetectionEnabled: boolean,
  masterStopShortcut: KeyCode[],     // e.g. ["ControlLeft", "ShiftLeft", "KeyS"]
  autoMomentumShortcut: KeyCode[],
  keyDetectionShortcut: KeyCode[],
  crossfadeDuration: number,         // ms (default: 500)
  keyCooldown: number,               // ms (default: 200)
  currentProfileId: string | null,
  audioDevice: string | null,        // null = follow system default
  chordWindowMs: number,             // 20-100ms (default: 30)
  momentumModifier: MomentumModifier, // "Shift" | "Ctrl" | "Alt" | "None"
  playlistImportEnabled: boolean,
}
```

**Atomic Writes:** Both `save_config()` and `save_profile()` use the write-to-temp-then-rename pattern (`file.json.tmp` → `file.json`) to prevent corruption if the process is interrupted mid-write.

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

### App State (`src-tauri/src/state.rs`)

```rust
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub audio_engine: AudioEngineHandle,
    pub key_detector: KeyDetector,
    pub youtube_cache: Arc<Mutex<YouTubeCache>>,
    pub waveform_cache: Arc<Mutex<WaveformCache>>,  // LRU cache, disk-persisted
    pub discovery_cancel: Arc<AtomicBool>,           // Cancellation flag for discovery
}
```

### Tauri Commands Pattern

All backend functionality is exposed via Tauri commands in `src-tauri/src/commands.rs`:

```rust
// --- Configuration ---
fn get_config() -> Result<AppConfig, String>;
fn update_config(updates: serde_json::Value) -> Result<(), String>;
fn set_profile_bindings(bindings: Vec<String>) -> Result<(), String>;

// --- Profiles ---
fn list_profiles() -> Result<Vec<ProfileSummary>, String>;
fn create_profile(name: String) -> Result<Profile, String>;
fn load_profile(id: String) -> Result<Profile, String>;
fn save_profile(profile: Profile) -> Result<(), String>;
fn delete_profile(id: String) -> Result<(), String>;
fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String>;

// --- Audio Playback ---
fn play_sound(track_id, sound_id, file_path, start_position, sound_volume) -> Result<(), String>;
fn stop_sound(track_id: String) -> Result<(), String>;
fn stop_all_sounds() -> Result<(), String>;
fn set_master_volume(volume: f32) -> Result<(), String>;
fn set_track_volume(track_id: String, volume: f32) -> Result<(), String>;
fn set_sound_volume(track_id: String, sound_id: String, volume: f32) -> Result<(), String>;
async fn get_audio_duration(path: String) -> Result<f64, String>;
async fn preload_profile_sounds(sounds: Vec<SoundPreloadEntry>) -> Result<HashMap<String, f64>, String>;

// --- Audio Devices ---
fn list_audio_devices() -> Vec<String>;
fn set_audio_device(device: Option<String>) -> Result<(), String>;

// --- Key Detection ---
fn set_key_detection(enabled: bool) -> Result<(), String>;
fn set_master_stop_shortcut(keys: Vec<String>) -> Result<(), String>;
fn set_key_cooldown(cooldown_ms: u32) -> Result<(), String>;

// --- Waveform Analysis ---
async fn get_waveform(path: String, num_points: usize) -> Result<WaveformData, String>;
async fn get_waveforms_batch(entries: Vec<WaveformBatchEntry>) -> Result<HashMap<String, WaveformData>, String>;

// --- YouTube ---
async fn add_sound_from_youtube(url: String, download_id: String) -> Result<Sound, String>;
async fn search_youtube(query: String, max_results: u32) -> Result<Vec<YoutubeSearchResult>, String>;
async fn fetch_playlist(url: String) -> Result<YoutubePlaylist, String>;
async fn check_yt_dlp_installed() -> Result<bool, String>;
async fn install_yt_dlp() -> Result<(), String>;
async fn check_ffmpeg_installed() -> Result<bool, String>;
async fn install_ffmpeg() -> Result<(), String>;

// --- Discovery ---
async fn start_discovery(profile_id: String) -> Result<Vec<DiscoverySuggestion>, String>;
fn get_discovery_suggestions(profile_id: String) -> Result<Option<Vec<DiscoverySuggestion>>, String>;
fn dismiss_discovery(profile_id: String, video_id: String) -> Result<(), String>;
fn cancel_discovery();
async fn predownload_suggestion(url: String, video_id: String, download_id: String) -> Result<PredownloadResult, String>;

// --- Import/Export ---
async fn export_profile(profile_id: String, output_path: String) -> Result<(), String>;
async fn import_profile(ktm_path: String) -> Result<String, String>;
async fn pick_save_location(default_name: String) -> Result<Option<String>, String>;
fn cleanup_export_temp();
fn cancel_export();
async fn pick_ktm_file() -> Result<Option<String>, String>;

// --- Legacy Import ---
async fn pick_legacy_file() -> Result<Option<String>, String>;
async fn import_legacy_save(path: String) -> Result<Profile, String>;

// --- Error Handling / Utility ---
fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo>;
async fn pick_audio_file() -> Result<Option<String>, String>;
async fn pick_audio_files() -> Result<Vec<String>, String>;
fn get_logs_folder() -> Result<String, String>;
fn get_data_folder() -> Result<String, String>;
fn open_folder(path: String) -> Result<(), String>;
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

**Event types:**

| Event | Payload | Description |
|-------|---------|-------------|
| `sound_started` | `{ trackId, soundId }` | Sound began playing on track |
| `sound_ended` | `{ trackId, soundId }` | Sound finished on track |
| `playback_progress` | `{ trackId, position }` | Playback position update (every 250ms) |
| `key_pressed` | `{ keyCode, withShift }` | Key detected by backend |
| `master_stop_triggered` | `{}` | Master stop shortcut activated |
| `toggle_key_detection` | `{}` | Key detection shortcut activated |
| `toggle_auto_momentum` | `{}` | Auto-momentum shortcut activated |
| `youtube_download_progress` | `{ downloadId, status, progress }` | YouTube download progress |
| `sound_not_found` | `{ soundId, path, trackId }` | Sound file missing during playback |
| `audio_error` | `{ message }` | Audio engine error |
| `export_progress` | `{ current, total, fileName }` | Export progress per file |
| `discovery_started` | `{}` | Discovery generation began |
| `discovery_progress` | `{ current, total, seedName }` | Per-seed progress during generation |
| `discovery_partial` | `Vec<DiscoverySuggestion>` | Streaming partial results after each seed |
| `discovery_complete` | `{ count }` | Discovery generation finished |
| `discovery_error` | `{ message }` | Discovery generation failed |

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

**Data directory structure:**
```
{app_data}/
├── config.json                    # Global app configuration
├── profiles/{uuid}.json           # Profile files
├── cache/
│   ├── cache_index.json           # YouTube audio cache index
│   ├── waveforms.json             # Waveform analysis cache (LRU, max 50)
│   └── {video_id}.m4a            # Cached YouTube audio files
├── discovery/{profile_id}.json    # Discovery cache per profile
├── bin/
│   ├── yt-dlp.exe                 # Auto-downloaded yt-dlp binary
│   └── ffmpeg.exe                 # Auto-downloaded ffmpeg binary
├── imported_sounds/{profile_uuid}/ # Sounds from .ktm imports
└── logs/keytomusic.log.*          # Daily rolling log files
```

## UI Design Considerations

- **Dark Theme:** Default color palette with indigo/violet accents
- **Minimum Window Size:** 800x600 pixels
- **Layout:** Header (logo, master volume, settings, window controls) + Sidebar (profiles, controls, now playing, discovery) + Main Content (track view, key assignments, sound details)
- **Key Assignment Grid:** Visual representation of assigned keys with custom name (or first sound name) and total sound count. Keys can be deleted via the SoundDetails panel.
- **Now Playing:** Seekable progress slider (drag-then-release pattern) with stop button per track. Updates position in real-time via `playback_progress` events.
- **AddSoundModal:** Files are added via native file picker ("Add Files" button using `pickAudioFiles`) or drag & drop. No manual path text input. Per-file momentum editors with number input, slider, and play/stop preview. Multiple sounds are grouped by key before creating bindings. Key assignment uses cycling: if fewer keys than sounds are provided (e.g., "ab" with 5 sounds), keys cycle (a,b,a,b,a). A single key assigns all sounds to that key.
- **Resizable SoundDetails Panel:** Divider bar between KeyGrid and SoundDetails with drag-to-resize (min 120px, default 256px). Uses mousedown/mousemove/mouseup pattern with body cursor override.
- **Drag & Drop:** Support for adding multiple audio files at once with bulk assignment to specified keys. Dropping files while AddSoundModal is open appends them to the existing file list (uses `processedFilesRef` to distinguish mount vs. subsequent prop changes, React StrictMode safe).
- **Discovery Panel:** Carousel UI in sidebar for browsing YouTube Mix recommendations. Shows waveform preview, auto-assigned key/track, and one-click add button.
- **Waveform Display:** Canvas-based visualization in SoundDetails and AddSoundModal. Shows audio energy structure with draggable momentum marker and suggested momentum indicator.

## Implemented Features

### Phase 8 - Core Features

#### Profile Duplication ✅
- Duplicate an existing profile via button in ProfileSelector
- Creates a copy with new UUID, appends "(Copy)" to name
- Backend: `duplicate_profile(id, new_name)` command in `storage/profile.rs`
- Frontend: Duplicate button (SVG icon) in ProfileSelector, calls `profileStore.duplicateProfile()`

#### Combined Key Shortcuts (Modifiers) ✅
- **Backend:** `detector.rs` emits combined codes ("Ctrl+Shift+KeyA")
- **Frontend detection:** `useKeyDetection.ts` matches combined codes with fallback to base key
- **Frontend UI:** AddSoundModal uses `KeyCaptureSlot` for key capture with full modifier support
- Modifier order: Ctrl > Shift > Alt > Key
- Shift+X on existing "X" binding still triggers momentum (backward compatible)
- `keyMapping.ts` has helpers: `buildKeyCombo()`, `parseKeyCombo()`, `checkShortcutConflicts()`

#### Undo/Redo System ✅
- Ctrl+Z for Undo, Ctrl+Y / Cmd+Shift+Z for Redo
- Implemented via `historyStore.ts` with past/future stacks
- Undoable: sound add/delete/modify, binding add/delete/modify, track add/delete
- Non-undoable: profile creation/deletion, YouTube downloads, duration preloads, playback index
- Maximum 50 history entries to limit memory usage
- Toast feedback: "Undo: {actionName}" / "Redo: {actionName}"
- History cleared on profile switch
- `useUndoRedo.ts` hook integrated in `App.tsx`

#### Multi-Key Chords ✅ (Phase 8.4)

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

#### Configurable Momentum Modifier ✅ (Phase 8.5)

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

### Smart Discovery Features ✅

#### YouTube Search Integration ✅
- Search YouTube directly from AddSoundModal via `search_youtube(query, max_results)` command
- Uses yt-dlp `ytsearch{N}:{query}` for backend search
- Results displayed with title, duration, channel, and one-click download

#### YouTube Playlist Support ✅
- `fetch_playlist(url)` command fetches playlist metadata
- Smart URL detection for video, video-in-playlist, and pure playlist URLs
- Configurable via `playlistImportEnabled` in settings

#### Waveform Visualization ✅
- RMS-based audio energy waveform displayed in SoundDetails and AddSoundModal
- Dual-canvas rendering: static waveform + dynamic playback cursor (performance optimized)
- Draggable momentum marker on waveform for visual momentum editing
- Backend: `get_waveform()` and `get_waveforms_batch()` commands
- LRU cache (50 entries) with disk persistence and file-mtime invalidation

#### Auto-Momentum Detection ✅
- `detect_momentum_point()` analyzes waveform to suggest optimal start position
- Skips intro silence, finds first significant energy rise
- Suggested momentum shown as visual marker on waveform, user can accept with one click
- `WaveformData.suggested_momentum` field carries the suggestion

#### YouTube Mix Discovery Engine ✅
- Intelligent recommendation based on existing YouTube sounds as "seeds"
- Fetches YouTube Mix (Radio) playlists per seed via yt-dlp
- Cross-seed aggregation: videos appearing in multiple mixes rank higher
- Top 30 suggestions filtered by duration (30-900s), excluding existing sounds
- Streaming partial results via `discovery_partial` events
- Cancelable generation with `cancel_discovery()` command
- Per-profile cache with seed hash to detect changes

#### Smart Pre-Download ✅
- Background pre-download of suggestions around carousel position
- Asymmetric window: [current-2, ..., current+3], max 3 concurrent
- `predownload_suggestion()` returns audio + duration + waveform in one call
- Waveform cached after computation

#### Smart Auto-Assignment ✅
- Profile mode detection: "single-sound" vs "multi-sound" based on avg sounds per binding
- Single-sound: linear key order, distribute to least-used track
- Multi-sound: cluster similar suggestions to bindings with matching seed videos
- `src/utils/profileAnalysis.ts` - `analyzeProfile()`, `computeAutoAssign()`

### Performance Optimizations

- **KeyGrid re-render optimization:** `usePlayingSoundIds()` hook extracts just the `Set<string>` of playing sound IDs with shallow comparison, preventing re-renders from position-only progress updates
- **SoundDetails targeted subscription:** Subscribes only to the specific track's playback entry instead of the full `playingTracks` map
- **Dual-canvas WaveformDisplay:** Static canvas for waveform + dynamic canvas for playback cursor, decoupling expensive rendering from progress ticks
- **Batched duration updates:** `computeProfileDurations()` returns all durations at once, applied in a single `set()` call instead of N individual state updates
- **Progress emission rate:** Audio engine emits `playback_progress` every 250ms (reduced from 100ms)
- **Waveform disk cache:** Persistent cache avoids recomputing waveforms across app restarts

## Known Limitations

### Numpad + Shift
When NumLock is ON and Shift is pressed with a numpad key, the OS sends the alternate key (ArrowLeft, End, etc.) instead of "Shift+Numpad4". This is standard keyboard hardware behavior, not a bug. **Workaround:** Go to Settings > Key Detection > Momentum Modifier and select "Ctrl" or "Alt" instead of "Shift".

### Discovery
- Max 15 seeds per generation (prevents timeouts)
- Some videos have no YouTube Mix (returns empty, non-blocking)
- Duration filter: 30-900 seconds (excludes very short clips or long streams)
- Momentum detection heuristic may be imprecise for some audio types (spoken word, etc.)

## Technical Limits

- **Max Tracks:** 20
- **Momentum Seeking:** Instant via symphonia byte-level seek (no pre-loading needed)
- **Cooldown Range:** 0ms - 5000ms
- **Crossfade Range:** 100ms - 2000ms
- **Supported Audio Formats:** MP3, WAV, OGG, FLAC, M4A/AAC, WebM
- **Audio Thread:** Dynamic timeout (200ms idle, 16ms when playing) to reduce CPU usage
- **Waveform Cache:** 50 entries max (LRU eviction), disk-persisted
- **Discovery:** Max 15 seeds, top 30 suggestions, 10 concurrent mix fetches

## Dependencies (Cargo.toml excerpt)

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rodio = "0.19"         # Audio playback
cpal = "0.15"          # Audio device enumeration
symphonia = { version = "0.5", features = ["mp3", "flac", "ogg", "wav", "pcm", "aac", "isomp4"] }  # Fast seeking + M4A
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }  # Timestamps (discovery cache)
dirs = "5"             # System directories (app data dir)
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }  # HTTP downloads (yt-dlp, ffmpeg)
rfd = "0.15"           # Native file dialogs
futures = "0.3"        # Async streams (buffer_unordered for discovery)
zip = { version = "2", default-features = false, features = ["deflate"] }  # ffmpeg ZIP extraction
tracing = "0.1"        # Structured logging
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }  # Log formatting
tracing-appender = "0.2"  # Daily rolling log files

# Platform-specific:
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-shell = "2"  # Shell open (folders, URLs)

[target."cfg(not(any(target_os = \"macos\", target_os = \"windows\")))".dependencies]
rdev = { git = "https://github.com/fufesou/rdev", branch = "master" }  # Global key detection (Linux)

[target."cfg(target_os = \"windows\")".dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_System_LibraryLoader",
    "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_Input",
    "Win32_Devices_HumanInterfaceDevice", "Win32_Graphics_Gdi"
] }  # Raw Input API

# macOS uses native CGEventTap via CoreGraphics FFI (no external crate)
```

### Frontend Dependencies (npm)

```json
"@tauri-apps/plugin-shell": "^2.0.0"  // Opening folders (logs, data)
```

## External Requirements

- **yt-dlp:** Auto-downloaded to `{app_data}/bin/yt-dlp.exe` on first YouTube download. No user installation needed. Also used for YouTube search, playlist fetching, and Mix discovery.
- **ffmpeg:** Auto-downloaded to `{app_data}/bin/ffmpeg.exe` on first YouTube download (required for M4A remuxing). No user installation needed.

## Reference

See `KeyToMusic_Technical_Specification.md` for comprehensive implementation details, data models, UI mockups, and pseudocode algorithms.
