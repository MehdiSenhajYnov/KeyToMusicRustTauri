# CLAUDE.md

## Project Overview

KeyToMusic is a Tauri 2.x desktop soundboard for manga reading. Global keyboard detection triggers sounds without interrupting reading. Features: multi-track audio with crossfading, YouTube downloads, momentum (start position), loop modes, waveform visualization, discovery system, and Manga Mood AI (local VLM detects page mood and auto-triggers tagged sounds).

**Platforms:** Windows 10/11, macOS 10.15+, Linux (Ubuntu, Fedora, Arch)
**Tech:** Tauri 2.x (Rust backend + React 18/TypeScript/Tailwind/Zustand frontend), rodio/cpal/symphonia (audio), yt-dlp (YouTube), llama.cpp/Qwen3-VL (mood AI)

## Project Structure

```
src/                              # React/TypeScript frontend
├── components/                   # UI (Layout, Tracks, Sounds, Keys, Discovery/, Errors/, common/)
│   ├── ConfirmDialog.tsx         # Custom confirm modal (macOS WKWebView compat)
│   ├── Sounds/SearchResultPreview.tsx  # Inline HTML5 audio player for YouTube search preview
│   ├── Settings/DislikedVideosPanel.tsx # Persistent dislike management in Settings
│   ├── common/SearchFilterBar.tsx  # Spotlight-style filter bar for KeyGrid (text + prefix chips)
│   ├── common/EmptyStateAction.tsx # Reusable onboarding CTA (icon + button + description)
│   ├── common/WarningTooltip.tsx # Hover tooltip for momentum modifier conflicts
│   └── common/KeyboardShortcutsModal.tsx # Help modal listing all shortcuts (dynamic config, platform-aware)
├── stores/                       # Zustand: profileStore, audioStore, settingsStore, discoveryStore,
│                                 #   historyStore, errorStore, exportStore, toastStore, confirmStore,
│                                 #   waveformStore, moodStore
├── hooks/                        # useAudioEvents, useKeyDetection, useDiscovery,
│                                 #   useDiscoveryPredownload, useUndoRedo, useTextInputFocus,
│                                 #   useTrackPosition, useWheelSlider, useMoodPlayback
├── types/index.ts                # All types (Sound, Profile, AppConfig, MoodCategory, KeyGridFilter, WaveformData, etc.)
└── utils/                        # tauriCommands, keyMapping, profileAnalysis, errorMessages,
                                  #   fileHelpers, soundHelpers, inputHelpers, moodHelpers
src-tauri/src/
├── main.rs                       # Entry point, logging, event forwarding
├── commands.rs                   # All Tauri commands
├── state.rs                      # AppState (config, audio, keys, caches)
├── types.rs                      # Shared Rust types
├── audio/                        # engine.rs, track.rs, crossfade.rs, symphonia_source.rs,
│                                 #   analysis.rs (waveform + cache), buffer.rs (duration)
├── keys/                         # detector.rs, mapping.rs, chord.rs,
│                                 #   macos_listener.rs, windows_listener.rs
├── discovery/                    # engine.rs, mix_fetcher.rs, cache.rs
├── mood/                         # llama_manager.rs (download llama-server + model),
│                                 #   inference.rs (LlamaServer lifecycle, image resize, analyze),
│                                 #   server.rs (axum HTTP API for external tools)
├── youtube/                      # downloader.rs, cache.rs, search.rs, yt_dlp_manager.rs, ffmpeg_manager.rs
├── import_export/                # .ktm file handling
└── storage/                      # Profile & config persistence
data/                             # Runtime: profiles/, cache/, discovery/, bin/, imported_sounds/, logs/, models/
resources/                        # Static: icons, error.mp3
```

## Agent Workflow

When exploring the codebase (searching for code, understanding how something works, gathering context for a task), **always launch multiple sub-agents in parallel** using the Task tool. Split the exploration by domain — for example, one agent for the Rust backend (`src-tauri/`), one for the React frontend (`src/`), and one for config/types/utils if relevant. This applies to any open-ended search, not just when the user asks explicitly. The goal is to maximize speed by parallelizing all independent searches.

## Development Commands

```bash
npm run tauri dev                                        # Dev mode (hot reload)
npm run tauri build                                      # Production build
cargo fmt --manifest-path src-tauri/Cargo.toml           # Format Rust
cargo clippy --manifest-path src-tauri/Cargo.toml        # Lint Rust
cargo test --manifest-path src-tauri/Cargo.toml          # Test Rust
```

## Core Architecture

### Audio System

- **Multi-Track:** Up to 20 tracks (OST, Ambiance, SFX). One sound per track; new sound triggers crossfade.
- **Volume:** `final = sound.volume × track.volume × master_volume`. Real-time sound volume via `set_sound_volume`.
- **Events:** mpsc channel in `main.rs` receives engine events → Tauri events. Engine emits `playback_progress` every 250ms.
- **Duration:** Symphonia header reading (`n_frames / sample_rate`), fallback to rodio. Batch via `preload_profile_sounds`.
- **Crossfade:** 500ms default, equal-power curve (cos/sin). Same-track only.
- **Momentum:** Start position in seconds. Triggered by Auto-Momentum mode or configurable modifier key (Shift/Ctrl/Alt).
- **Playback:** Custom `SymphoniaSource` (implements `rodio::Source`). Async decode thread with bounded channel (4 buffers ahead) — never blocks the audio callback. Source pre-created off the audio thread (`PlaySoundPrepared` command). Formats: MP3, M4A/AAC, OGG, FLAC, WAV. Instant seeking (O(1) CBR, O(log n) VBR). Requires `isomp4` symphonia feature for M4A.
- **Volume Ramping:** Gradual volume transitions (0.1 step per tick, ~160ms full sweep) to eliminate clicks/pops from instant volume changes.
- **Device:** User-selectable output device via `cpal`. Persisted as `audioDevice` in config. Seamless switching: captures track states, rebuilds OutputStream, resumes at captured positions (<50ms gap, no SoundEnded events). Polls for default device changes every 5s when `audioDevice = None`.
- **CPU Pool:** Shared Rayon thread pool (4 threads) in `AppState::cpu_pool` for all CPU-bound audio operations (waveform computation, duration reading). Limits concurrent CPU work via work-stealing queue instead of spawning unbounded threads. Batch operations (`preload_profile_sounds`, `get_waveforms_batch`) use chunked `par_iter` (chunks of 4) to avoid monopolizing the pool — yields between chunks so interactive requests (user waveforms, single duration) get serviced promptly.

### Waveform Analysis

- **Backend (`audio/analysis.rs`):** `WaveformData` = `Vec<f32>` RMS samples (0.0-1.0) + duration + suggested_momentum
  - `compute_waveform_sampled()` - ~40x faster, seeks to N positions. Falls back to full decode.
  - `detect_momentum_point()` - Multi-pass momentum detection: adaptive percentile thresholds (P25/P50/P75) → windowed gradient candidates → quality scoring (amplitude rise + sustained energy + position) → best candidate above MIN_QUALITY_SCORE.
- **Cache:** In-memory LRU (50 entries) + disk (`data/cache/waveforms.json`). Dirty-flag batched writes (flush every 5s). File-mtime validation on profile load only (not per-access). Atomic writes (tmp+remove+rename for Windows compat).
- **Frontend (`WaveformDisplay.tsx`):** Triple-canvas (static waveform + momentum markers + playback cursor). Draggable momentum marker. Suggested momentum indicator (cyan dashed line with label, pulse glow on new suggestion). Momentum drag only redraws markers layer.
- **Frontend (`MomentumSuggestionBadge.tsx`):** Reusable badge component for applying suggested momentum. Cyan pill with sparkle icon, value, chevron. Sizes: sm/md. Used in SoundDetails, AddSoundModal, DiscoveryPanel.
- **Frontend (`waveformStore.ts`):** LRU eviction (max 50 entries) with access order tracking. Bounded memory growth.

### Discovery System

```
Seeds (YouTube + local sounds) → YouTube Mix per seed → Cross-seed aggregation → Score by occurrence → Top 30 → Pre-download (audio + duration) → Lazy waveform on visible → One-click add
```

- **Local Sound Seeds:** Local sounds auto-resolve to YouTube video IDs via metadata tags (title/artist) or cleaned filename search. Resolved IDs cached in `Sound.resolved_video_id` (persisted with profile). Resolution uses query cascade: tags > title > filename. Max 5 concurrent yt-dlp searches. Cross-seed aggregation naturally filters bad matches.
- **Backend (`discovery/`):**
  - `engine.rs` - Extracts video IDs as seeds (max 15), fetches Mix concurrently (`buffer_unordered(10)`), aggregates, filters (30-900s, excludes existing), returns top 30. Streaming partial results. Cancelable via `AtomicBool`. Background mode (skips events, appends to cache). Also: `clean_filename_for_search()`, `resolve_local_seeds()`, `build_search_query()` for local sound resolution. `collect_discovery_video_ids()` protects cached suggestion audio from cleanup.
  - `mix_fetcher.rs` - `yt-dlp "{url}&list=RD{id}" --flat-playlist --dump-json`. 15s timeout. Best-effort.
  - `cache.rs` - Per-profile (`data/discovery/{profile_id}.json`). Stores seed_hash + dismissed_ids + cursor/revealed/visited state. Permanent dislikes stored in profile's `disliked_videos` field (persisted across sessions, managed via Settings).
- **Frontend:**
  - `discoveryStore.ts` - `EnrichedSuggestion` with predownload status, waveform, auto-assignment, preview state. Pagination (10 initial, +10 increment). Carousel navigation. Separate tracking for `downloadProgresses`, `poolPredownloads` (pre-downloaded but not yet visible), `refreshPredownloads` (mirrors visited+1/+2 for instant refresh). Visited locking prevents reordering items user has seen.
  - `useDiscoveryPredownload.ts` - Asymmetric window [current-2, current+3], max 3 concurrent. Waveforms are lazy: computed only for visible suggestions [current-1, current+1] after predownload completes, via `getWaveform()`. In-flight guard prevents duplicate requests. Refresh pre-download: pre-downloads first 2 items of unseen pool for instant UX.
  - `DiscoveryPanel.tsx` - Carousel, auto-triggers on profile load, streaming display, preview, dislike (permanent), one-click add. Preview volume slider (persisted to localStorage).
- **Smart Auto-Assignment (`profileAnalysis.ts`):**
  - `analyzeProfile()` - "single-sound" (avg ≤ 2/binding) vs "multi-sound" mode
  - Single-sound: next available key, least-used track. Multi-sound: cluster to bindings with matching seeds.

### Manga Mood AI

Detects manga page mood via a local VLM pipeline and auto-triggers sounds tagged with that mood. The canonical product flow is documented in [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md).

Current runtime path:

```
Reader page in browser
  → extension preloads/captures page images in base64
  → POST /api/chapter/page + POST /api/chapter/focus
  → POST /api/lookup for visible page
  → fallback POST /api/analyze-window if cache miss
  → mood cache + mood trigger + committed playback
```

- **Base moods:** 8 values — `epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery`
- **Intensity:** `1..3`
- **Mood Tagging:** Manual per-binding via dropdown in SoundDetails. `KeyBinding.mood: Option<BaseMood>` and `KeyBinding.moodIntensity: Option<MoodIntensity>`. Mood badge (colored pill) in KeyGrid. Filter with `m:` prefix in SearchFilterBar.
- **Backend (`mood/`):**
  - `llama_manager.rs` — Download + manage llama-server binary + `Qwen3-VL-4B-Thinking` model assets (`GGUF` + `mmproj`) in `data/bin/` and `data/models/`.
  - `inference.rs` — `LlamaServer` lifecycle, image preparation, runtime sizing/fallbacks, local helper analysis methods.
  - `winner.rs` — Shared production/benchmark winner core (window prompts, aggregation, repairs, hold logic).
  - `chapter_pipeline.rs` — Hot-zone chapter scheduler centered on the visible page rather than page `0`.
  - `server.rs` — Axum HTTP server. Active routes: `POST /api/analyze-window`, `POST /api/chapter/page`, `POST /api/chapter/focus`, `POST /api/live/cancel`, `POST /api/lookup`, `POST /api/trigger`, `GET /api/cache/status`, `GET /api/status`, `GET /api/moods`.
- **Frontend:**
  - `moodStore.ts` — Runtime/API status, install state, download progress, last detected and committed moods. Actions: checkInstallation, refreshServiceStatus, installServer, installModel, startServer, stopServer.
  - `useMoodPlayback.ts` — Uses `mood_detected` for UI state and `mood_committed` for actual playback. Finds all bindings with matching mood tag/intensity. Triggers all via `Promise.allSettled` (multi-track). Toast notification.
  - `MoodAiSection` in SettingsModal — Toggle, install status (green/red dots), download progress bar, start/stop server, API port config.
  - Sidebar `MoodIndicator` — Shows last detected mood as colored pill.
- **AppConfig:** `moodAiEnabled: bool`, `moodApiPort: u16` (default 8765)
- **AppState:** `llama_server`, `mood_api_server`, `mood_cache`, `mood_director`

### Key Detection

Platform-specific global capture (foreground AND background):
- **Windows:** Raw Input API (`windows_listener.rs`) - Hidden `HWND_MESSAGE` window + `RIDEV_INPUTSINK`. Needed because `rdev`/`WH_KEYBOARD_LL` don't work with Tauri/WebView2 focus.
- **macOS:** CGEventTap (`macos_listener.rs`) - HID-level tap in CFRunLoop. Needed because rdev crashes on macOS 13+ (TSMGetInputSourceProperty thread issue).
- **Linux:** `rdev` crate

**Architecture:**
- `KeyDetectorConfig` (enabled, cooldown, shortcuts) consolidated in single `RwLock` for minimal lock contention (1 read lock per keypress instead of 5 separate Mutex locks)
- Timer thread uses `Condvar` (0 CPU idle) instead of 5ms polling
- Pre-computed static lookup tables for key code mapping (zero allocation in hot path)

**Behavior:**
- 200ms global cooldown (configurable 0-5000ms)
- Global shortcuts (Stop All, Auto-Momentum, Key Detection toggle) checked before `enabled` guard in `detector.rs`
- Auto-disable on text input focus (`useTextInputFocus` → `setKeyDetection(false)`). Non-text inputs (range, checkbox) excluded.
- **Textual key filtering (Windows):** When `enabled_flag == false`, filters letters/digits/space/numpad to allow normal typing. Uses `is_textual_key()` helper.
- AZERTY support: `charToKeyCode(e.key) || e.code` pattern, dynamic `layoutMap` via `recordKeyLayout()`
- Sticky modifier fix: `pressedKeysRef` cleared on window `blur`

### Multi-Key Chords

Trie-based combo detection (like fighting game inputs):
- Trigger immediately at leaf node (no extensions). Timer (30-50ms) only when extensions exist.
- Format: modifiers first (Ctrl > Shift > Alt), then base keys sorted alphabetically. `"KeyZ+KeyA"` → `"KeyA+KeyZ"`
- Config: `chordWindowMs` (20-100ms, default 30). Files: `keys/chord.rs`, `keys/detector.rs`, `utils/keyMapping.ts`

### Momentum Modifier

Configurable key (Shift/Ctrl/Alt/None) that triggers momentum playback. Exact binding match takes priority over momentum.
- Conflict detection: warns when shortcuts conflict with momentum modifier + bound keys (toast warnings + persistent warning icons in Settings and KeyGrid).
- Solves Numpad+Shift hardware limitation.

### Sound Assignment & Loop Modes

Binding uniqueness = `(keyCode, trackId)`. One key can have multiple bindings on different tracks — pressing a key triggers all its bindings simultaneously (`Promise.allSettled` for concurrent playback). Each binding has its own sound IDs list, loop mode, currentIndex, and mood tag.
- Key reassignment: move all bindings to new key (merges by track) or move individual sound.
- Loop modes: `off` (random, stop), `single` (loop same), `random` (avoid repeat, auto-next), `sequential` (cycle, auto-next)
- **Mood tagging:** Each binding can be tagged with an 8-value base mood plus optional minimum intensity. Tagged bindings auto-trigger when Mood AI detects a compatible committed mood. Mood changes are undoable.
- KeyGrid groups bindings by keyCode (one cell per key, shows track count indicator when >1 track, colored mood badge pill).
- SoundDetails renders each binding separately with per-binding track selector, loop mode, mood dropdown, and delete.

### YouTube Integration

- **Downloads:** Concurrent, each with `download_id`. M4A format, stored as `{video_id}.m4a` in cache.
- **Search:** `yt-dlp ytsearch{N}:{query}` via `search_youtube` command. Inline preview via `get_youtube_stream_url` — extracts direct audio stream URL (`--dump-json -f bestaudio`), played in HTML5 `<audio>` element (`SearchResultPreview.tsx`). One active preview at a time.
- **Playlists:** `fetch_playlist(url)` for imports and discovery Mix fetching.
- **yt-dlp/ffmpeg:** Auto-downloaded to `data/bin/` on first use. ffmpeg needed for DASH→M4A remux.
- **Cache:** Canonical URL lookup (`watch?v={id}`). Secondary index `video_id_index: HashMap<String, String>` for O(1) video_id lookups. Cleanup scans all profiles, removes unreferenced entries. Deferred to 5s after startup.
- **Retry:** 3 attempts, 2s delay for transient errors. Immediate fail for permanent errors. Auto-update: If download fails with HTTP 403/429 or extraction errors, attempts yt-dlp update (once per request).

### Profiles & Config

- Profiles: `data/profiles/{uuid}.json` with sounds, tracks, bindings, disliked_videos. All sounds stopped on switch.
- Config (`data/config.json`): masterVolume, autoMomentum, keyDetectionEnabled, shortcuts, crossfadeDuration (500ms), keyCooldown (200ms), currentProfileId, audioDevice, chordWindowMs (30ms), momentumModifier, playlistImportEnabled, moodAiEnabled, moodApiPort (8765)
- Atomic writes (`.tmp` → remove dest → rename, for Windows compat). Config: debounced saves via `AtomicBool` dirty flag, flushed every 2s by background thread. Profile list: partial JSON parsing (`serde_json::Value`) for O(1) field extraction instead of full deserialization.

### Import/Export

- **Format:** `.ktm` = ZIP with `profile.json`, `metadata.json`, `sounds/` folder, `waveforms.json` (optional).
- **Export safety:** Temp file → rename. Tracking file (`export_in_progress.txt`) for crash recovery. `AtomicBool` cancellation. Window close interception.
- **Import:** New UUID, copies audio to `data/imported_sounds/{new_id}/`.
- **Legacy import:** Old Unity KeyToMusic format. Maps Windows VK codes → web KeyCode strings.
- **Tauri 2 permissions:** `capabilities/default.json` needs `core:window:allow-destroy` and `core:window:allow-close`.

### Error Handling

- **Sound not found:** `play_error_sound()` (error.mp3 at 50% volume) → `sound_not_found` event → `errorStore.missingQueue` → `FileNotFoundModal` (Locate/Re-download/Remove/Skip/Skip All).
- **Profile verification:** On load, checks all paths, queues missing files.
- **Toast notifications:** Non-blocking errors via `useToastStore`. `errorMessages.ts` maps raw errors → user-friendly.
- **Custom confirm dialog:** `useConfirmStore.getState().confirm(msg)` returns `Promise<boolean>`. 30s auto-timeout resolves to `false`. Required because browser `confirm()` fails on macOS WKWebView.

### Undo/Redo

- Ctrl+Z / Ctrl+Y (Cmd+Shift+Z). `historyStore.ts` with past/future stacks (max 50).
- Undoable: sound/binding/track add/delete/modify. Non-undoable: profile CRUD, YouTube downloads, durations.
- Toast feedback. History cleared on profile switch. Hook: `useUndoRedo.ts` in `App.tsx`.

## App State

```rust
pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub audio_engine: Arc<OnceLock<AudioEngineHandle>>,  // Deferred init (std::sync::OnceLock)
    pub key_detector: KeyDetector,
    pub youtube_cache: Arc<Mutex<YouTubeCache>>,    // Lazy-loaded on first access
    pub waveform_cache: Arc<Mutex<WaveformCache>>,  // Lazy-loaded on first access
    pub discovery_cancel: Arc<AtomicBool>,
    pub cpu_pool: Arc<rayon::ThreadPool>,
    pub profile_load_gen: Arc<AtomicU64>,
    pub config_dirty: Arc<AtomicBool>,
    pub llama_server: Arc<tokio::sync::Mutex<Option<LlamaServer>>>,  // Mood AI inference server
    pub mood_api_server: Arc<Mutex<Option<JoinHandle<()>>>>,          // HTTP API for external tools
    pub mood_cache: Arc<Mutex<MoodCache>>,                            // Per-chapter in-memory mood cache
    pub mood_director: Arc<Mutex<MoodDirector>>,                      // Playback smoothing / commit logic
}
```

### Startup Optimization

- **Window:** Starts hidden (`visible: false`), shown after React render via double `requestAnimationFrame` + `getCurrentWindow().show()`.
- **Skeleton:** CSS-only skeleton in `index.html` (no JS), replaced when React hydrates `#root`. Fade-in transition (0.25s).
- **Audio engine:** Deferred via `Arc<OnceLock<AudioEngineHandle>>` (std library). Init runs in `tokio::spawn` after window creation. Commands use `state.get_audio_engine()?` (error if not ready) or graceful `if let Ok(engine)` for volume sync.
- **Caches:** Both `YouTubeCache` and `WaveformCache` use lazy loading (`ensure_loaded()` on first access). Saves ~40-150ms at startup.
- **Unified IPC:** Single `get_initial_state` command replaces 3 sequential calls (config + profiles + current profile).
- **Parallelization:** `load_config()` and `cleanup_interrupted_export()` run in parallel via `std::thread::scope`.
- **Code splitting:** `SettingsModal`, `FileNotFoundModal`, `AddSoundModal`, `DiscoveryPanel`, `KeyboardShortcutsModal` lazy-loaded with `React.lazy` + `Suspense`.

## Tauri Commands (`commands.rs`)

**Startup:** `get_initial_state`
**Config:** `get_config`, `update_config`, `set_profile_bindings`
**Profiles:** `list_profiles`, `create_profile`, `load_profile`, `save_profile`, `delete_profile`, `duplicate_profile`
**Audio:** `play_sound(track_id, sound_id, file_path, start_position, sound_volume)`, `stop_sound`, `stop_all_sounds`, `set_master_volume`, `set_track_volume`, `set_sound_volume`, `get_audio_duration`, `preload_profile_sounds`
**Devices:** `list_audio_devices`, `set_audio_device`
**Keys:** `set_key_detection`, `set_stop_all_shortcut`, `set_key_cooldown`
**Waveform:** `get_waveform(path, num_points)`, `get_waveforms_batch(entries)`
**YouTube:** `add_sound_from_youtube(url, download_id)`, `search_youtube`, `fetch_playlist`, `get_youtube_stream_url(video_id)`, `check_yt_dlp_installed`, `install_yt_dlp`, `check_ffmpeg_installed`, `install_ffmpeg`
**Discovery:** `start_discovery(profile_id, exclude_ids, background)`, `get_discovery_suggestions`, `save_discovery_cursor`, `update_discovery_pool`, `dismiss_discovery`, `dislike_discovery`, `undislike_discovery`, `list_disliked_videos`, `cancel_discovery`, `predownload_suggestion`
**Import/Export:** `export_profile`, `import_profile`, `pick_save_location`, `cleanup_export_temp`, `cancel_export`, `pick_ktm_file`, `pick_legacy_file`, `import_legacy_save`
**Mood AI:** `check_llama_server_installed`, `install_llama_server`, `check_mood_model_installed`, `install_mood_model`, `start_mood_server`, `stop_mood_server`, `get_mood_server_status`, `get_mood_service_status`, `analyze_mood(image_path)`
**Utility:** `verify_profile_sounds`, `pick_audio_file`, `pick_audio_files`, `get_logs_folder`, `get_data_folder`, `open_folder`

## Backend → Frontend Events

| Event | Payload |
|-------|---------|
| `sound_started` / `sound_ended` | `{ trackId, soundId }` |
| `playback_progress` | `{ trackId, position }` (every 250ms) |
| `key_pressed` | `{ keyCode, withShift }` |
| `stop_all_triggered` / `toggle_key_detection` / `toggle_auto_momentum` | `{}` |
| `youtube_download_progress` | `{ downloadId, status, progress }` |
| `sound_not_found` | `{ soundId, path, trackId }` |
| `audio_error` | `{ message }` |
| `export_progress` | `{ current, total, fileName }` |
| `discovery_started` / `discovery_complete` / `discovery_error` | `{}` / `{ count }` / `{ message }` |
| `discovery_resolving` | `{ count }` (local sounds being resolved) |
| `discovery_progress` | `{ current, total, seedName }` |
| `discovery_partial` | `Vec<DiscoverySuggestion>` |
| `waveform_progress` | `{ path, points, duration, sampleRate }` (streaming waveform, every 5 iterations) |
| `mood_model_download_progress` | `{ downloaded, total }` (bytes) |
| `mood_server_status` | `{ status: "starting" \| "running" \| "stopped" \| "error" }` |
| `mood_detected` | `{ mood, source }` (source: "api" or "local") |
| `mood_committed` | `{ mood, source, previous_mood?, dwell_count? }` |

## Technical Notes

- **Thread safety:** Audio engine in separate thread. Use Tokio channels or `Arc<Mutex<>>`. CPU-bound work uses shared `Arc<rayon::ThreadPool>` (4 threads, `Send + Sync` natively, no Mutex needed). Each `SymphoniaSource` spawns its own decode thread (bounded channel, stopped on Drop).
- **Logging:** `tracing` + `tracing-appender`. Daily rolling logs in `data/logs/`. `RUST_LOG` env var (default: info).
- **Data paths:** Windows `AppData/Roaming/KeyToMusic/`, macOS `Library/Application Support/KeyToMusic/`, Linux `.local/share/keytomusic/`
- **External tools:** yt-dlp and ffmpeg auto-downloaded to `data/bin/`. llama-server auto-downloaded from llama.cpp GitHub releases. `Qwen3-VL-4B-Thinking` model assets auto-downloaded to `data/models/`. No user install needed.

## Technical Limits

- Max 20 tracks. Audio formats: MP3, WAV, OGG, FLAC, M4A/AAC, WebM.
- Cooldown: 0-5000ms. Crossfade: 100-2000ms. Waveform cache: 50 entries (LRU).
- Discovery: max 15 seeds, top 30 suggestions, 10 concurrent mix fetches.
- Audio thread: dynamic timeout (200ms idle, 16ms playing).
- Mood AI: 8 base moods + intensity `1..3`. Visible-window fallback uses local context, chapter prefetch targets `X-10 .. X+20` and loads up to `X-14 .. X+24`. Runtime model assets are ~3.3GB plus mmproj overhead depending on package.

## Known Limitations

- **Numpad+Shift:** OS sends alternate key (ArrowLeft, End) instead of "Shift+Numpad4". Workaround: use Ctrl/Alt as momentum modifier.
- **Discovery:** Some videos have no Mix. Momentum detection imprecise for spoken word.
- **Mood AI:** Requires a capable GPU for good UX. The visible-page fallback is much slower than the old 2B experiments, so the extension relies on chapter prefetch and cache warming. Mood tagging remains manual. Model assets are several GB.

## UI Notes

- Dark theme, indigo/violet accents. Min window: 800x600.
- Layout: Header (logo, master volume, settings) + Sidebar (profiles, controls, now playing, discovery) + Main (tracks, keys, sound details)
- AddSoundModal: file picker or drag & drop, per-file momentum editors, key cycling for bulk assignment, inline YouTube search preview (HTML5 audio streaming)
- Resizable SoundDetails panel (min 120px, default 256px)
- KeyGrid filter bar: Spotlight-style search with inline prefix filters (`t:`, `l:`, `s:`, `m:`), chips, counter. `Ctrl+F` focuses, `Escape` clears. Non-matching bindings grayed out (opacity-30). Resets on profile switch.
- Sidebar: MoodIndicator shows last detected mood as colored pill (visible when moodAiEnabled).
- Settings: "Manga Mood AI" section between Audio and Data — toggle, install status (green/red dots for llama-server + model), download progress bar, start/stop server button, API port config.
