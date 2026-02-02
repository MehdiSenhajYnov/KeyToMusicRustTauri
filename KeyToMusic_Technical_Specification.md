# KeyToMusic - Document Technique Complet

## Table des Matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Stack Technique](#2-stack-technique)
3. [Architecture Globale](#3-architecture-globale)
4. [Modèle de Données](#4-modèle-de-données)
5. [Moteur Audio](#5-moteur-audio)
6. [Système de Détection des Touches](#6-système-de-détection-des-touches)
7. [Gestion des Pistes](#7-gestion-des-pistes)
8. [Téléchargement YouTube](#8-téléchargement-youtube)
9. [Interface Utilisateur](#9-interface-utilisateur)
10. [Sauvegarde et Persistance](#10-sauvegarde-et-persistance)
11. [Import/Export](#11-importexport)
12. [Gestion des Erreurs](#12-gestion-des-erreurs)
13. [Instructions de Développement](#13-instructions-de-développement)
14. [Fonctionnalités Phase 8](#14-fonctionnalités-phase-8)
15. [Smart Discovery System (Phase 8+)](#15-smart-discovery-system-phase-8)

---

## 1. Vue d'ensemble

### 1.1 Description du Projet

**KeyToMusic** est une application de type soundboard conçue pour accompagner la lecture de mangas avec des musiques/OST adaptées à l'ambiance des planches. L'application détecte les touches du clavier en arrière-plan et déclenche des sons assignés, permettant une expérience immersive sans interrompre la lecture.

### 1.2 Fonctionnalités Principales

- Détection globale des touches clavier (fonctionne en arrière-plan)
- Assignation de sons à des touches avec support multi-sons par touche
- Système de pistes multiples pour superposer différents types de sons (OST, ambiance, SFX)
- Crossfade fluide entre les sons d'une même piste
- Mode Momentum pour démarrer les sons à une position spécifique
- Modes de boucle variés (Off, Random, Single, Sequential)
- Téléchargement de sons depuis YouTube avec système de cache
- Multi-profils/playlists sauvegardables
- Import/Export de configurations

### 1.3 Plateformes Cibles

- Windows 10/11
- macOS 10.15+
- Linux (distributions majeures : Ubuntu, Fedora, Arch)

---

## 2. Stack Technique

### 2.1 Technologies Principales

| Composant | Technologie | Justification |
|-----------|-------------|---------------|
| Framework Desktop | **Tauri 2.x** | Performances natives, taille réduite, accès système via Rust |
| Frontend | **React 18+ avec TypeScript** | UI moderne, écosystème riche, typage fort |
| Backend Audio | **Rust (rodio/cpal + symphonia)** | Lecture audio performante, seeking instantané |
| Détection Touches | **Rust (Raw Input/CGEventTap/rdev)** | Windows Raw Input API, macOS CGEventTap, Linux rdev |
| Téléchargement YT | **yt-dlp** (binaire externe) | Fiable, maintenu activement |
| Styling | **Tailwind CSS** | Styling rapide, thème sombre facile |
| State Management | **Zustand** | Simple, performant, TypeScript natif |

### 2.2 Dépendances Rust (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rodio = "0.19"          # Lecture audio
cpal = "0.15"           # Énumération des périphériques audio
symphonia = { version = "0.5", features = ["mp3", "flac", "ogg", "wav", "pcm", "aac", "isomp4"] }  # Seeking rapide + M4A
tokio = { version = "1", features = ["full"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }  # Timestamps (discovery cache)
dirs = "5"              # Répertoires système (app data dir)
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }  # HTTP (yt-dlp/ffmpeg downloads)
rfd = "0.15"            # Dialogues natifs (file picker)
futures = "0.3"         # Async streams (buffer_unordered pour discovery)
zip = { version = "2", default-features = false, features = ["deflate"] }  # Extraction ZIP ffmpeg
tracing = "0.1"         # Structured logging
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }  # Log formatting
tracing-appender = "0.2"  # Daily rolling log files

# Dépendances conditionnelles par plateforme:
[target."cfg(not(any(target_os = \"android\", target_os = \"ios\")))".dependencies]
tauri-plugin-shell = "2"  # Shell open (folders, URLs)

[target."cfg(not(any(target_os = \"macos\", target_os = \"windows\")))".dependencies]
rdev = { git = "https://github.com/fufesou/rdev", branch = "master" }  # Détection globale des touches (Linux)

[target."cfg(target_os = \"windows\")".dependencies]
windows = { version = "0.58", features = [
    "Win32_Foundation", "Win32_UI_WindowsAndMessaging", "Win32_System_LibraryLoader",
    "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_Input",
    "Win32_Devices_HumanInterfaceDevice", "Win32_Graphics_Gdi"
] }  # Raw Input API

# Note: macOS utilise CoreGraphics CGEventTap directement via FFI (voir section 6.7)
```

### 2.3 Dépendances npm (package.json)

```json
{
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "zustand": "^4.5.0",
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "tailwindcss": "^3.4.0",
    "vite": "^5.0.0",
    "@tauri-apps/cli": "^2.0.0"
  }
}
```

---

## 3. Architecture Globale

### 3.1 Structure des Dossiers

```
keytomusic/
├── src/                          # Code source React/TypeScript
│   ├── components/               # Composants React
│   │   ├── Tracks/
│   │   │   └── TrackView.tsx           # Vue des pistes (volume, rename, track management)
│   │   ├── Sounds/
│   │   │   ├── SoundDetails.tsx        # Détails et édition d'un son (momentum, volume, preview)
│   │   │   ├── MultiKeyDetails.tsx     # Sélection multi-touches (detail panel)
│   │   │   └── AddSoundModal.tsx       # Modal ajout sons (local + YouTube + playlists)
│   │   ├── Keys/
│   │   │   ├── KeyGrid.tsx             # Grille visuelle des touches assignées
│   │   │   └── KeyCaptureSlot.tsx      # Capture de touche (binding + modifier support)
│   │   ├── Controls/
│   │   │   ├── MasterStopButton.tsx    # Bouton Master Stop
│   │   │   ├── GlobalToggles.tsx       # Toggles (key detection, auto-momentum)
│   │   │   └── NowPlaying.tsx          # Tracks en cours de lecture (seekable)
│   │   ├── Profiles/
│   │   │   └── ProfileSelector.tsx     # Sélecteur/gestion de profils (create, rename, duplicate, delete)
│   │   ├── Settings/
│   │   │   └── SettingsModal.tsx       # Modal settings (audio, keys, shortcuts, import/export)
│   │   ├── Layout/
│   │   │   ├── Header.tsx              # Barre supérieure (logo, master volume, settings)
│   │   │   ├── MainContent.tsx         # Zone principale (tracks, key grid, sound details)
│   │   │   └── Sidebar.tsx             # Barre latérale (profils, controls, now playing)
│   │   ├── Toast/
│   │   │   └── ToastContainer.tsx      # Notifications toast non-bloquantes
│   │   ├── Export/
│   │   │   └── ExportProgress.tsx      # Progress bar flottante d'export
│   │   ├── Errors/
│   │   │   └── FileNotFoundModal.tsx   # Modal fichier manquant (locate/re-download/remove)
│   │   ├── Discovery/
│   │   │   └── DiscoveryPanel.tsx      # Carousel de suggestions YouTube Mix
│   │   ├── common/
│   │   │   ├── WaveformDisplay.tsx     # Canvas waveform RMS (dual-canvas: static + cursor)
│   │   │   └── WarningTooltip.tsx      # Icône warning avec tooltip
│   │   └── ConfirmDialog.tsx           # Modal de confirmation custom (remplace confirm())
│   ├── stores/                   # State management Zustand
│   │   ├── profileStore.ts
│   │   ├── settingsStore.ts
│   │   ├── audioStore.ts         # Playing tracks state (usePlayingSoundIds hook)
│   │   ├── historyStore.ts       # Undo/Redo stacks
│   │   ├── discoveryStore.ts     # Discovery state & actions
│   │   ├── errorStore.ts
│   │   ├── exportStore.ts
│   │   ├── toastStore.ts
│   │   ├── confirmStore.ts
│   │   └── waveformStore.ts     # Client-side waveform cache (Map-based)
│   ├── hooks/                    # Custom React hooks
│   │   ├── useKeyDetection.ts
│   │   ├── useAudioEvents.ts
│   │   ├── useTextInputFocus.ts          # Auto-disable key detection on text inputs
│   │   ├── useUndoRedo.ts
│   │   ├── useDiscovery.ts               # Discovery carousel/playback
│   │   ├── useDiscoveryPredownload.ts    # Background pre-download pipeline
│   │   └── useTrackPosition.ts           # Efficient position polling via RAF
│   ├── types/                    # TypeScript types
│   │   └── index.ts
│   ├── utils/                    # Utilitaires
│   │   ├── fileHelpers.ts
│   │   ├── keyMapping.ts
│   │   ├── tauriCommands.ts      # All invoke() wrappers
│   │   ├── soundHelpers.ts
│   │   ├── inputHelpers.ts
│   │   ├── errorMessages.ts
│   │   └── profileAnalysis.ts    # Smart auto-assignment profile analysis
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css
├── src-tauri/                    # Code source Rust/Tauri
│   ├── src/
│   │   ├── main.rs               # Point d'entrée Tauri, event forwarding, logging
│   │   ├── commands.rs           # Commandes Tauri exposées au frontend
│   │   ├── state.rs              # AppState (audio engine, key detector, waveform cache, cpu_pool, profile_load_gen)
│   │   ├── types.rs              # Toutes les structures de données sérialisables
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs         # Moteur audio principal
│   │   │   ├── track.rs          # Gestion des pistes
│   │   │   ├── crossfade.rs      # Logique de crossfade
│   │   │   ├── symphonia_source.rs # Source custom avec seeking rapide
│   │   │   ├── buffer.rs         # Métadonnées audio (durées)
│   │   │   └── analysis.rs       # Waveform RMS + auto-momentum detection
│   │   ├── keys/
│   │   │   ├── mod.rs
│   │   │   ├── detector.rs       # Détection globale des touches + chords
│   │   │   ├── mapping.rs        # Mapping touches -> actions, KeyEvent types
│   │   │   ├── chord.rs          # Multi-key chord detection (Trie-based)
│   │   │   ├── macos_listener.rs # macOS CGEventTap implementation
│   │   │   └── windows_listener.rs # Windows Raw Input API implementation
│   │   ├── youtube/
│   │   │   ├── mod.rs
│   │   │   ├── downloader.rs     # Téléchargement via yt-dlp (retry, canonical URLs)
│   │   │   ├── cache.rs          # Système de cache YouTube (file-to-profile mapping, cleanup)
│   │   │   ├── search.rs         # YouTube search + playlist fetch via yt-dlp
│   │   │   ├── yt_dlp_manager.rs # Auto-download/update yt-dlp binary
│   │   │   └── ffmpeg_manager.rs # Auto-download ffmpeg for M4A remux
│   │   ├── discovery/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs         # YouTube Mix discovery (streaming, cross-seed aggregation)
│   │   │   ├── mix_fetcher.rs    # Fetch YouTube Mix playlists via yt-dlp
│   │   │   └── cache.rs          # Per-profile discovery cache with seed hash
│   │   ├── storage/
│   │   │   ├── mod.rs
│   │   │   ├── profile.rs        # Gestion des profils
│   │   │   └── config.rs         # Configuration globale
│   │   └── import_export/        # .ktm file handling
│   │       ├── mod.rs
│   │       ├── export.rs         # Export profil → .ktm (ZIP), cancellation, tracking
│   │       └── import.rs         # Import .ktm → profil (new UUIDs, file extraction)
│   ├── Cargo.toml
│   └── tauri.conf.json
├── data/                         # Données utilisateur (créé au runtime)
│   ├── profiles/                 # Fichiers de profil JSON
│   ├── cache/
│   │   ├── cache_index.json      # Index du cache YouTube
│   │   ├── waveforms.json        # Disk-persisted waveform cache (LRU, mtime-validated)
│   │   └── *.m4a                 # Fichiers audio cachés (video ID as filename)
│   ├── bin/
│   │   ├── yt-dlp.exe            # Auto-downloaded yt-dlp binary
│   │   └── ffmpeg.exe            # Auto-downloaded ffmpeg binary
│   ├── imported_sounds/          # Sons importés depuis .ktm
│   ├── logs/                     # Daily rolling log files
│   └── config.json               # Configuration globale
└── resources/                    # Ressources statiques
    ├── sounds/
    │   └── error.mp3             # Son d'erreur système
    └── icons/
```

### 3.2 Communication Frontend ↔ Backend

```
┌─────────────────────────────────────────────────────────────┐
│                      FRONTEND (React)                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Stores    │  │  Components │  │   Event Listeners   │  │
│  │  (Zustand)  │  │   (React)   │  │  (Tauri Events)     │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                    Tauri invoke()                            │
└──────────────────────────┼───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────┼───────────────────────────────────┐
│                   BACKEND (Rust/Tauri)                       │
│                          │                                   │
│  ┌───────────────────────┼───────────────────────────────┐  │
│  │                 Commands Handler                       │  │
│  │   - play_sound()    - stop_sound()    - set_volume()  │  │
│  │   - add_sound()     - remove_sound()  - download_yt() │  │
│  │   - save_profile()  - load_profile()  - export()      │  │
│  └───────────────────────┬───────────────────────────────┘  │
│                          │                                   │
│         ┌────────────────┼────────────────┐                 │
│         ▼                ▼                ▼                 │
│  ┌────────────┐  ┌────────────┐  ┌────────────────┐        │
│  │   Audio    │  │    Keys    │  │    Storage     │        │
│  │   Engine   │  │  Detector  │  │    Manager     │        │
│  └────────────┘  └────────────┘  └────────────────┘        │
│                                                              │
│                    Tauri emit() (events)                     │
│                                                              │
│  Audio Event Thread (mpsc channel, blocking recv):             │
│    receives AudioEngine events → emits Tauri events           │
│    (sound_started, sound_ended, playback_progress)            │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. Modèle de Données

### 4.1 Types TypeScript (Frontend)

```typescript
// types/index.ts

// Identifiants uniques
type SoundId = string;      // UUID v4
type TrackId = string;      // UUID v4
type ProfileId = string;    // UUID v4
type KeyCode = string;      // Ex: "KeyA", "Digit1", "F5", "ShiftLeft"

// Source d'un son
type SoundSource = 
  | { type: "local"; path: string }
  | { type: "youtube"; url: string; cachedPath: string };

// Mode de boucle
type LoopMode = "off" | "random" | "single" | "sequential";

// Configuration d'un son individuel
interface Sound {
  id: SoundId;
  name: string;
  source: SoundSource;
  momentum: number;           // Position de départ en secondes (décimales autorisées)
  volume: number;             // 0.0 à 1.0 (volume individuel du son)
  duration: number;           // Durée totale en secondes (calculée au chargement)
}

// Configuration d'une touche
interface KeyBinding {
  keyCode: KeyCode;
  trackId: TrackId;
  soundIds: SoundId[];        // Liste des sons assignés à cette touche
  loopMode: LoopMode;
  currentIndex: number;       // Index actuel pour le mode sequential/random
  name?: string;              // Nom personnalisé (défaut: nom du premier son)
}

// Configuration d'une piste
interface Track {
  id: TrackId;
  name: string;
  volume: number;             // 0.0 à 1.0 (volume de la piste)
  currentlyPlaying: SoundId | null;
  playbackPosition: number;   // Position actuelle de lecture en secondes
  isPlaying: boolean;
}

// Profil utilisateur (une "playlist" / configuration complète)
interface Profile {
  id: ProfileId;
  name: string;
  createdAt: string;          // ISO 8601
  updatedAt: string;          // ISO 8601
  sounds: Sound[];
  tracks: Track[];
  keyBindings: KeyBinding[];
}

// Modificateur momentum configurable
type MomentumModifier = "Shift" | "Ctrl" | "Alt" | "None";

// Configuration globale de l'application
interface AppConfig {
  masterVolume: number;           // 0.0 à 1.0
  autoMomentum: boolean;          // Si true, tous les sons démarrent au momentum
  keyDetectionEnabled: boolean;   // Si true, les touches sont détectées
  masterStopShortcut: KeyCode[];  // Combinaison de touches (ex: ["ControlLeft", "ShiftLeft", "KeyS"])
  autoMomentumShortcut: KeyCode[];  // Shortcut pour toggle auto-momentum
  keyDetectionShortcut: KeyCode[];  // Shortcut pour toggle key detection (fonctionne même si désactivé)
  crossfadeDuration: number;      // Durée du crossfade en millisecondes (défaut: 500)
  keyCooldown: number;            // Cooldown global entre pressions en millisecondes (défaut: 200)
  currentProfileId: ProfileId | null;
  audioDevice: string | null;     // null = follow system default, string = force specific device
  chordWindowMs: number;          // Fenêtre de détection multi-key chords (défaut: 30ms)
  momentumModifier: MomentumModifier; // Quel modifier déclenche le momentum (défaut: "Shift")
  playlistImportEnabled: boolean; // Préférence persistée: checkbox "Download entire playlist"
}

// État "Now Playing" pour l'affichage
interface NowPlayingState {
  trackId: TrackId;
  trackName: string;
  soundName: string;
  currentTime: number;
  duration: number;
  isPlaying: boolean;
}

// Résultat de recherche YouTube
interface YoutubeSearchResult {
  videoId: string;
  title: string;
  duration: number;       // Durée en secondes
  channel: string;
  thumbnailUrl: string;
  url: string;
  alreadyDownloaded: boolean;  // Déjà dans le cache YouTube
}

// Playlist YouTube
interface YoutubePlaylist {
  title: string;
  entries: YoutubeSearchResult[];
  totalCount: number;
}

// Données de waveform RMS
interface WaveformData {
  points: number[];           // 0.0-1.0 normalized RMS values
  duration: number;           // total seconds
  sampleRate: number;
  suggestedMomentum: number | null;  // Auto-detected pre-peak point (seconds)
}

// Suggestion Discovery (définie dans discoveryStore.ts)
interface DiscoverySuggestion {
  videoId: string;
  title: string;
  channel: string;
  duration: number;         // Durée en secondes
  url: string;
  occurrenceCount: number;  // Nombre de mixes où cette suggestion apparaît
  sourceSeedNames: string[];  // Noms des sons sources
  sourceSeedIds: string[];    // IDs des sons sources
}

// Résultat d'un pré-téléchargement Discovery
interface PredownloadResult {
  videoId: string;
  cachedPath: string;
  title: string;
  duration: number;
  waveform: WaveformData;
}

// Événements émis par le backend vers le frontend
type BackendEvent =
  | { type: "sound_started"; trackId: TrackId; soundId: SoundId }
  | { type: "sound_ended"; trackId: TrackId; soundId: SoundId }
  | { type: "playback_progress"; trackId: TrackId; position: number }
  | { type: "key_pressed"; keyCode: KeyCode; withShift: boolean }
  | { type: "master_stop_triggered" }
  | { type: "toggle_key_detection" }
  | { type: "toggle_auto_momentum" }
  | { type: "youtube_download_progress"; downloadId: string; status: string; progress: number | null }
  | { type: "sound_not_found"; soundId: SoundId; path: string; trackId: TrackId }
  | { type: "audio_error"; message: string }
  | { type: "export_progress"; current: number; total: number; fileName: string }
  | { type: "discovery_started" }
  | { type: "discovery_progress"; current: number; total: number; seedName: string }
  | { type: "discovery_partial"; suggestions: DiscoverySuggestion[] }
  | { type: "discovery_complete"; count: number }
  | { type: "discovery_error"; message: string };
```

### 4.2 Structures Rust (Backend)

```rust
// src-tauri/src/types.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type SoundId = String;
pub type TrackId = String;
pub type ProfileId = String;
pub type KeyCode = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SoundSource {
    #[serde(rename = "local")]
    Local { path: String },
    #[serde(rename = "youtube")]
    YouTube { url: String, cached_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    Off,
    Random,
    Single,
    Sequential,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sound {
    pub id: SoundId,
    pub name: String,
    pub source: SoundSource,
    pub momentum: f64,
    pub volume: f32,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyBinding {
    pub key_code: KeyCode,
    pub track_id: TrackId,
    pub sound_ids: Vec<SoundId>,
    pub loop_mode: LoopMode,
    pub current_index: usize,
    #[serde(default)]
    pub name: Option<String>,  // Nom personnalisé pour le groupe de sons
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub volume: f32,
    pub currently_playing: Option<SoundId>,
    pub playback_position: f64,
    pub is_playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub sounds: Vec<Sound>,
    pub tracks: Vec<Track>,
    pub key_bindings: Vec<KeyBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MomentumModifier { Shift, Ctrl, Alt, None }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub master_volume: f32,
    pub auto_momentum: bool,
    pub key_detection_enabled: bool,
    pub master_stop_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub auto_momentum_shortcut: Vec<KeyCode>,
    #[serde(default)]
    pub key_detection_shortcut: Vec<KeyCode>,
    pub crossfade_duration: u32,
    pub key_cooldown: u32,
    pub current_profile_id: Option<ProfileId>,
    #[serde(default)]
    pub audio_device: Option<String>,
    #[serde(default = "default_chord_window")]
    pub chord_window_ms: u32,                    // Multi-key chord detection window (default: 30ms)
    #[serde(default)]
    pub momentum_modifier: MomentumModifier,     // Which modifier triggers momentum (default: Shift)
    #[serde(default)]
    pub playlist_import_enabled: bool,           // Persisted playlist checkbox preference
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            master_volume: 0.8,
            auto_momentum: false,
            key_detection_enabled: true,
            master_stop_shortcut: vec!["ControlLeft".into(), "ShiftLeft".into(), "KeyS".into()],
            auto_momentum_shortcut: vec![],
            key_detection_shortcut: vec![],
            crossfade_duration: 500,
            key_cooldown: 200,
            current_profile_id: None,
            audio_device: None,
            chord_window_ms: 30,
            momentum_modifier: MomentumModifier::Shift,
            playlist_import_enabled: false,
        }
    }
}

// Types additionnels pour waveform et discovery

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub points: Vec<f32>,
    pub duration: f64,
    pub sample_rate: u32,
    pub suggested_momentum: Option<f64>,  // Auto-detected pre-peak point
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubeSearchResult {
    pub video_id: String,
    pub title: String,
    pub duration: f64,
    pub channel: String,
    pub thumbnail_url: String,
    pub url: String,
    pub already_downloaded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YoutubePlaylist {
    pub title: String,
    pub entries: Vec<YoutubeSearchResult>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverySuggestion {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub url: String,
    pub occurrence_count: usize,
    pub source_seed_names: Vec<String>,
    #[serde(default)]
    pub source_seed_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PredownloadResult {
    pub video_id: String,
    pub cached_path: String,
    pub title: String,
    pub duration: f64,
    pub waveform: WaveformData,
}

// Discovery seed (backend-only, dans discovery/engine.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedInfo {
    pub video_id: String,
    pub sound_name: String,
}

// Cache des résultats discovery (backend-only, dans discovery/cache.rs)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryCacheData {
    pub profile_id: String,
    pub seed_hash: String,           // Hash des seed IDs pour détecter les changements
    pub generated_at: String,
    pub suggestions: Vec<DiscoverySuggestion>,
    #[serde(default)]
    pub dismissed_ids: Vec<String>,  // Suggestions rejetées par l'utilisateur
}
```

### 4.3 Format de Sauvegarde JSON

#### 4.3.1 Configuration Globale (`data/config.json`)

```json
{
  "masterVolume": 0.8,
  "autoMomentum": false,
  "keyDetectionEnabled": true,
  "masterStopShortcut": ["ControlLeft", "ShiftLeft", "KeyS"],
  "autoMomentumShortcut": [],
  "keyDetectionShortcut": [],
  "crossfadeDuration": 500,
  "keyCooldown": 200,
  "currentProfileId": "550e8400-e29b-41d4-a716-446655440000",
  "audioDevice": null,
  "chordWindowMs": 30,
  "momentumModifier": "Shift",
  "playlistImportEnabled": false
}
```

#### 4.3.2 Profil (`data/profiles/{profile_id}.json`)

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Lecture Shonen",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-20T14:45:00Z",
  "sounds": [
    {
      "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
      "name": "Epic Battle OST",
      "source": {
        "type": "local",
        "path": "/Users/mehdi/Music/OST/epic_battle.mp3"
      },
      "momentum": 45.5,
      "volume": 1.0,
      "duration": 180.0
    },
    {
      "id": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
      "name": "Rain Ambiance",
      "source": {
        "type": "youtube",
        "url": "https://www.youtube.com/watch?v=XXXXXXXXXXX",
        "cachedPath": "C:\\Users\\mehdi\\AppData\\Roaming\\KeyToMusic\\cache\\XXXXXXXXXXX.m4a"
      },
      "momentum": 0,
      "volume": 0.6,
      "duration": 3600.0
    }
  ],
  "tracks": [
    {
      "id": "track-001",
      "name": "OST Principale",
      "volume": 0.9,
      "currentlyPlaying": null,
      "playbackPosition": 0,
      "isPlaying": false
    },
    {
      "id": "track-002",
      "name": "Ambiance",
      "volume": 0.5,
      "currentlyPlaying": null,
      "playbackPosition": 0,
      "isPlaying": false
    }
  ],
  "keyBindings": [
    {
      "keyCode": "KeyA",
      "trackId": "track-001",
      "soundIds": ["a1b2c3d4-e5f6-7890-abcd-ef1234567890"],
      "loopMode": "single",
      "currentIndex": 0,
      "name": "Battle OST"
    },
    {
      "keyCode": "KeyR",
      "trackId": "track-002",
      "soundIds": ["b2c3d4e5-f6a7-8901-bcde-f12345678901"],
      "loopMode": "off",
      "currentIndex": 0
    }
  ]
}
```

#### 4.3.3 Cache YouTube (`data/cache/cache_index.json`)

```json
{
  "entries": [
    {
      "url": "https://www.youtube.com/watch?v=XXXXXXXXXXX",
      "cachedPath": "C:\\Users\\mehdi\\AppData\\Roaming\\KeyToMusic\\cache\\XXXXXXXXXXX.m4a",
      "title": "Rain Ambiance 10 Hours",
      "downloadedAt": "2024-01-15T10:30:00Z",
      "fileSize": 52428800
    }
  ]
}
```

Note: Cache URLs are canonical (`https://www.youtube.com/watch?v={id}`) regardless of the original URL format. Filenames use the video ID directly (`{id}.m4a`). Unused entries are cleaned up automatically by scanning profiles.

---

## 5. Moteur Audio

### 5.1 Architecture du Moteur

```
┌─────────────────────────────────────────────────────────────────┐
│                        AudioEngine                               │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                     MasterMixer                             │ │
│  │  ┌─────────────────────────────────────────────────────┐   │ │
│  │  │ master_volume: f32                                   │   │ │
│  │  └─────────────────────────────────────────────────────┘   │ │
│  │                           │                                 │ │
│  │     ┌─────────────────────┼─────────────────────┐          │ │
│  │     ▼                     ▼                     ▼          │ │
│  │ ┌─────────┐         ┌─────────┐          ┌─────────┐       │ │
│  │ │ Track 1 │         │ Track 2 │          │ Track N │       │ │
│  │ │ (OST)   │         │(Ambiance)│         │  (SFX)  │       │ │
│  │ │ vol:0.9 │         │ vol:0.5 │          │ vol:1.0 │       │ │
│  │ └────┬────┘         └────┬────┘          └────┬────┘       │ │
│  │      │                   │                    │             │ │
│  │      ▼                   ▼                    ▼             │ │
│  │ ┌─────────┐         ┌─────────┐          ┌─────────┐       │ │
│  │ │Crossfade│         │Crossfade│          │Crossfade│       │ │
│  │ │ Handler │         │ Handler │          │ Handler │       │ │
│  │ └────┬────┘         └────┬────┘          └────┬────┘       │ │
│  │      │                   │                    │             │ │
│  │      ▼                   ▼                    ▼             │ │
│  │ ┌─────────┐         ┌─────────┐          ┌─────────┐       │ │
│  │ │ Sound A │         │ Sound C │          │ Sound E │       │ │
│  │ │(playing)│         │(playing)│          │ (idle)  │       │ │
│  │ │ Sound B │         │         │          │         │       │ │
│  │ │(fading) │         │         │          │         │       │ │
│  │ └─────────┘         └─────────┘          └─────────┘       │ │
│  └────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Logique de Lecture

#### 5.2.1 Volume Final d'un Son

```
volume_final = sound.volume × track.volume × master_volume
```

#### 5.2.2 Déclenchement d'un Son

```
QUAND touche_pressée(key_code):
    
    SI cooldown_actif:
        RETOURNER (ignorer la pression)
    
    ACTIVER cooldown_global (200ms par défaut)
    
    SI key_code == master_stop_shortcut:
        STOPPER tous les sons de toutes les pistes
        RETOURNER
    
    key_binding = TROUVER binding pour key_code
    SI key_binding EST NULL:
        RETOURNER
    
    track = TROUVER track par key_binding.track_id
    sounds = TROUVER sons par key_binding.sound_ids
    
    SI sounds EST VIDE:
        RETOURNER
    
    // Sélection du son selon le mode loop
    sound = SÉLECTIONNER_SON(key_binding, sounds)
    
    // Déterminer la position de départ
    SI auto_momentum OU shift_est_pressé:
        start_position = sound.momentum
    SINON:
        start_position = 0
    
    // Lancer le crossfade si un son joue déjà sur cette piste
    SI track.currently_playing EST NOT NULL:
        DÉMARRER_CROSSFADE(track, sound, start_position)
    SINON:
        JOUER_SON(track, sound, start_position)
```

#### 5.2.3 Sélection du Son (selon Loop Mode)

```
FONCTION SÉLECTIONNER_SON(key_binding, sounds):
    
    SELON key_binding.loop_mode:

        CAS "off":
            // Sélection aléatoire (évite de répéter le même)
            SI sounds.length == 1:
                RETOURNER sounds[0]
            candidates = sounds SAUF sounds[key_binding.current_index]
            selected = RANDOM(candidates)
            key_binding.current_index = INDEX_DE(selected)
            RETOURNER selected

        CAS "random":
            SI sounds.length == 1:
                RETOURNER sounds[0]
            // Éviter de rejouer le même son
            candidates = sounds SAUF sounds[key_binding.current_index]
            selected = RANDOM(candidates)
            key_binding.current_index = INDEX_DE(selected)
            RETOURNER selected
        
        CAS "single":
            RETOURNER sounds[key_binding.current_index]
        
        CAS "sequential":
            sound = sounds[key_binding.current_index]
            key_binding.current_index = (key_binding.current_index + 1) % sounds.length
            RETOURNER sound
```

#### 5.2.4 Gestion de Fin de Son

```
QUAND son_terminé(track, sound):
    
    key_binding = TROUVER binding pour track ET sound
    
    SELON key_binding.loop_mode:

        CAS "off":
            // Son terminé, pas de relance
            track.currently_playing = NULL
            track.is_playing = FALSE
        
        CAS "random":
            next_sound = SÉLECTIONNER_SON(key_binding, sounds)
            start_position = SI auto_momentum ALORS next_sound.momentum SINON 0
            JOUER_SON(track, next_sound, start_position)
        
        CAS "single":
            start_position = SI auto_momentum ALORS sound.momentum SINON sound.momentum
            // Note: En loop single avec momentum, on repart TOUJOURS du momentum
            JOUER_SON(track, sound, sound.momentum SI momentum_actif SINON sound.momentum)
            // Correction: Toujours repartir du momentum si le son a été lancé avec momentum
        
        CAS "sequential":
            next_sound = SÉLECTIONNER_SON(key_binding, sounds)
            start_position = SI auto_momentum ALORS next_sound.momentum SINON 0
            JOUER_SON(track, next_sound, start_position)
```

**Note importante pour Loop Single + Momentum**: Si un son est lancé avec Shift (ou AutoMomentum activé), quand il arrive à la fin et boucle, il doit repartir du momentum, pas de 0:00.

### 5.3 Logique de Crossfade

Le crossfade permet une transition fluide entre deux sons sur la même piste.

#### 5.3.1 Courbe de Crossfade

```
Durée totale: 500ms (configurable)

Temps      Son A (sortant)    Son B (entrant)
──────────────────────────────────────────────
  0ms          100%                0%
175ms           30%                0%
325ms            0%               30%
500ms            0%              100%

// Zone de chevauchement: 175ms à 325ms (150ms de superposition)
```

#### 5.3.2 Implémentation

```rust
struct CrossfadeState {
    outgoing_sound: Option<SoundId>,
    incoming_sound: SoundId,
    start_time: Instant,
    duration: Duration,
}

impl CrossfadeState {
    fn get_volumes(&self, elapsed: Duration) -> (f32, f32) {
        let progress = (elapsed.as_millis() as f32) / (self.duration.as_millis() as f32);
        let progress = progress.clamp(0.0, 1.0);
        
        // Courbe avec zone de silence au milieu
        let outgoing_vol = if progress < 0.35 {
            1.0 - (progress / 0.35) * 0.7  // 100% -> 30%
        } else if progress < 0.65 {
            0.3 - ((progress - 0.35) / 0.3) * 0.3  // 30% -> 0%
        } else {
            0.0
        };
        
        let incoming_vol = if progress < 0.35 {
            0.0
        } else if progress < 0.65 {
            ((progress - 0.35) / 0.3) * 0.3  // 0% -> 30%
        } else {
            0.3 + ((progress - 0.65) / 0.35) * 0.7  // 30% -> 100%
        };
        
        (outgoing_vol, incoming_vol)
    }
}
```

### 5.4 Seeking et Streaming via Symphonia

Pour les fichiers longs (2-3 heures), le seeking doit être instantané. L'approche initiale de pré-charger des buffers en RAM a été remplacée par un seeking byte-level via symphonia.

#### 5.4.1 Problème avec rodio skip_duration

rodio's `skip_duration` décode TOUS les samples depuis le début jusqu'à la position cible — c'est O(n) pour les formats compressés comme MP3. Pour un fichier de 3 heures avec un momentum à 2h, cela signifie décoder 2 heures de données avant de pouvoir jouer.

#### 5.4.2 Solution: SymphoniaSource

La solution utilise symphonia directement pour un seeking byte-level:
- **CBR (Constant Bit Rate)**: Seek instantané O(1) — calcul direct de l'offset byte
- **VBR avec header Xing/VBRI**: Seek quasi-instantané via la table de seek dans le header
- **VBR sans header**: Seek par bisection O(log n) — toujours rapide

```rust
/// Custom rodio::Source using symphonia for fast byte-level seeking.
pub struct SymphoniaSource {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    sample_buf: Vec<f32>,
    sample_pos: usize,
    finished: bool,
}

impl SymphoniaSource {
    /// Open a file and seek to the given position in seconds.
    pub fn new(file_path: &str, seek_to_secs: f64) -> Result<Self, String> {
        // 1. Open file and probe format
        // 2. Create decoder from codec params
        // 3. Seek to position via reader.seek(SeekMode::Coarse, SeekTo::Time {...})
        // 4. Reset decoder state after seek
        // 5. Pre-fill first buffer with decode_next_packet()
    }
}

impl Iterator for SymphoniaSource { type Item = f32; ... }
impl Source for SymphoniaSource { ... }
```

#### 5.4.3 Stratégie de Lecture dans AudioTrack

```rust
// Dans track.rs play():
// Always use SymphoniaSource for consistent format support (mp3, m4a, ogg, flac, wav)
let source = SymphoniaSource::new(file_path, start_position_secs)?;
new_sink.append(source);
```

Note: SymphoniaSource is used for ALL playback (not just momentum seeking) to ensure consistent format support, especially M4A from YouTube downloads which requires the `isomp4` symphonia feature.

#### 5.4.4 BufferManager (metadata only)

Le BufferManager est conservé uniquement pour le calcul des durées audio. La durée est lue via les headers symphonia (`n_frames / sample_rate`), ce qui est instantané sans décodage:

```rust
impl BufferManager {
    pub fn get_audio_duration(path: &str) -> Result<f64, String> {
        // 1. Probe format with symphonia
        // 2. Read n_frames from track params
        // 3. Return n_frames / sample_rate
        // 4. Fallback to rodio sample-counting if headers lack frame count
    }
}
```

Les durées sont calculées en batch au chargement du profil via `preload_profile_sounds` (2 threads parallèles).

#### 5.4.5 Real-time Sound Volume (SetSoundVolume)

Le volume d'un son peut être modifié en temps réel pendant la lecture via la commande `set_sound_volume`:

```rust
// AudioCommand variant:
SetSoundVolume { track_id: TrackId, sound_id: SoundId, volume: f32 }

// Updates the stored sound_volumes map and recalculates sink volume:
// final_volume = sound_volume × master_volume
// Only updates if no crossfade is active on the track
```

### 5.5 Gestion des Périphériques Audio

#### 5.5.1 Sélection du Périphérique

L'utilisateur peut choisir un périphérique de sortie audio spécifique via les Settings, ou suivre le périphérique par défaut du système (None). La liste des périphériques est obtenue via `cpal::default_host().output_devices()`.

```rust
// Commandes Tauri:
fn list_audio_devices() -> Vec<String>;
fn set_audio_device(device: Option<String>) -> Result<(), String>;
```

Le périphérique sélectionné est persisté dans `config.json` sous le champ `audioDevice`.

#### 5.5.2 Polling du Périphérique par Défaut

Quand `audioDevice = None` (suivre le système), le thread audio vérifie toutes les 3 secondes si le périphérique par défaut a changé via `cpal::default_host().default_output_device()`.

#### 5.5.3 Seamless Device Switching (Reprise Transparente)

Lors d'un changement de périphérique (Settings ou changement système), les sons en cours reprennent automatiquement sur le nouveau périphérique :

```rust
struct TrackResumeInfo {
    track_id: TrackId,
    sound_id: SoundId,
    file_path: String,
    position: f64,          // start_position + elapsed time
    sound_volume: f32,
    track_volume: f32,
}
```

**Algorithme:**
1. Capturer `TrackResumeInfo` pour chaque piste en lecture (via `track.get_position()` et `track.file_path`)
2. Stopper et supprimer toutes les pistes (elles référencent l'ancien `OutputStreamHandle`)
3. Créer un nouveau `OutputStream` sur le nouveau périphérique
4. Pour chaque entrée de reprise: créer un nouveau `AudioTrack`, appeler `play()` à la position capturée
5. Aucun event `SoundEnded` n'est émis — le frontend ne voit pas d'interruption

**Résultat:** Gap <50ms, les sons continuent à la même position sur le nouveau périphérique.

---

## 6. Système de Détection des Touches

### 6.1 Architecture

Le système capture les événements clavier au niveau système, permettant la détection même quand l'application est en arrière-plan.

**Implémentation par plateforme:**
- **Windows:** Utilise Raw Input API (`windows_listener.rs`) — `rdev` et `SetWindowsHookEx` ne reçoivent pas les événements quand la fenêtre Tauri/WebView2 est focalisée. Raw Input avec `RIDEV_INPUTSINK` contourne cette limitation.
- **Linux:** Utilise `rdev` pour la capture globale
- **macOS:** Utilise une implémentation custom CGEventTap (`macos_listener.rs`) — sur macOS 13+, rdev crash car il appelle `TSMGetInputSourceProperty` depuis un thread background.

```rust
// src-tauri/src/keys/detector.rs

use rdev::{listen, Event, EventType, Key};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct KeyDetector {
    enabled: Arc<Mutex<bool>>,
    cooldown_ms: Arc<Mutex<u32>>,
    pressed_keys: Arc<Mutex<HashSet<String>>>,
    master_stop_shortcut: Arc<Mutex<Vec<String>>>,
    key_detection_shortcut: Arc<Mutex<Vec<String>>>,
    auto_momentum_shortcut: Arc<Mutex<Vec<String>>>,
}

impl KeyDetector {
    pub fn new(cooldown_ms: u32) -> Self {
        Self {
            enabled: Arc::new(Mutex::new(true)),
            last_key_time: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(10))),
            cooldown: Duration::from_millis(cooldown_ms as u64),
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
            master_stop_shortcut: Arc::new(Mutex::new(vec![])),
        }
    }
    
    pub fn start<F>(&self, callback: F)
    where
        F: Fn(KeyEvent) + Send + Sync + 'static
    {
        let enabled = self.enabled.clone();
        let pressed_keys = self.pressed_keys.clone();
        let master_stop_shortcut = self.master_stop_shortcut.clone();
        let key_detection_shortcut = self.key_detection_shortcut.clone();
        let auto_momentum_shortcut = self.auto_momentum_shortcut.clone();

        std::thread::spawn(move || {
            let callback = Arc::new(callback);
            listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        let code = key_to_code(key);
                        let mut pressed = pressed_keys.lock().unwrap();

                        // Éviter les répétitions (touche maintenue)
                        if pressed.contains(&code) { return; }
                        pressed.insert(code.clone());

                        // Global shortcuts: work even when key detection is disabled
                        // Key detection shortcut
                        let kd_keys = key_detection_shortcut.lock().unwrap();
                        if !kd_keys.is_empty() && is_shortcut_pressed(&pressed, &kd_keys) {
                            drop(pressed); drop(kd_keys);
                            callback(KeyEvent::ToggleKeyDetection);
                            return;
                        }
                        drop(kd_keys);

                        // Master stop shortcut
                        let stop_keys = master_stop_shortcut.lock().unwrap();
                        if !stop_keys.is_empty() && is_shortcut_pressed(&pressed, &stop_keys) {
                            drop(pressed); drop(stop_keys);
                            callback(KeyEvent::MasterStop);
                            return;
                        }
                        drop(stop_keys);

                        // Auto momentum shortcut
                        let am_keys = auto_momentum_shortcut.lock().unwrap();
                        if !am_keys.is_empty() && is_shortcut_pressed(&pressed, &am_keys) {
                            drop(pressed); drop(am_keys);
                            callback(KeyEvent::ToggleAutoMomentum);
                            return;
                        }
                        drop(am_keys);

                        // If detection is disabled, don't trigger sound key presses
                        if !*enabled.lock().unwrap() { return; }

                        if is_modifier(&key) { return; }

                        let with_shift = pressed.contains("ShiftLeft")
                            || pressed.contains("ShiftRight");
                        drop(pressed);

                        callback(KeyEvent::KeyPressed { key_code: code, with_shift });
                    }
                    EventType::KeyRelease(key) => {
                        let code = key_to_code(key);
                        pressed_keys.lock().unwrap().remove(&code);
                    }
                    _ => {}
                }
            }).expect("Failed to listen to keyboard events");
        });
    }
}

pub enum KeyEvent {
    KeyPressed { key_code: String, with_shift: bool },
    MasterStop,
    ToggleKeyDetection,
    ToggleAutoMomentum,
}

fn key_to_code(key: Key) -> String {
    match key {
        Key::KeyA => "KeyA".to_string(),
        Key::KeyB => "KeyB".to_string(),
        // ... autres lettres
        Key::Num0 => "Digit0".to_string(),
        Key::Num1 => "Digit1".to_string(),
        // ... autres chiffres
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        // ... autres touches F
        Key::UpArrow => "ArrowUp".to_string(),
        Key::DownArrow => "ArrowDown".to_string(),
        Key::LeftArrow => "ArrowLeft".to_string(),
        Key::RightArrow => "ArrowRight".to_string(),
        Key::Space => "Space".to_string(),
        Key::Return => "Enter".to_string(),
        Key::Escape => "Escape".to_string(),
        // ... etc
        _ => format!("{:?}", key),
    }
}
```

### 6.2 Touches Supportées

| Catégorie | Touches | KeyCode |
|-----------|---------|---------|
| Lettres | A-Z | `KeyA` à `KeyZ` |
| Chiffres | 0-9 | `Digit0` à `Digit9` |
| Pavé numérique | 0-9 | `Numpad0` à `Numpad9` |
| Fonction | F1-F12 | `F1` à `F12` |
| Flèches | ↑ ↓ ← → | `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight` |
| Spéciales | Espace, Entrée, Tab, Échap | `Space`, `Enter`, `Tab`, `Escape` |
| Modificateurs | Shift, Ctrl, Alt | `ShiftLeft/Right`, `ControlLeft/Right`, `AltLeft/Right` |
| Ponctuation | ; , . / etc. | `Semicolon`, `Comma`, `Period`, `Slash`, etc. |

### 6.3 Comportement du Cooldown

- **Cooldown global** : 200ms par défaut (configurable, plage 0-5000ms)
- **Portée** : S'applique à TOUTES les touches (pas par touche individuelle)
- **But** : Éviter les déclenchements accidentels par appui prolongé ou spam

```
Timeline:
─────────────────────────────────────────────────────────────
0ms      100ms    200ms     300ms     400ms
│        │        │         │         │
▼        ▼        ▼         ▼         ▼
[A]      [A]      │         [B]       [A]
 ✓     (ignoré)  (cooldown   ✓         ✓
                   fin)
```

### 6.4 Shortcuts en Foreground

Tous les raccourcis globaux (Master Stop, Auto-Momentum toggle, Key Detection toggle) fonctionnent aussi quand l'application est au premier plan via un handler clavier navigateur:

```typescript
// Track pressed keys in a Set (uses character-based codes for layout support)
const pressedKeysRef = useRef<Set<string>>(new Set());

// On keydown: resolve layout-aware code, add to set
const resolvedCode = charToKeyCode(e.key) || e.code;
pressedKeysRef.current.add(resolvedCode);
recordKeyLayout(resolvedCode, e.key);

// Check shortcuts (all work regardless of keyDetectionEnabled)
if (config.keyDetectionShortcut.every(k => pressedKeysRef.current.has(k))) {
    toggleKeyDetection();
}
if (config.masterStopShortcut.every(k => pressedKeysRef.current.has(k))) {
    commands.stopAllSounds();
}
if (config.autoMomentumShortcut.every(k => pressedKeysRef.current.has(k))) {
    toggleAutoMomentum();
}

// On keyup: remove from set
// On window blur: clear entire set (prevents sticky modifiers after Alt+Tab)
```

### 6.5 Support Clavier AZERTY et Layouts Non-QWERTY

Le système utilise des codes basés sur les caractères plutôt que sur les positions physiques pour assurer la compatibilité avec tous les layouts:

- **`charToKeyCode(e.key)`**: Convertit le caractère produit en un code standard (ex: 'a' → 'KeyA')
- **`recordKeyLayout(code, key)`**: Enregistre dynamiquement le mapping physique → caractère
- **`keyCodeToDisplay(code)`**: Affiche le caractère correct selon le layout détecté
- Les shortcuts sont capturés et comparés en utilisant les codes basés sur les caractères

### 6.6 Désactivation Automatique

La détection des touches doit être temporairement désactivée quand l'utilisateur interagit avec un champ de texte dans l'application.

```typescript
// Frontend: Gérer le focus des champs de texte
useEffect(() => {
    const handleFocusIn = (e: FocusEvent) => {
        if (e.target instanceof HTMLInputElement || 
            e.target instanceof HTMLTextAreaElement) {
            invoke('set_key_detection', { enabled: false });
        }
    };
    
    const handleFocusOut = (e: FocusEvent) => {
        if (e.target instanceof HTMLInputElement || 
            e.target instanceof HTMLTextAreaElement) {
            invoke('set_key_detection', { enabled: true });
        }
    };
    
    document.addEventListener('focusin', handleFocusIn);
    document.addEventListener('focusout', handleFocusOut);
    
    return () => {
        document.removeEventListener('focusin', handleFocusIn);
        document.removeEventListener('focusout', handleFocusOut);
    };
}, []);
```

### 6.7 Implémentation macOS (CGEventTap)

Sur macOS, le système utilise une implémentation custom via CoreGraphics CGEventTap au lieu de rdev.

**Fichier:** `src-tauri/src/keys/macos_listener.rs`

**Raison:** rdev appelle `TSMGetInputSourceProperty` depuis un thread background, ce qui cause un crash (SIGTRAP) sur macOS 13+ car Apple enforce que cette API doit être appelée depuis la main dispatch queue.

**Architecture:**
```rust
// Points d'entrée FFI CoreGraphics/CoreFoundation
extern "C" {
    fn CGEventTapCreate(...) -> *mut c_void;
    fn CFRunLoopAddSource(...);
    fn CFRunLoopRun();
    // etc.
}

pub enum MacKeyEvent {
    Press(String),   // key code string
    Release(String),
}

/// Bloque le thread courant et écoute les événements clavier
pub fn listen_macos<F>(callback: F) -> Result<(), String>
where
    F: Fn(MacKeyEvent) + 'static
{
    // 1. Créer un CGEventTap au niveau HID
    // 2. Attacher au CFRunLoop
    // 3. Bloquer avec CFRunLoopRun()
}
```

**Mapping des Hardware Keycodes:**
Les keycodes macOS (0x00-0x7E) sont mappés vers des strings Web KeyboardEvent.code:
- 0x00 → "KeyA", 0x01 → "KeyS", ...
- 0x12 → "Digit1", 0x13 → "Digit2", ...
- 0x7A → "F1", 0x78 → "F2", ...
- Modificateurs via flags: Shift, Control, Option, Command

**Touches supportées:**
- 26 lettres (A-Z)
- 10 chiffres (0-9)
- F1-F12
- Flèches, navigation (Home, End, PageUp, PageDown, Insert, Delete)
- Pavé numérique complet
- Modificateurs (Shift, Control, Alt/Option, Meta/Command, CapsLock)
- Touches spéciales (Enter, Tab, Space, Backspace, Escape)
- Ponctuation et symboles

**Intégration dans detector.rs:**
```rust
#[cfg(target_os = "macos")]
{
    use crate::keys::macos_listener::{listen_macos, MacKeyEvent};
    listen_macos(move |event| {
        let (code, is_press) = match event {
            MacKeyEvent::Press(c) => (c, true),
            MacKeyEvent::Release(c) => (c, false),
        };
        handle_key_event(code, is_press);
    }).ok();
}

#[cfg(not(target_os = "macos"))]
{
    rdev::listen(move |event| { ... }).ok();
}
```

---

## 7. Gestion des Pistes

### 7.1 Concept

Les pistes permettent de superposer plusieurs types de sons :

- **Piste OST** : Musiques de fond
- **Piste Ambiance** : Sons d'ambiance (pluie, vent, foule)
- **Piste SFX** : Effets sonores ponctuels

Chaque piste est indépendante : lancer un son sur une piste n'affecte pas les autres.

### 7.2 Règles

1. **Une touche = une piste** : Quand un son est assigné à une touche, il est automatiquement lié à la piste de cette touche
2. **Création de piste** : Si la touche n'a pas encore de piste assignée, l'utilisateur choisit ou crée une piste
3. **Réutilisation** : Si la touche a déjà une piste, les nouveaux sons sont ajoutés à cette même piste
4. **Limite** : Maximum 20 pistes (pour éviter les problèmes de performance)
5. **Volume** : Chaque piste a son propre volume (0-100%)

### 7.3 Crossfade Intra-Piste

Le crossfade ne se produit qu'entre sons de la **même piste**.

```
Exemple:
- Piste "OST" : Son A joue
- Piste "Ambiance" : Son C joue
- L'utilisateur appuie sur une touche liée à la Piste "OST" avec Son B

Résultat:
- Son A fait un fade-out
- Son B fait un fade-in
- Son C continue sans interruption (piste différente)
```

---

## 8. Téléchargement YouTube

### 8.1 Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                    YouTube Downloader                           │
│                                                                  │
│  1. URL reçue, extraire video ID                               │
│     │                                                           │
│     ▼                                                           │
│  2. Vérifier le cache (canonical URL) ──┐                      │
│     │                                    │                      │
│     │ (pas en cache)                     │ (en cache)           │
│     ▼                                    ▼                      │
│  3. Auto-installer yt-dlp          Retourner le chemin          │
│     si non présent                  du fichier caché             │
│     │                                                           │
│     ▼                                                           │
│  4. Auto-installer ffmpeg                                       │
│     si non présent (remux M4A)                                  │
│     │                                                           │
│     ▼                                                           │
│  5. Appeler yt-dlp (retry up to 3x)                            │
│     │   yt-dlp -f bestaudio[ext=m4a]                           │
│     │   --write-info-json --ffmpeg-location                    │
│     │   --output "cache/%(id)s.%(ext)s"                        │
│     ▼                                                           │
│  6. ffmpeg remux: [FixupM4a] (auto via yt-dlp)                 │
│     │                                                           │
│     ▼                                                           │
│  7. Lire le titre depuis {id}.info.json                        │
│     │                                                           │
│     ▼                                                           │
│  8. Mettre à jour cache_index.json                             │
│     │                                                           │
│     ▼                                                           │
│  9. Retourner le chemin + métadonnées                          │
└────────────────────────────────────────────────────────────────┘
```

### 8.1.1 Téléchargements Concurrents

Le système supporte les téléchargements simultanés. Chaque appel à `add_sound_from_youtube` reçoit un `download_id` unique généré par le frontend. Les events de progression incluent ce `download_id` pour permettre au frontend de distinguer les téléchargements :

```rust
#[tauri::command]
pub async fn add_sound_from_youtube(url: String, download_id: String) -> Result<Sound, String>;

// Progress event payload:
{ "downloadId": "dl_1706000000_0", "status": "Downloading...", "progress": 45.0 }
```

Le frontend maintient une `Map<downloadId, { url, status, progress }>` pour afficher les barres de progression individuelles. L'input URL reste disponible pendant les téléchargements, permettant d'en lancer plusieurs à la suite.

### 8.2 Système de Cache

Le cache évite de re-télécharger un son déjà présent.

#### 8.2.1 Logique de Cache

```rust
pub struct YouTubeCache {
    index_path: PathBuf,
    cache_dir: PathBuf,
    entries: HashMap<String, CacheEntry>,  // canonical URL -> Entry
}

#[derive(Serialize, Deserialize)]
pub struct CacheEntry {
    url: String,
    cached_path: String,
    title: String,
    downloaded_at: String,
    file_size: u64,
}

impl YouTubeCache {
    /// Check cache before downloading (uses canonical URL)
    pub fn get(&self, url: &str) -> Option<&CacheEntry> {
        // Returns entry only if file still exists on disk
    }

    /// Add entry after successful download
    pub fn add_entry(&mut self, url, cached_path, title, file_size) -> CacheEntry { ... }

    /// Remove entries whose files are missing from disk
    pub fn verify_integrity(&mut self) { ... }

    /// Remove cache entries not referenced by any saved profile
    pub fn cleanup_unused(&mut self) {
        let used_paths = collect_used_cached_paths();  // scan all profiles
        // Delete files + entries where cached_path is NOT in used_paths
    }
}

/// Scan all profile JSONs → collect every cachedPath from YouTube sound sources
fn collect_used_cached_paths() -> HashSet<String> { ... }
```

#### 8.2.2 Cleanup Automatique

Le nettoyage des fichiers cache inutilisés se fait par **scan des profils** (pas de tracking `usedBy` en temps réel) :

1. Parcourir tous les fichiers `profiles/*.json`
2. Collecter tous les `source.cachedPath` des sons YouTube
3. Comparer avec les entrées du cache index
4. Supprimer les fichiers et entrées non-référencés

**Moments de cleanup :**
- Au **démarrage** de l'app (après `verify_integrity`)
- Après **`save_profile`** (un son YouTube a pu être retiré)
- Après **`delete_profile`** (un profil entier supprimé)

### 8.3 Noms de Fichiers

Les fichiers audio téléchargés utilisent directement l'ID vidéo comme nom de fichier (`{video_id}.m4a`), évitant tout problème de caractères spéciaux. Le titre est stocké uniquement dans le cache index JSON et dans le `Sound.name` du profil.

### 8.4 Commandes yt-dlp

```bash
# Télécharger en M4A (best audio) avec remux ffmpeg
yt-dlp -f "bestaudio[ext=m4a]" \
    -o "cache/%(id)s.%(ext)s" \
    --write-info-json \
    --no-playlist \
    --newline \
    --force-overwrite \
    --no-update \
    --socket-timeout 30 \
    --ffmpeg-location "/path/to/bin/" \
    "https://www.youtube.com/watch?v=XXXXXXXXXXX"

# Title is extracted from {id}.info.json after download
# ffmpeg auto-remuxes DASH M4A via [FixupM4a]
```

### 8.4.1 Binary Management

**yt-dlp:** Auto-downloaded from GitHub releases to `{app_data}/bin/yt-dlp.exe`. Checked via version command before use.

**ffmpeg:** Auto-downloaded from `yt-dlp/FFmpeg-Builds` GitHub releases (win64-gpl ZIP). Only `ffmpeg.exe` is extracted from the archive's `bin/` directory. Required because YouTube provides DASH fragmented MP4 audio that must be remuxed to proper M4A for playback.

### 8.4.2 Retry Logic

Transient network errors are retried automatically:
- Up to 3 attempts per download
- 2-second delay between retries
- Partial files cleaned before each retry
- Only network-related errors are retried (connection, timeout, incomplete read)
- Non-retryable errors fail immediately (private video, unavailable, geo-blocked, invalid URL)

### 8.5 Gestion des Erreurs YouTube

| Erreur | Message Utilisateur | Retry? |
|--------|---------------------|--------|
| URL invalide | "Invalid YouTube URL" | Non |
| Vidéo privée | "This video is private or requires sign-in" | Non |
| Vidéo indisponible | "This video is not available" | Non |
| Geo-bloquée | "This video is not available in your region" | Non |
| Erreur réseau | "Network error. Check your internet connection" | Oui (3x) |
| Timeout | (retried automatically) | Oui (3x) |
| yt-dlp non trouvé | Auto-installed | N/A |
| ffmpeg non trouvé | Auto-installed | N/A |

---

## 9. Interface Utilisateur

### 9.1 Design System

#### 9.1.1 Palette de Couleurs (Thème Sombre)

```css
:root {
  /* Backgrounds */
  --bg-primary: #0f0f0f;      /* Fond principal */
  --bg-secondary: #1a1a1a;    /* Panneaux, cartes */
  --bg-tertiary: #252525;     /* Éléments surélevés */
  --bg-hover: #2d2d2d;        /* État hover */
  
  /* Text */
  --text-primary: #ffffff;    /* Texte principal */
  --text-secondary: #a0a0a0;  /* Texte secondaire */
  --text-muted: #666666;      /* Texte discret */
  
  /* Accent */
  --accent-primary: #6366f1;  /* Indigo - actions principales */
  --accent-secondary: #8b5cf6; /* Violet - éléments actifs */
  --accent-success: #22c55e;  /* Vert - succès */
  --accent-warning: #f59e0b;  /* Orange - attention */
  --accent-error: #ef4444;    /* Rouge - erreur */
  
  /* Borders */
  --border-color: #333333;
  --border-focus: #6366f1;
}
```

#### 9.1.2 Typographie

```css
:root {
  --font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
  --font-size-xs: 0.75rem;    /* 12px */
  --font-size-sm: 0.875rem;   /* 14px */
  --font-size-base: 1rem;     /* 16px */
  --font-size-lg: 1.125rem;   /* 18px */
  --font-size-xl: 1.25rem;    /* 20px */
}
```

### 9.2 Layout Principal

```
┌──────────────────────────────────────────────────────────────────┐
│  ┌────────────────────────────────────────────────────────────┐  │
│  │                         HEADER                              │  │
│  │  [Logo] KeyToMusic              [🔊 Vol] [⚙️] [—] [×]       │  │
│  └────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌────────────┬───────────────────────────────────────────────┐  │
│  │            │                                                │  │
│  │  SIDEBAR   │              MAIN CONTENT                      │  │
│  │            │                                                │  │
│  │ ┌────────┐ │  ┌──────────────────────────────────────────┐ │  │
│  │ │Profiles│ │  │              TRACK VIEW                   │ │  │
│  │ │        │ │  │                                           │ │  │
│  │ │ • Shonen│ │  │  [Track: OST Principale] [Vol ████░░] ▼  │ │  │
│  │ │ • Seinen│ │  │                                           │ │  │
│  │ │ • Horror│ │  │  ┌─────────────────────────────────────┐ │ │  │
│  │ │   + New │ │  │  │         KEY ASSIGNMENTS             │ │ │  │
│  │ └────────┘ │  │  │                                       │ │ │  │
│  │            │  │  │  [A] Epic Battle    [S] Sad Theme    │ │ │  │
│  │ ┌────────┐ │  │  │  [D] Victory OST    [F] Tension      │ │ │  │
│  │ │Controls│ │  │  │  [G] Combat         [H] Peaceful     │ │ │  │
│  │ │        │ │  │  │                                       │ │ │  │
│  │ │ [Auto] │ │  │  │  [+ Add Sound]                       │ │ │  │
│  │ │ [Keys] │ │  │  └─────────────────────────────────────┘ │ │  │
│  │ │ [Stop] │ │  │                                           │ │  │
│  │ └────────┘ │  └──────────────────────────────────────────┘ │  │
│  │            │                                                │  │
│  │ ┌────────┐ │  ┌──────────────────────────────────────────┐ │  │
│  │ │  Now   │ │  │              SOUND DETAILS               │ │  │
│  │ │Playing │ │  │  (Appears when a key is selected)        │ │  │
│  │ │        │ │  │                                           │ │  │
│  │ │Track:  │ │  │  Sounds for key [A]:                     │ │  │
│  │ │ OST    │ │  │  ┌─────────────────────────────────────┐ │ │  │
│  │ │        │ │  │  │ 1. Epic Battle OST                  │ │ │  │
│  │ │♪ Epic..│ │  │  │    Momentum: 45.5s  Vol: 100%       │ │ │  │
│  │ │        │ │  │  │    [Edit] [Remove]                  │ │ │  │
│  │ │ ▶━━━━░ │ │  │  ├─────────────────────────────────────┤ │ │  │
│  │ │1:23/5:0│ │  │  │ 2. Hero Theme                       │ │ │  │
│  │ └────────┘ │  │  │    Momentum: 0s     Vol: 80%        │ │ │  │
│  │            │  │  │    [Edit] [Remove]                  │ │ │  │
│  │            │  │  └─────────────────────────────────────┘ │ │  │
│  │            │  │                                           │ │  │
│  │            │  │  Loop Mode: [Single ▼]                   │ │  │
│  │            │  │  [+ Add Sound to Key]                    │ │  │
│  │            │  └──────────────────────────────────────────┘ │  │
│  └────────────┴───────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

### 9.3 Composants Principaux

#### 9.3.1 Header

```tsx
interface HeaderProps {
  masterVolume: number;
  onVolumeChange: (volume: number) => void;
  onSettingsClick: () => void;
  onMinimize: () => void;
  onClose: () => void;
}

// Fonctionnalités:
// - Logo et nom de l'app
// - Slider de volume master (toujours visible)
// - Bouton paramètres (ouvre modal)
// - Boutons de fenêtre (minimiser = réduire en tray, fermer = vraie fermeture)
```

#### 9.3.2 Sidebar - Profiles

```tsx
interface ProfileSidebarProps {
  profiles: Profile[];
  currentProfileId: string | null;
  onProfileSelect: (id: string) => void;
  onProfileCreate: () => void;
  onProfileDelete: (id: string) => void;
  onProfileRename: (id: string, name: string) => void;
}

// Fonctionnalités:
// - Liste des profils avec sélection
// - Bouton "+" pour créer un nouveau profil
// - Click droit pour renommer/supprimer
```

#### 9.3.3 Sidebar - Controls

```tsx
interface ControlsSidebarProps {
  autoMomentum: boolean;
  keyDetectionEnabled: boolean;
  onAutoMomentumToggle: () => void;
  onKeyDetectionToggle: () => void;
  onMasterStop: () => void;
}

// Fonctionnalités:
// - Toggle Auto-Momentum (avec indicateur visuel ON/OFF)
// - Toggle Détection Touches (avec indicateur)
// - Bouton Master Stop (gros bouton rouge)
```

#### 9.3.4 Sidebar - Now Playing

```tsx
interface NowPlayingProps {
  selectedTrackId: string | null;
  tracks: Track[];
}

// Affiche pour chaque piste active:
// - Nom de la piste et du son en cours
// - Slider de progression interactif (seekable, drag-then-release)
// - Bouton Stop (■) par piste
// - Temps actuel / durée totale (formaté MM:SS)
// - "Nothing playing" si aucun son ne joue
//
// Pattern de seek:
// - onChange: met à jour la position locale (seekPosition state)
// - onMouseUp/onTouchEnd: déclenche le seek réel (playSound avec nouvelle position)
// - updateProgress() appelé avant le seek async pour éviter le jump-back du slider
```

#### 9.3.5 Track View

```tsx
interface TrackViewProps {
  tracks: Track[];
  selectedTrackId: string | null;
  onTrackSelect: (id: string) => void;
  onTrackVolumeChange: (id: string, volume: number) => void;
  onTrackRename: (id: string, name: string) => void;
  onTrackCreate: () => void;
  onTrackDelete: (id: string) => void;
}

// Fonctionnalités:
// - Dropdown pour sélectionner la piste à afficher
// - Slider de volume de la piste
// - Double-clic sur le nom de la piste pour renommer (input avec autoFocus)
// - Bouton pour créer/supprimer des pistes
```

#### 9.3.6 Key Assignments Grid

```tsx
interface KeyGridProps {
  keyBindings: KeyBinding[];
  sounds: Sound[];
  selectedKey: string | null;
  onKeySelect: (keyCode: string) => void;
  onAddSound: () => void;
}

// Affichage:
// - Grille de "cartes" représentant chaque touche assignée
// - Chaque carte montre: [Touche] + nom personnalisé (ou premier son) + "X sons"
// - Carte mise en surbrillance si son en cours de lecture (indicateur vert)
// - Bouton "+ Add Sound" pour ouvrir le modal d'ajout
```

#### 9.3.7 Sound Details Panel

```tsx
interface SoundDetailsProps {
  selectedKey: string | null;
  keyBinding: KeyBinding | null;
  sounds: Sound[];
  onSoundEdit: (soundId: string, updates: Partial<Sound>) => void;
  onSoundRemove: (soundId: string) => void;
  onSoundAdd: () => void;
  onLoopModeChange: (mode: LoopMode) => void;
}

// Apparaît quand une touche est sélectionnée
// Affiche:
// - Nom personnalisé (éditable via input text)
// - Bouton "Change Key" pour réassigner la touche du binding entier
//   -> Mode capture: l'utilisateur appuie sur la nouvelle touche
//   -> Si la touche cible a déjà des sons: propose de fusionner
//   -> Met à jour le selectedKey du parent via onKeyChanged callback
// - Bouton "Delete Key" pour supprimer le binding complet
// - Liste des sons assignés à cette touche
// - Pour chaque son: nom, durée, volume slider (temps réel), momentum mini-player
// - Momentum mini-player: number input + range slider + play/stop preview button
// - Boutons "Move" par son (déplacer vers une autre touche via capture)
//   -> Si la touche cible a un binding: ajoute le son
//   -> Si pas de binding: crée un nouveau binding
//   -> Si le binding source devient vide: suppression auto
// - Boutons Remove par son
// - Sélecteur de Track (dropdown des pistes existantes)
// - Sélecteur de Loop Mode
// - Bouton pour ajouter un son à cette touche
// - Panel redimensionnable via divider bar (min 120px, default 256px)
```

#### 9.3.8 Add Sound Modal

```tsx
interface AddSoundModalProps {
  initialFiles?: string[];  // Pre-populated files (from drag & drop)
  onClose: () => void;
}

// Ajout de fichiers:
// - Bouton "Add Files": ouvre le file picker natif (pick_audio_files, multi-select)
// - Drag & Drop: depuis l'OS directement dans le modal ou la fenêtre
//   - Si modal ouvert: les fichiers droppés s'ajoutent à la liste existante
//   - Utilise processedFilesRef pour distinguer mount vs drop subséquent
//   - Safe en React StrictMode (pas de double-ajout)
// - Pas de champ de texte manuel (UX simplifiée)
//
// Sources:
// - Tab "Local": Fichiers locaux via Add Files ou drag & drop
// - Tab "YouTube": Champ URL + bouton "Télécharger"
//   - Téléchargements concurrents supportés (input reste actif)
//   - Chaque download a sa propre barre de progression
//   - Les sons téléchargés s'ajoutent à la liste au fur et à mesure
//
// Configuration:
//    - Touche(s) à assigner (champ texte, ex: "ab")
//      - Les touches cyclent si moins de touches que de sons:
//        "ab" avec 5 sons → a,b,a,b,a
//        "a" avec 5 sons → a,a,a,a,a (tous sur la même touche)
//      - L'indicateur par fichier reflète le cycling en temps réel
//    - Piste (dropdown existantes + "Nouvelle piste")
//    - Per-file momentum editors:
//      - Number input + range slider + play/stop preview per file
//      - Duration auto-fetched via getAudioDuration (symphonia)
//      - Playing one preview auto-stops any other playing preview
//    - Volume individuel (slider)
//
// Bouton "Add All":
//    - Sounds are grouped by key (with cycling) before creating bindings
//    - Multiple sounds on same key → single binding with all sound IDs
```

### 9.4 Modals

#### 9.4.1 Settings Modal

```tsx
interface SettingsModalProps {
  config: AppConfig;
  onConfigUpdate: (updates: Partial<AppConfig>) => void;
}

// Contenu:
// - Master Stop Shortcut: Affichage + bouton "Change" + bouton "Clear"
//   -> Mode capture: "Press keys..." (uses charToKeyCode for layout support)
// - Auto-Momentum Shortcut: Affichage + "Change" / "Clear"
// - Key Detection Shortcut: Affichage + "Change" / "Clear"
//   (Note: Ce shortcut fonctionne même quand la détection est désactivée)
// - Crossfade Duration: Slider (100ms - 2000ms)
// - Key Cooldown: Slider (500ms - 5000ms)
// - Audio Device: Dropdown (system default + available devices)
//   -> Seamless switch: playing sounds resume on new device
// - Export/Import buttons
// - À propos (version, liens, "Open Data Folder", "Open Logs Folder")
```

#### 9.4.2 Error Modal (Fichier Introuvable)

```tsx
// FileNotFoundModal (src/components/Errors/FileNotFoundModal.tsx)
// Utilise errorStore (Zustand) avec une queue d'entrées:
interface SoundNotFoundEntry {
  soundId: string;
  soundName: string;
  path: string;
  trackId: string;
  sourceType: "local" | "youtube";
}

// Affichage queue-based: une erreur à la fois
// - Nom du son + chemin attendu
// - Compteur: "(1 of N)" si plusieurs erreurs
//
// Actions selon sourceType:
// - Local:
//   - "Locate File" → pick_audio_file() → met à jour le chemin du son
//   - "Remove" → supprime le son du profil
//   - "Skip" → passe à l'erreur suivante
// - YouTube:
//   - "Re-download" → appelle addSoundFromYoutube avec l'URL originale
//   - "Remove" → supprime le son du profil
//   - "Skip" → passe à l'erreur suivante
// - "Skip All" (si queue > 1) → dismiss toutes les erreurs restantes
//
// Alimenté par:
// 1. verify_profile_sounds() au chargement du profil
// 2. sound_not_found events pendant la lecture
```

#### 9.4.3 ConfirmDialog (Confirmation Custom)

**Problème:** Le `confirm()` natif du navigateur ne fonctionne pas sur macOS WKWebView (retourne immédiatement sans attendre l'input utilisateur).

**Solution:** Un composant React modal avec un store Zustand pour gérer l'état.

```tsx
// src/stores/confirmStore.ts
interface ConfirmStore {
  isOpen: boolean;
  message: string;
  resolve: ((value: boolean) => void) | null;
  confirm: (message: string) => Promise<boolean>;
  close: (result: boolean) => void;
}

// Usage:
const confirmed = await useConfirmStore.getState().confirm("Supprimer cet élément ?");
if (confirmed) {
  // Procéder à la suppression
}
```

**Composant (`src/components/ConfirmDialog.tsx`):**
- Modal avec fond semi-transparent (`bg-black/60`)
- Deux boutons: "Cancel" (gris) et "Confirm" (indigo, `autoFocus`)
- Thème sombre cohérent avec l'app
- Se ferme automatiquement après choix

**Utilisation dans l'app:**
- Fermeture de fenêtre pendant un export
- Suppression de profil
- Suppression de piste
- Fusion de bindings lors du changement de touche
- Déplacement de sons entre touches

### 9.5 Interactions Drag & Drop

#### 9.5.1 Drop Zone pour Fichiers

```tsx
// Comportement:
// 1. L'utilisateur fait glisser des fichiers audio (Tauri onDragDropEvent)
// 2. La zone s'illumine (overlay) pour indiquer qu'elle accepte le drop
// 3. Au drop:
//    - Si AddSoundModal fermé: ouvre le modal avec les fichiers droppés (initialFiles)
//    - Si AddSoundModal déjà ouvert: append les fichiers à la liste existante
//      (MainContent met à jour droppedFiles → modal détecte new ref via processedFilesRef)
// 4. Seuls les fichiers audio sont acceptés (filtrés via isAudioFile)

// Assignation multiple avec cycling (champ "ad"):
// - Son 1 -> A
// - Son 2 -> D
// - Son 3 -> A (cycle)
// - Son 4 -> D (cycle)
// - Son 5 -> A (cycle)
// etc.
// Avec un seul caractère "a":
// - Tous les sons sont assignés à la touche A
```

### 9.6 Responsive Design

L'application a une taille minimale de 800x600 pixels. Elle peut être redimensionnée mais ne doit pas devenir plus petite.

```css
.app-container {
  min-width: 800px;
  min-height: 600px;
}
```

---

## 10. Sauvegarde et Persistance

### 10.1 Structure des Fichiers

```
[App Data Directory]/
├── config.json              # Configuration globale
├── profiles/
│   ├── {uuid1}.json         # Profil 1
│   ├── {uuid2}.json         # Profil 2
│   └── ...
├── cache/
│   ├── cache_index.json     # Index du cache YouTube
│   ├── waveforms.json       # Disk-persisted waveform cache (LRU, mtime-validated)
│   ├── XXXXXXXXXXX.m4a      # Fichier audio caché (video ID as filename)
│   └── ...
├── bin/
│   ├── yt-dlp.exe           # Auto-downloaded yt-dlp binary
│   └── ffmpeg.exe           # Auto-downloaded ffmpeg binary
├── imported_sounds/         # Sons importés depuis .ktm
│   └── {profile_uuid}/     # Dossier par profil importé
└── logs/
    ├── keytomusic.log.2026-01-31  # Daily rolling log files
    └── ...
```

### 10.2 Chemins par Plateforme

```rust
fn get_app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        // C:\Users\{user}\AppData\Roaming\KeyToMusic\
        dirs::data_dir().unwrap().join("KeyToMusic")
    }
    
    #[cfg(target_os = "macos")]
    {
        // /Users/{user}/Library/Application Support/KeyToMusic/
        dirs::data_dir().unwrap().join("KeyToMusic")
    }
    
    #[cfg(target_os = "linux")]
    {
        // /home/{user}/.local/share/keytomusic/
        dirs::data_dir().unwrap().join("keytomusic")
    }
}
```

### 10.3 Sauvegarde Automatique

La configuration est sauvegardée automatiquement:

- À chaque modification de paramètre
- À la fermeture de l'application
- Toutes les 5 minutes (backup périodique)

```rust
impl ProfileManager {
    pub fn auto_save(&self) {
        // Debounce: attendre 1 seconde après la dernière modification
        // avant de sauvegarder pour éviter les écritures excessives
    }
}
```

### 10.4 Gestion des Fichiers Locaux Manquants

Quand un fichier son local n'est plus trouvé:

1. **À l'ouverture du profil** : Vérifier tous les fichiers locaux
2. **Au déclenchement** : Vérifier juste avant de jouer
3. **Notification** : Jouer un son d'erreur (`resources/sounds/error.mp3`) et afficher le modal

```rust
impl SoundManager {
    pub fn verify_sound_file(&self, sound: &Sound) -> Result<(), SoundError> {
        match &sound.source {
            SoundSource::Local { path } => {
                if !Path::new(path).exists() {
                    Err(SoundError::FileNotFound {
                        sound_id: sound.id.clone(),
                        path: path.clone(),
                    })
                } else {
                    Ok(())
                }
            }
            SoundSource::YouTube { cached_path, .. } => {
                if !Path::new(cached_path).exists() {
                    Err(SoundError::CacheCorrupted {
                        sound_id: sound.id.clone(),
                    })
                } else {
                    Ok(())
                }
            }
        }
    }
}
```

---

## 11. Import/Export

### 11.1 Format d'Export

L'export crée un fichier `.ktm` (KeyToMusic) qui est en réalité un ZIP contenant:

```
export_profile_name.ktm (ZIP)
├── profile.json           # Configuration du profil
├── sounds/
│   ├── sound1.mp3         # Copie des fichiers audio
│   ├── sound2.wav
│   └── ...
├── metadata.json          # Métadonnées d'export
└── waveforms.json         # Données waveform pré-calculées (optionnel)
```

### 11.2 Metadata

```json
{
  "version": "1.0.0",
  "exportedAt": "2024-01-20T14:45:00Z",
  "appVersion": "1.0.0",
  "platform": "windows"
}
```

### 11.3 Processus d'Export

```rust
pub type ProgressCallback = Box<dyn Fn(usize, usize, &str) + Send>;

static EXPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

pub fn export_profile(
    profile_id: &str,
    output_path: &str,
    progress_cb: Option<ProgressCallback>,
) -> Result<(), String> {
    // Reset cancellation flag
    EXPORT_CANCELLED.store(false, Ordering::Relaxed);

    let profile = load_profile(profile_id)?;
    let output = Path::new(output_path);
    let temp_path = output.with_extension("ktm.tmp");

    // Track temp file for crash recovery
    fs::write(export_tracking_path(), temp_path.to_string_lossy().as_bytes()).ok();

    // Create ZIP directly (no temp directory)
    let file = File::create(&temp_path)?;
    let mut zip = ZipWriter::new(file);

    let total = profile.sounds.len();
    let mut updated_profile = profile.clone();

    for (i, sound) in updated_profile.sounds.iter_mut().enumerate() {
        // Check cancellation between each file
        if EXPORT_CANCELLED.load(Ordering::Relaxed) {
            drop(zip);
            fs::remove_file(&temp_path).ok();
            fs::remove_file(export_tracking_path()).ok();
            return Err("Export cancelled".to_string());
        }

        let source_path = sound.get_file_path();
        let filename = make_unique_filename(&source_path);

        // Report progress
        if let Some(ref cb) = progress_cb {
            cb(i + 1, total, &filename);
        }

        // Write file directly into ZIP
        zip.start_file(format!("sounds/{}", filename), options)?;
        let data = fs::read(&source_path)?;
        zip.write_all(&data)?;

        // Update path to relative
        sound.source = SoundSource::Local {
            path: format!("sounds/{}", filename),
        };
    }

    // Write profile.json and metadata.json into ZIP
    // ...

    zip.finish()?;

    // Rename temp to final
    fs::rename(&temp_path, output)?;

    // Remove tracking file on success
    fs::remove_file(export_tracking_path()).ok();

    Ok(())
}
```

### 11.5 Annulation et Nettoyage d'Export

```rust
/// Annuler un export en cours (appelé depuis un autre thread)
pub fn cancel_export() {
    EXPORT_CANCELLED.store(true, Ordering::Relaxed);
}

/// Chemin du fichier de suivi d'export en cours
fn export_tracking_path() -> PathBuf {
    storage::get_app_data_dir().join("export_in_progress.txt")
}

/// Nettoyer un fichier temporaire orphelin (appelé au démarrage)
pub fn cleanup_interrupted_export() {
    let tracking = export_tracking_path();
    if tracking.exists() {
        if let Ok(temp_path) = fs::read_to_string(&tracking) {
            let temp = Path::new(temp_path.trim());
            if temp.exists() {
                fs::remove_file(temp).ok();
            }
        }
        fs::remove_file(&tracking).ok();
    }
}
```

**Mécanisme de sécurité:**
- Le fichier `export_in_progress.txt` contient le chemin du `.ktm.tmp` en cours d'écriture
- Au démarrage de l'app, `cleanup_interrupted_export()` vérifie et supprime les fichiers orphelins
- L'annulation utilise un `AtomicBool` pour la communication inter-threads sans mutex
- La fermeture de fenêtre pendant l'export déclenche `cleanupExportTemp()` avant de fermer

### 11.4 Processus d'Import

```rust
pub async fn import_profile(ktm_path: &Path) -> Result<ProfileId> {
    // Extraire le ZIP
    let temp_dir = tempdir()?;
    extract_zip(ktm_path, temp_dir.path())?;
    
    // Lire le profil
    let profile_json = std::fs::read_to_string(temp_dir.path().join("profile.json"))?;
    let mut profile: Profile = serde_json::from_str(&profile_json)?;
    
    // Générer un nouvel ID pour éviter les conflits
    let new_id = uuid::Uuid::new_v4().to_string();
    profile.id = new_id.clone();
    profile.name = format!("{} (Imported)", profile.name);
    
    // Copier les sons vers le dossier de l'app
    let app_sounds_dir = get_app_data_dir().join("imported_sounds").join(&new_id);
    std::fs::create_dir_all(&app_sounds_dir)?;
    
    for sound in &mut profile.sounds {
        if let SoundSource::Local { path } = &sound.source {
            let source = temp_dir.path().join(path);
            let filename = Path::new(path).file_name().unwrap();
            let dest = app_sounds_dir.join(filename);
            
            std::fs::copy(&source, &dest)?;
            
            // Mettre à jour avec le chemin absolu
            sound.source = SoundSource::Local {
                path: dest.to_string_lossy().to_string(),
            };
        }
    }
    
    // Sauvegarder le profil
    save_profile(&profile)?;
    
    Ok(new_id)
}
```

### 11.6 Import Legacy Save (ancienne version)

L'application peut importer les fichiers de sauvegarde de l'ancienne version de KeyToMusic (Unity-based). Ces fichiers sont des JSON avec un format différent.

#### 11.6.1 Format Legacy

```json
{
  "Sounds": [
    {
      "Key": 68,
      "UserKeyChar": "D",
      "SoundInfos": [
        {
          "uniqueId": "e6fb6419-7c99-432d-917d-ce7e7d6633a2",
          "soundPath": "C:/Users/mehdi/AppData/LocalLow/KeyToMusicCompany/KeyToMusic/WalidPlaylist/Sound.mp3",
          "soundName": "Sound Name",
          "soundMomentum": 118
        }
      ]
    }
  ]
}
```

#### 11.6.2 Mapping des Champs

| Champ Legacy | Champ Nouveau | Transformation |
|---|---|---|
| `Key` (u32) | `keyCode` (String) | VK code → web KeyCode (65→KeyA, 48→Digit0, 112→F1, 221→BracketRight, etc.) |
| `SoundInfos[].uniqueId` | `sound.id` | Conservé tel quel |
| `SoundInfos[].soundPath` | `sound.source` | `{ type: "local", path }` avec `/` → `\` sur Windows |
| `SoundInfos[].soundName` | `sound.name` | Conservé tel quel |
| `SoundInfos[].soundMomentum` | `sound.momentum` | Conservé (f64, secondes) |
| (absent) | `sound.volume` | Défaut: 1.0 |
| (absent) | `sound.duration` | Défaut: 0.0 (calculé au chargement) |
| (absent) | `track` | Track "OST" créé par défaut |
| (absent) | `loopMode` | Défaut: "off" |

#### 11.6.3 Commandes Tauri

```rust
#[tauri::command]
async fn pick_legacy_file() -> Result<Option<String>, String>;

#[tauri::command]
async fn import_legacy_save(path: String) -> Result<Profile, String>;
```

#### 11.6.4 Algorithme de Conversion

```
1. Ouvrir file picker (filtre: *.json)
2. Lire et parser le JSON legacy
3. Créer un nouveau Profile (UUID, timestamps, nom = "{filename} (Legacy)")
4. Créer un Track "OST" par défaut
5. Pour chaque entrée dans Sounds[]:
   a. Convertir Key (u32) en KeyCode string via vk_to_keycode()
   b. Si code inconnu: skip avec warning log
   c. Pour chaque SoundInfo: créer un Sound (source Local, momentum, volume 1.0)
   d. Créer un KeyBinding liant le keyCode aux sound IDs
6. Sauvegarder le profil
7. Retourner le Profile au frontend
```

#### 11.6.5 UI

Bouton "Import Legacy Save" (stylé en jaune) dans la section Import/Export du Settings modal. Affiche le statut de conversion et charge automatiquement le profil importé.

---

## 12. Gestion des Erreurs

### 12.1 Types d'Erreurs

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // Audio
    #[error("Sound file not found: {path}")]
    SoundFileNotFound { sound_id: String, path: String },
    
    #[error("Unsupported audio format: {format}")]
    UnsupportedFormat { format: String },
    
    #[error("Audio playback failed: {reason}")]
    PlaybackFailed { reason: String },
    
    // YouTube
    #[error("Invalid YouTube URL: {url}")]
    InvalidYouTubeUrl { url: String },
    
    #[error("YouTube download failed: {reason}")]
    YouTubeDownloadFailed { reason: String },
    
    #[error("yt-dlp not found")]
    YtDlpNotFound,
    
    // Storage
    #[error("Profile not found: {id}")]
    ProfileNotFound { id: String },
    
    #[error("Failed to save profile: {reason}")]
    SaveFailed { reason: String },
    
    #[error("Failed to load profile: {reason}")]
    LoadFailed { reason: String },
    
    // Import/Export
    #[error("Invalid export file: {reason}")]
    InvalidExportFile { reason: String },
    
    #[error("Export failed: {reason}")]
    ExportFailed { reason: String },
    
    // Keys
    #[error("Key already assigned: {key_code}")]
    KeyAlreadyAssigned { key_code: String },
    
    #[error("Invalid key combination for master stop")]
    InvalidMasterStopShortcut,
}
```

### 12.2 Messages Utilisateur

| Erreur | Message Affiché |
|--------|-----------------|
| `SoundFileNotFound` | "Le fichier audio n'a pas été trouvé. Voulez-vous mettre à jour son emplacement ?" |
| `UnsupportedFormat` | "Ce format audio n'est pas supporté. Formats acceptés : MP3, WAV, OGG, FLAC, M4A/AAC" |
| `InvalidYouTubeUrl` | "L'URL YouTube n'est pas valide" |
| `YouTubeDownloadFailed` | "Échec du téléchargement. Vérifiez votre connexion et l'URL" |
| `YtDlpNotFound` | "yt-dlp n'est pas installé. Installez-le pour télécharger depuis YouTube" |
| `KeyAlreadyAssigned` | "Cette touche est déjà utilisée pour le Master Stop. Choisissez une autre touche" |

### 12.3 Son d'Erreur

Quand une erreur se produit lors du déclenchement d'un son, jouer un bref son d'erreur (`resources/sounds/error.mp3`) pour notifier l'utilisateur sans interrompre sa lecture.

**Implémentation:**
- `SetErrorSoundPath { path }`: Stocke le chemin du son d'erreur dans l'audio thread
- `PlayErrorSound`: Crée un sink one-shot via `SymphoniaSource::new(path, 0.0)`, volume = master * 0.5, `sink.detach()` (fire-and-forget)
- Le chemin est résolu depuis le resource_dir Tauri au démarrage (setup)
- Bundled via `tauri.conf.json` → `bundle.resources: ["../resources/sounds/error.mp3"]`

### 12.4 Logging

**Infrastructure:** Crates `tracing`, `tracing-subscriber`, `tracing-appender`.

```rust
fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let logs_dir = storage::get_app_data_dir().join("logs");
    let file_appender = tracing_appender::rolling::daily(&logs_dir, "keytomusic.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info")))
        .with_ansi(false)
        .init();
    guard
}
```

- Logs écrits dans `{app_data}/logs/keytomusic.log` (rotation journalière)
- Configurable via `RUST_LOG` (défaut: `info`)
- Guard maintenu pour toute la durée du programme

**Commande Open Logs:**
```rust
#[tauri::command]
fn get_logs_folder() -> Result<String, String>;
```
Le frontend utilise `@tauri-apps/plugin-shell` → `open(folder)` pour ouvrir le dossier dans l'explorateur.

### 12.5 Vérification des Fichiers au Chargement

```rust
#[tauri::command]
fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo> {
    // Pour chaque son du profil:
    // - Local: vérifie que file_path existe sur le disque
    // - YouTube: vérifie que cached_path existe
    // Retourne la liste des sons manquants
}

#[derive(Serialize)]
struct MissingSoundInfo {
    sound_id: String,
    sound_name: String,
    file_path: String,
    source_type: String,  // "local" ou "youtube"
}
```

Appelé dans `profileStore.loadProfile()` après chargement. Les résultats alimentent `errorStore.missingQueue` → affichage dans `FileNotFoundModal`.

### 12.6 Commandes File Picker

```rust
#[tauri::command]
async fn pick_audio_file() -> Result<Option<String>, String>;  // Single file (pour "Locate File")

#[tauri::command]
async fn pick_audio_files() -> Result<Vec<String>, String>;   // Multi-file (pour "Add Files")
```

Utilisent `rfd::FileDialog` avec filtre extensions audio (mp3, wav, ogg, flac, m4a, aac).

---

## 13. Instructions de Développement

### 13.1 Initialisation du Projet

```bash
# 1. Créer le projet Tauri avec React + TypeScript
npm create tauri-app@latest keytomusic -- --template react-ts

# 2. Entrer dans le dossier
cd keytomusic

# 3. Installer les dépendances frontend
npm install zustand

# 4. Installer les dépendances de développement
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p

# 5. Ajouter les dépendances Rust dans src-tauri/Cargo.toml
# (voir section 2.2)

# 6. Créer la structure de dossiers
mkdir -p src/components/{Layout,Tracks,Sounds,Keys,Controls,Profiles,Settings}
mkdir -p src/stores src/hooks src/types src/utils
mkdir -p src-tauri/src/{audio,keys,youtube,storage}
mkdir -p resources/sounds resources/icons
```

### 13.2 Ordre de Développement Recommandé

#### Phase 1 : Fondations (Backend Rust)

1. **Types et structures de données** (`src-tauri/src/types.rs`)
2. **Stockage et persistance** (`src-tauri/src/storage/`)
   - Lecture/écriture config.json
   - Gestion des profils
3. **Commandes Tauri de base** (`src-tauri/src/commands.rs`)
   - `get_config`, `set_config`
   - `create_profile`, `load_profile`, `save_profile`, `delete_profile`

#### Phase 2 : Moteur Audio

4. **Moteur audio basique** (`src-tauri/src/audio/engine.rs`)
   - Lecture d'un fichier audio
   - Contrôle du volume
5. **Gestion des pistes** (`src-tauri/src/audio/track.rs`)
   - Pistes multiples indépendantes
   - Volume par piste
6. **Crossfade** (`src-tauri/src/audio/crossfade.rs`)
   - Transition fluide entre sons
7. **Seeking/Streaming** (`src-tauri/src/audio/symphonia_source.rs`)
   - Seeking instantané via symphonia pour le momentum

#### Phase 3 : Détection des Touches

8. **Détecteur de touches** (`src-tauri/src/keys/detector.rs`)
   - Capture globale avec rdev
   - Cooldown
   - Master Stop

#### Phase 4 : Interface Utilisateur

9. **Layout principal** et navigation
10. **Gestion des profils** (sidebar + modals)
11. **Vue des pistes et touches**
12. **Modal d'ajout de son** (fichier local)
13. **Panneau de détails du son**
14. **Paramètres et contrôles**

#### Phase 5 : YouTube

15. **Téléchargeur YouTube** (`src-tauri/src/youtube/`)
16. **Système de cache**
17. **Intégration UI** (champ URL dans modal)

#### Phase 6 : Import/Export

18. **Export** (création du .ktm)
19. **Import** (lecture et intégration)

#### Phase 7 : Polish

20. **Gestion des erreurs complète**
21. **Son d'erreur**
22. **Tests et debug**
23. **Optimisations performances**

### 13.3 Commandes Tauri à Implémenter

```rust
// src-tauri/src/commands.rs

// Configuration
#[tauri::command]
async fn get_config() -> Result<AppConfig, String>;

#[tauri::command]
async fn update_config(updates: serde_json::Value) -> Result<(), String>;

// Profils
#[tauri::command]
async fn list_profiles() -> Result<Vec<ProfileSummary>, String>;

#[tauri::command]
async fn create_profile(name: String) -> Result<Profile, String>;

#[tauri::command]
async fn load_profile(id: String) -> Result<Profile, String>;

#[tauri::command]
async fn save_profile(profile: Profile) -> Result<(), String>;

#[tauri::command]
async fn delete_profile(id: String) -> Result<(), String>;

// Audio
#[tauri::command]
async fn play_sound(track_id: String, sound_id: String, start_position: f64) -> Result<(), String>;

#[tauri::command]
async fn stop_sound(track_id: String) -> Result<(), String>;

#[tauri::command]
async fn stop_all_sounds() -> Result<(), String>;

#[tauri::command]
async fn set_master_volume(volume: f32) -> Result<(), String>;

#[tauri::command]
async fn set_track_volume(track_id: String, volume: f32) -> Result<(), String>;

#[tauri::command]
fn set_sound_volume(track_id: String, sound_id: String, volume: f32) -> Result<(), String>;

// Sons
#[tauri::command]
async fn add_sound_from_file(path: String) -> Result<Sound, String>;

#[tauri::command]
async fn add_sound_from_youtube(url: String) -> Result<Sound, String>;

#[tauri::command]
async fn update_sound(sound_id: String, updates: serde_json::Value) -> Result<Sound, String>;

#[tauri::command]
async fn delete_sound(sound_id: String) -> Result<(), String>;

#[tauri::command]
async fn get_audio_duration(path: String) -> Result<f64, String>;

// Touches
#[tauri::command]
fn set_profile_bindings(state: State<AppState>, bindings: Vec<String>) -> Result<(), String>;

#[tauri::command]
async fn set_key_detection(enabled: bool) -> Result<(), String>;

#[tauri::command]
async fn set_master_stop_shortcut(keys: Vec<String>) -> Result<(), String>;

#[tauri::command]
fn set_key_cooldown(state: State<AppState>, cooldown_ms: u32) -> Result<(), String>;

// Import/Export
#[tauri::command]
async fn export_profile(app: AppHandle, profile_id: String, output_path: String) -> Result<(), String>;

#[tauri::command]
async fn import_profile(ktm_path: String) -> Result<String, String>;

#[tauri::command]
fn cancel_export();

#[tauri::command]
fn cleanup_export_temp();

#[tauri::command]
async fn pick_save_location(default_name: String) -> Result<Option<String>, String>;

#[tauri::command]
async fn pick_ktm_file() -> Result<Option<String>, String>;

// Utilitaires
#[tauri::command]
async fn pick_audio_files() -> Result<Vec<String>, String>;

#[tauri::command]
async fn pick_audio_file() -> Result<Option<String>, String>;

#[tauri::command]
fn get_logs_folder() -> Result<String, String>;

#[tauri::command]
fn get_data_folder() -> Result<String, String>;

#[tauri::command]
fn open_folder(path: String) -> Result<(), String>;

// Profil supplémentaire
#[tauri::command]
fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String>;

#[tauri::command]
fn verify_profile_sounds(profile: Profile) -> Vec<MissingSoundInfo>;

#[tauri::command]
async fn preload_profile_sounds(sounds: Vec<SoundPreloadEntry>) -> Result<HashMap<String, f64>, String>;

// Audio: durée + périphériques
#[tauri::command]
async fn get_audio_duration(path: String) -> Result<f64, String>;

#[tauri::command]
fn list_audio_devices() -> Vec<String>;

#[tauri::command]
fn set_audio_device(device: Option<String>) -> Result<(), String>;

// Waveform et analyse audio
#[tauri::command]
async fn get_waveform(path: String, num_points: u32) -> Result<WaveformData, String>;

#[tauri::command]
async fn get_waveforms_batch(entries: Vec<WaveformRequest>) -> Result<HashMap<String, WaveformData>, String>;

// YouTube: recherche, playlist, binaires
#[tauri::command]
async fn search_youtube(query: String, max_results: u32) -> Result<Vec<YoutubeSearchResult>, String>;

#[tauri::command]
async fn fetch_playlist(url: String) -> Result<YoutubePlaylist, String>;

#[tauri::command]
async fn check_yt_dlp_installed() -> Result<bool, String>;

#[tauri::command]
async fn install_yt_dlp() -> Result<(), String>;

#[tauri::command]
async fn check_ffmpeg_installed() -> Result<bool, String>;

#[tauri::command]
async fn install_ffmpeg() -> Result<(), String>;

// Discovery (YouTube Mix recommendations)
#[tauri::command]
async fn start_discovery(app: AppHandle, state: State<AppState>, profile_id: String) -> Result<Vec<DiscoverySuggestion>, String>;

#[tauri::command]
fn get_discovery_suggestions(profile_id: String) -> Result<Option<Vec<DiscoverySuggestion>>, String>;

#[tauri::command]
fn dismiss_discovery(state: State<AppState>, profile_id: String, video_id: String) -> Result<(), String>;

#[tauri::command]
fn cancel_discovery(state: State<AppState>);

// Pre-download (background caching for discovery carousel)
#[tauri::command]
async fn predownload_suggestion(app: AppHandle, state: State<AppState>, url: String, video_id: String, download_id: String) -> Result<PredownloadResult, String>;

// Legacy import
#[tauri::command]
async fn pick_legacy_file() -> Result<Option<String>, String>;

#[tauri::command]
async fn import_legacy_save(path: String) -> Result<Profile, String>;
```

### 13.4 Events Tauri (Backend → Frontend)

```rust
// Émettre depuis Rust
app_handle.emit_all("sound_started", SoundStartedPayload {
    track_id: "...",
    sound_id: "...",
}).ok();

// Écouter côté React
import { listen } from '@tauri-apps/api/event';

useEffect(() => {
    const unlisten = listen('sound_started', (event) => {
        const { track_id, sound_id } = event.payload;
        // Mettre à jour le state
    });
    
    return () => { unlisten.then(f => f()); };
}, []);
```

**Events disponibles:**
- `sound_started` - `{ trackId, soundId }` - Un son commence à jouer
- `sound_ended` - `{ trackId, soundId }` - Un son a fini de jouer
- `playback_progress` - `{ trackId, position }` - Position de lecture (émis toutes les 250ms)
- `key_pressed` - `{ keyCode, withShift }` - Touche détectée
- `master_stop_triggered` - `{}` - Master stop activé
- `toggle_key_detection` - `{}` - Toggle raccourci détection
- `toggle_auto_momentum` - `{}` - Toggle raccourci auto-momentum
- `sound_not_found` - `{ soundId, path, trackId }` - Fichier audio introuvable
- `audio_error` - `{ message }` - Erreur audio générique
- `export_progress` - `{ current, total, filename }` - Progression de l'export
- `youtube_download_progress` - `{ downloadId, status, progress }` - Progression du téléchargement YouTube
- `discovery_started` - `{}` - Début de la recherche Discovery
- `discovery_progress` - `{ current, total, seedName }` - Progression des seeds traités
- `discovery_partial` - `[DiscoverySuggestion[]]` - Résultats intermédiaires (streaming, après chaque seed)
- `discovery_complete` - `{ count }` - Nombre de résultats finaux
- `discovery_error` - `{ message }` - Erreur pendant la discovery

### 13.5 Configuration Tauri

```json
// src-tauri/tauri.conf.json
{
  "build": {
    "beforeBuildCommand": "npm run build",
    "beforeDevCommand": "npm run dev",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "package": {
    "productName": "KeyToMusic",
    "version": "1.0.0"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "shell": {
        "all": false,
        "open": true
      },
      "dialog": {
        "all": true
      },
      "fs": {
        "all": true,
        "scope": ["$APPDATA/**", "$HOME/**"]
      },
      "path": {
        "all": true
      },
      "globalShortcut": {
        "all": true
      }
    },
    "bundle": {
      "active": true,
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.keytomusic.app",
      "targets": "all"
    },
    "windows": [
      {
        "title": "KeyToMusic",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "systemTray": {
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    }
  }
}
```

**Capabilities Tauri 2 (src-tauri/capabilities/default.json):**

```json
{
  "identifier": "default",
  "description": "Default capabilities for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:webview:default",
    "core:window:default",
    "core:window:allow-destroy",
    "core:window:allow-close",
    "shell:allow-open"
  ]
}
```

> **Note:** `core:window:allow-destroy` et `core:window:allow-close` sont requis pour que `onCloseRequested` fonctionne correctement. Ces permissions ne sont PAS incluses dans `core:window:default`.

### 13.6 Notes Importantes pour le Développement

1. **Latence audio** : Utiliser SymphoniaSource pour TOUTE la lecture (pas seulement le momentum seeking) pour un support de format consistant
2. **Thread safety** : Le moteur audio doit tourner dans un thread séparé avec communication via channels
3. **Mémoire** : Surveiller l'utilisation mémoire avec les fichiers longs
4. **Cross-platform** : Tester régulièrement sur les 3 OS cibles
5. **yt-dlp & ffmpeg** : Auto-téléchargés dans `{app_data}/bin/`. yt-dlp depuis GitHub releases, ffmpeg depuis yt-dlp/FFmpeg-Builds. Pas d'installation manuelle requise.
6. **Format M4A** : YouTube fournit du DASH fMP4 qui nécessite ffmpeg pour être remuxé en M4A lisible. Le feature `isomp4` de symphonia est requis pour le playback.
7. **macOS Key Detection** : Utiliser CGEventTap au lieu de rdev sur macOS 13+ pour éviter le crash lié à `TSMGetInputSourceProperty`. Le code est dans `macos_listener.rs`.
8. **Dialogs natifs** : Ne pas utiliser `confirm()` ou `alert()` du navigateur sur macOS (WKWebView ne les supporte pas correctement). Utiliser `ConfirmDialog` + `confirmStore` à la place.

---

## 14. Fonctionnalités Phase 8 ✅

### 14.1 Duplication de Profil ✅ IMPLÉMENTÉ

Permet de dupliquer un profil existant pour créer une variante.

**Backend (`storage/profile.rs`):**
```rust
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    let source = load_profile(id)?;
    let new_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let name = new_name.unwrap_or_else(|| format!("{} (Copy)", source.name));
    let new_profile = Profile {
        id: new_id,
        name,
        created_at: now.clone(),
        updated_at: now,
        sounds: source.sounds,
        tracks: source.tracks,
        key_bindings: source.key_bindings,
    };
    save_profile(&new_profile)?;
    Ok(new_profile)
}
```

**Commande Tauri (`commands.rs`):**
```rust
#[tauri::command]
pub fn duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String> {
    storage::duplicate_profile(id, new_name)
}
```

**Frontend (`ProfileSelector.tsx`):** Bouton "Duplicate" (icône SVG) à côté du bouton "Delete".

### 14.2 Raccourcis Clavier Combinés (Modificateurs) ✅ IMPLÉMENTÉ

Permet d'utiliser des combinaisons de touches comme triggers (ex: `Ctrl+A`, `Shift+F1`, `Alt+1`).

**Status:**
- ✅ Backend (`detector.rs`): Émet les codes combinés
- ✅ Frontend detection (`useKeyDetection.ts`): Match les codes combinés avec fallback
- ✅ Key mapping (`keyMapping.ts`): Fonctions d'affichage et validation
- ✅ Frontend UI (`AddSoundModal.tsx`): KeyCaptureSlot composant avec capture de touches

**Format de stockage:** Notation combinée `keyCode: "Ctrl+KeyA"` (backward compatible).

**Ordre des modificateurs:** `Ctrl > Shift > Alt > Key` (consistant backend/frontend).

**Détection backend (`detector.rs`):**
```rust
// Build combined key code with modifiers (Ctrl, Alt, Shift)
let has_ctrl = pressed.contains("ControlLeft") || pressed.contains("ControlRight");
let has_alt = pressed.contains("AltLeft") || pressed.contains("AltRight");
let has_shift = pressed.contains("ShiftLeft") || pressed.contains("ShiftRight");

let mut combo = String::new();
if has_ctrl { combo.push_str("Ctrl+"); }
if has_shift { combo.push_str("Shift+"); }
if has_alt { combo.push_str("Alt+"); }
combo.push_str(&code);

cb(KeyEvent::KeyPressed { key_code: combo, with_shift: has_shift });
```

**Frontend detection avec fallback (`useKeyDetection.ts`):**
```typescript
// 1. Essayer le match exact (Ctrl+A)
let binding = profile.keyBindings.find((kb) => kb.keyCode === payload.keyCode);
let useModifierForMomentum = false;

// 2. Fallback: si pas de match et combo contient "+", essayer la touche de base
if (!binding && payload.keyCode.includes("+")) {
  const parts = payload.keyCode.split("+");
  const baseKey = parts[parts.length - 1];
  binding = profile.keyBindings.find((kb) => kb.keyCode === baseKey);
  // Si fallback avec Shift, appliquer momentum
  if (binding && payload.withShift) {
    useModifierForMomentum = true;
  }
}
```

**Validation des conflits (`keyMapping.ts`):**
```typescript
export function checkKeyComboConflict(keyCode: string): { type: 'error' | 'warning'; message: string } | null {
  const forbidden = ["Ctrl+C", "Ctrl+V", "Ctrl+X", "Ctrl+Z", "Ctrl+Y", "Ctrl+A", "Ctrl+S", ...];
  const warnings = ["Ctrl+1", "Ctrl+2", ...]; // Browser tabs
  // ...
}
```

#### 14.2.1 UI AddSoundModal avec KeyCaptureSlot ✅

L'UI utilise des composants `KeyCaptureSlot` pour capturer les touches avec support complet des modificateurs:

```
Keys:
┌────────────────────────────────────────┐
│ 1. [Click to capture]  →  [Ctrl+A] [×] │
│ 2. [Click to capture]  →  [Z]      [×] │
│ 3. [+ Add key]                         │
└────────────────────────────────────────┘

Sounds (5):                  Assigned to:
├─ epic_battle.mp3           → Ctrl+A (key 1)
├─ calm_ambient.mp3          → Z (key 2)
├─ victory_theme.mp3         → Ctrl+A (cycle)
├─ tension.mp3               → Z (cycle)
└─ finale.mp3                → Ctrl+A (cycle)
```

**Composant `KeyCaptureSlot`:**
- Click → mode capture → "Press key..."
- Capture touche(s) avec modifiers (Ctrl, Shift, Alt)
- Affiche "Ctrl+A" ou "Shift+F1"
- Bouton × pour supprimer
- Réutilisable (Settings global shortcuts, AddSoundModal, SoundDetails)

### 14.3 Système Undo/Redo ✅ IMPLÉMENTÉ

Permet d'annuler et rétablir les modifications de profil via Ctrl+Z / Ctrl+Y.

**Store (`src/stores/historyStore.ts`):**
```typescript
interface ProfileState {
  sounds: Sound[];
  tracks: Track[];
  keyBindings: KeyBinding[];
}

interface HistoryEntry {
  timestamp: number;
  actionName: string;       // "Add sound", "Delete binding", etc.
  previousState: ProfileState;
}

interface HistoryStore {
  past: HistoryEntry[];     // Stack pour undo
  future: HistoryEntry[];   // Stack pour redo

  pushState: (actionName: string, previousState: ProfileState) => void;
  undo: () => HistoryEntry | null;
  redo: () => HistoryEntry | null;
  clear: () => void;
  canUndo: () => boolean;
  canRedo: () => boolean;
}
```

**Actions annulables (intégrées dans `profileStore.ts`):**
- `addSound` / `removeSound` / `updateSound`
- `addTrack` / `removeTrack` / `updateTrack`
- `addKeyBinding` / `updateKeyBinding` / `removeKeyBinding`

**Actions NON annulables:**
- Création/suppression de profil
- Téléchargements YouTube
- Mises à jour de `duration` (preload)
- Mises à jour de `currentIndex` (playback)
- Modifications de configuration globale

**Intégration avec profileStore:**
```typescript
// Exemple dans addSound:
addSound: (sound) => {
  const prev = captureProfileState(get().currentProfile);  // Capture avant
  set((state) => ({
    currentProfile: state.currentProfile
      ? { ...state.currentProfile, sounds: [...state.currentProfile.sounds, sound] }
      : null,
  }));
  if (prev) {
    useHistoryStore.getState().pushState("Add sound", prev);
  }
}
```

**Hook (`src/hooks/useUndoRedo.ts`):**
```typescript
export function useUndoRedo() {
  const { undo, redo } = useProfileStore();
  const addToast = useToastStore((s) => s.addToast);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Skip if text input focused (but not range/checkbox inputs like sliders)
      const target = e.target as HTMLElement;
      if (target instanceof HTMLTextAreaElement) return;
      if (target instanceof HTMLInputElement && target.type !== "range" && target.type !== "checkbox") return;

      const isMac = navigator.platform.toUpperCase().includes("MAC");
      const isUndo = (e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey;
      const isRedo = (e.ctrlKey && e.key === "y") || (isMac && e.metaKey && e.shiftKey && e.key === "z");

      if (isUndo) {
        e.preventDefault();
        const result = undo();
        if (result) addToast(`Undo: ${result.actionName}`, "info");
      }
      if (isRedo) {
        e.preventDefault();
        const result = redo();
        if (result) addToast(`Redo: ${result.actionName}`, "info");
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [undo, redo, addToast]);
}
```

**Intégration App.tsx:** `useUndoRedo()` est appelé dans le composant App.

### 14.4 Multi-Key Chords (Accords) ✅ IMPLÉMENTÉ

Support pour presser plusieurs touches non-modifier simultanément (comme un accord de piano), avec un système de détection inspiré des jeux de combat (Street Fighter, Tekken).

**Exemple:** `KeyA+KeyZ` = A et Z pressés en même temps.

**Avantage combinatoire:**
| Type | Combinaisons (~50 touches) |
|------|----------------------------|
| 1 touche | 50 |
| 2 touches | C(50,2) = 1,225 |
| 3 touches | C(50,3) = 19,600 |
| + Modifiers (×8) | ×8 pour chaque |

**Architecture: Trie (prefix tree)**

Le système utilise un Trie pour une détection optimale des combos:

```rust
// src-tauri/src/keys/chord.rs
pub struct ComboTrie { root: TrieNode }
pub struct ChordDetector {
    trie: ComboTrie,
    current_combo: Vec<String>,
    timer: Option<Instant>,
    window_ms: u32,  // configurable (20-100ms, défaut: 30ms)
}
```

**Algorithme de détection:**
- Trigger immédiat quand le combo atteint un "leaf" (pas d'extensions possibles)
- Timer uniquement quand des extensions existent (latence conditionnelle)

```
Bindings: A, A+Z, A+Z+E

A pressé → Extensions possibles (A+Z, A+Z+E) → Start 30ms timer
Z pressé → Extensions possibles (A+Z+E) → Continue timer
E pressé → Leaf node (pas de A+Z+E+*) → TRIGGER IMMÉDIAT "A+Z+E"
```

**Latence optimisée:**
- 0ms si la touche est un leaf (pas d'extensions dans le profil)
- 0ms si le combo actuel est un leaf (trigger immédiat)
- 30-50ms uniquement quand des extensions sont possibles

**Format:** Modifiers d'abord (Ctrl > Shift > Alt), puis base keys triées alphabétiquement.
- `"KeyZ+KeyA"` → normalisé en `"KeyA+KeyZ"`
- `"Ctrl+KeyZ+KeyA"` → normalisé en `"Ctrl+KeyA+KeyZ"`

**Configuration:** `config.chordWindowMs`: 20-100ms (configurable dans Settings, défaut: 30ms)

**Fichiers clés:**
- `src-tauri/src/keys/chord.rs` - ComboTrie et ChordDetector
- `src-tauri/src/keys/detector.rs` - Intégration avec la détection globale
- `src/utils/keyMapping.ts` - `normalizeCombo()`, `buildComboFromPressedKeys()`

### 14.5 Modificateur Momentum Configurable ✅ IMPLÉMENTÉ

L'utilisateur peut choisir quel modificateur déclenche le momentum playback.

**Problème résolu:** Numpad + Shift ne fonctionne pas (limitation hardware - voir section 14.6).

**Configuration:**
```typescript
// config.json
{
  "momentumModifier": "Shift" | "Alt" | "Ctrl" | "None"
}
```

**Options:**
| Modifier | Avantage | Inconvénient |
|----------|----------|--------------|
| Shift (défaut) | Intuitif | Conflit Numpad |
| Alt | Fonctionne partout | Moins naturel |
| Ctrl | Fonctionne partout | Conflits système possibles |
| None | Désactivé | Perd la flexibilité |

**UI:** Dropdown dans Settings, section "Key Detection".

**Règle de priorité:** Le match exact de binding a priorité. Ex: si "Ctrl+A" est un binding, presser Ctrl+A déclenche ce binding normalement, pas "A" avec momentum.

**Détection de conflits:**
- Quand on change le momentum modifier: vérifie si des shortcuts existants utilisent le modifier + une touche bindée
- Quand on configure un shortcut: vérifie s'il utilise le momentum modifier + une touche bindée
- Affiche des toasts d'avertissement et des icônes persistantes:
  - À côté des shortcuts en conflit dans Settings
  - À côté du dropdown Momentum Modifier si des conflits existent
  - Sur les touches concernées dans le KeyGrid

**Fichiers clés:**
- `src/types/index.ts` - Type `MomentumModifier`
- `src/stores/settingsStore.ts` - Action `setMomentumModifier()`
- `src-tauri/src/types.rs` - Enum `MomentumModifier`
- `src/hooks/useKeyDetection.ts` - `hasMomentumModifier()` check
- `src/components/Settings/SettingsModal.tsx` - Dropdown UI avec détection de conflits
- `src/components/Keys/KeyGrid.tsx` - Icônes d'avertissement sur les touches en conflit
- `src/components/common/WarningTooltip.tsx` - Composant réutilisable
- `src/utils/keyMapping.ts` - `findMomentumConflicts()`, `getKeyMomentumConflict()`

### 14.6 Limitations Connues

#### Numpad + Shift (Hardware)

Quand NumLock est ON et Shift est pressé avec une touche numpad, le système d'exploitation envoie la touche alternative au lieu de "Shift+Numpad4":

| Touche | NumLock ON | Shift + NumLock ON |
|--------|------------|-------------------|
| Numpad4 | "Numpad4" | "ArrowLeft" ou "End" |
| Numpad8 | "Numpad8" | "ArrowUp" |
| Numpad2 | "Numpad2" | "ArrowDown" |

C'est un comportement standard des claviers depuis DOS, pas un bug de l'application.

**Workarounds:**
1. Utiliser Alt ou Ctrl comme momentum modifier (Phase 14.5)
2. Assigner des bindings explicites pour les touches alternatives
3. Désactiver NumLock

---

## 15. Smart Discovery System (Phase 8+) ✅

Le système Smart Discovery permet de découvrir de nouveaux sons basés sur les sons YouTube déjà présents dans le profil. Il combine plusieurs sous-systèmes : YouTube Search, Playlist Import, Waveform RMS, Auto-Momentum, et YouTube Mix Discovery.

### 15.1 YouTube Search

Recherche directe de vidéos YouTube depuis l'UI via yt-dlp:

```rust
#[tauri::command]
async fn search_youtube(query: String, max_results: u32) -> Result<Vec<YoutubeSearchResult>, String>;
```

- Utilise `yt-dlp "ytsearch{N}:{query}" --flat-playlist --dump-json`
- Retourne titre, durée, channel, thumbnail, URL
- Intégré dans le tab YouTube de AddSoundModal

### 15.2 YouTube Playlist Import

Import en bulk depuis des playlists YouTube:

```rust
#[tauri::command]
async fn fetch_playlist(url: String) -> Result<YoutubePlaylist, String>;
```

**Détection d'URL:**
| Type | Exemple | Comportement |
|------|---------|--------------|
| Vidéo simple | `watch?v=abc` | Download direct |
| Vidéo dans playlist | `watch?v=abc&list=PLxxx` | Checkbox "Download entire playlist" |
| Playlist pure | `playlist?list=PLxxx` | Mode playlist direct |

**Préférence persistée:** `playlistImportEnabled` dans `config.json` (cross-profil).

### 15.3 Waveform RMS (Visualisation d'Énergie Audio)

Affiche une courbe d'énergie dans l'éditeur de momentum pour positionner le point de départ visuellement.

```rust
// audio/analysis.rs
pub fn compute_waveform(file_path: &str, num_points: u32) -> Result<WaveformData, String>;
```

**Algorithme:**
1. Décode les samples via symphonia
2. Calcule le RMS par segments (typiquement 250 points)
3. Normalise entre 0.0 et 1.0
4. Lisse avec moyenne mobile (fenêtre de 3)

**Composant `WaveformDisplay.tsx`:**
- Dual-canvas: canvas statique (waveform + marqueurs) + canvas curseur (position de lecture)
- Marqueur momentum draggable (ligne jaune)
- Clic direct sur la waveform = déplacer le marqueur
- Curseur de lecture en temps réel pendant le preview
- État loading (skeleton) et fallback (slider classique si erreur)

**Cache:**
- Disk-persisted dans `cache/waveforms.json` (LRU, max 50 entrées)
- Invalidation par mtime du fichier audio
- Vider au changement de profil (cache mémoire)

### 15.4 Auto-Momentum Detection

Détecte automatiquement le point de "pré-pic" (fin de l'intro calme, début du buildup) et place un marqueur visuel suggéré sur la waveform.

```rust
// audio/analysis.rs
pub fn detect_momentum_point(waveform: &WaveformData) -> Option<f64>;
```

**Algorithme de gradient:**
1. Calcule la dérivée des points RMS
2. Parcourt depuis 5% du morceau
3. Cherche le premier segment où:
   - Le gradient est positif et significatif
   - Les segments précédents avaient une énergie basse et stable
4. Retourne `Some(timestamp_seconds)` ou `None`

**Marqueur visuel:**
- Ligne verticale en pointillés (blanc 30% opacity), distinct du marqueur momentum user (jaune solide)
- Au clic: accepte la suggestion et met à jour le momentum
- Pas affiché si: suggestion null, trop proche du momentum actuel, ou momentum déjà modifié manuellement

### 15.5 YouTube Mix Discovery

Recommandation de nouveaux sons basée sur les YouTube Mix des sons existants.

```rust
// discovery/engine.rs — DiscoveryEngine method
pub async fn generate_suggestions(
    &self,
    seeds: Vec<SeedInfo>,                                    // max 15 seeds
    existing_ids: Vec<String>,                               // IDs déjà dans le profil (filtrés)
    yt_dlp_bin: PathBuf,                                     // chemin vers yt-dlp
    progress_callback: impl Fn(usize, usize, &str) + Send + Sync,  // (current, total, seed_name)
    partial_callback: impl Fn(&[DiscoverySuggestion]) + Send + Sync,
) -> Vec<DiscoverySuggestion>;
```

**Algorithme:**
1. Extrait les video IDs YouTube du profil comme "seeds" (max 15)
2. Pour chaque seed: `fetch_mix(video_id)` via `yt-dlp --flat-playlist` sur YouTube Mix (`&list=RD{id}`)
3. Concurrence via `buffer_unordered(10)` (10 seeds simultanés)
4. Agrégation cross-seed via `OccurrenceMap`: les suggestions qui apparaissent dans plusieurs Mix reçoivent un `occurrence_count` plus élevé
5. Filtre: durée 30-900s, exclut les sons déjà dans le profil
6. Tri par occurrence_count décroissant, retourne top 30
7. Streaming: émet `discovery_partial` après chaque seed traité via callback

**Frontend (DiscoveryPanel.tsx):**
- Bouton "Discover" dans la sidebar (nécessite ≥1 son YouTube dans le profil)
- Carousel horizontal avec flèches de navigation
- Preview audio (30s) avec contrôle play/pause
- Bouton "Add" pour ajouter à un binding
- Bouton "Dismiss" pour retirer de la liste

**Smart Auto-Assignment:**
- Analyse le profil pour détecter le mode (single-sound vs multi-sound)
- Suggest la touche et piste appropriées basées sur les patterns existants
- `profileAnalysis.ts`: analyse les distributions de tracks, loop modes, binding patterns

**Pre-download Pipeline (`useDiscoveryPredownload.ts`):**
- Background downloading autour de la position du carousel
- Fenêtre asymétrique: [current-2, ..., current+3] (2 derrière, 3 devant)
- Max 3 téléchargements concurrents
- Chaque download retourne `PredownloadResult` (cachedPath, waveform, duration)
- Le `suggestedMomentum` de la waveform est appliqué automatiquement

**Commandes Tauri:**
```rust
start_discovery(profile_id: String)  // Lance la recherche (async, emits events)
get_discovery_suggestions(profile_id: String)  // Récupère depuis le cache
dismiss_discovery(profile_id: String, video_id: String)  // Retire une suggestion
cancel_discovery()  // Annule la recherche en cours (AtomicBool)
predownload_suggestion(url: String, video_id: String, download_id: String) -> PredownloadResult
```

### 15.6 Waveform Batch

Pour les imports multiples (playlist ou multi-file), une commande batch parallélise le calcul:

```rust
#[tauri::command]
async fn get_waveforms_batch(
    entries: Vec<WaveformRequest>
) -> Result<HashMap<String, WaveformData>, String>;

pub struct WaveformRequest {
    pub path: String,
    pub num_points: u32,
}
```

- Utilise le Rayon thread pool partagé (2 threads, `AppState::cpu_pool`)
- Chaque résultat inclut `suggested_momentum`

---

## Annexes

### A. Formats Audio Supportés

| Format | Extension | Notes |
|--------|-----------|-------|
| MP3 | `.mp3` | Le plus commun, bonne compression |
| WAV | `.wav` | Sans perte, fichiers volumineux |
| OGG | `.ogg` | Bonne qualité, open source |
| FLAC | `.flac` | Sans perte, meilleure compression que WAV |
| AAC | `.aac`, `.m4a` | Bonne qualité, commun sur YouTube |
| WebM | `.webm` | Format conteneur ouvert, supporté par le frontend (drag & drop) |

### B. Raccourcis Clavier par Défaut

| Action | Raccourci | Note |
|--------|-----------|------|
| Master Stop | `Ctrl + Shift + S` (configurable) | Fonctionne même si Key Detection est off |
| Toggle Key Detection | Configurable dans Settings | Fonctionne même si Key Detection est off |
| Toggle Auto-Momentum | Configurable dans Settings | Fonctionne même si Key Detection est off |
| Momentum Modifier | `Shift` (configurable: Shift/Ctrl/Alt/None) | Configurable dans Settings > Key Detection |
| Undo | `Ctrl + Z` (Windows/Linux) / `Cmd + Z` (macOS) | Annule la dernière action de profil |
| Redo | `Ctrl + Y` (Windows/Linux) / `Cmd + Shift + Z` (macOS) | Rétablit l'action annulée |

### C. Limites Techniques

| Paramètre | Limite |
|-----------|--------|
| Nombre de pistes | 20 max |
| Nombre de sons par touche | Illimité |
| Durée d'un son | Illimitée (streaming) |
| Seeking momentum | Instantané (symphonia byte-level seek) |
| Cooldown minimum | 0ms |
| Cooldown maximum | 5000ms |
| Crossfade minimum | 100ms |
| Crossfade maximum | 2000ms |
| Chord window | 20-100ms (défaut: 30ms) |
| Waveform cache | 50 entrées max (LRU, disk-persisted) |
| History (undo/redo) | 50 entrées max |
| Progress emission | 250ms interval |
| Discovery pre-download | 3 concurrent max |

---

**Document généré le** : 2024-01-20
**Dernière mise à jour** : 2026-02-01
**Version** : 1.4.0
**Auteur** : Document technique pour Claude Code

### Changelog
- **v1.4.0** (2026-02-01):
  - Phase 8 complétée: Combined Shortcuts UI (KeyCaptureSlot) ✅, Multi-Key Chords (Trie-based) ✅, Momentum Modifier Configurable ✅
  - Smart Discovery System complet (section 15): YouTube Search, Playlist Import, Waveform RMS, Auto-Momentum Detection, YouTube Mix Discovery
  - Nouveaux types: WaveformData, YoutubeSearchResult, YoutubePlaylist, DiscoverySuggestion, MomentumModifier
  - Nouveaux champs AppConfig: chordWindowMs, momentumModifier, playlistImportEnabled
  - Nouvelles commandes: discovery (start, get, dismiss, cancel), waveform (get, batch), search_youtube, fetch_playlist, predownload_suggestion, check/install yt-dlp/ffmpeg
  - Nouveaux events: discovery_started, discovery_progress, discovery_partial, discovery_complete, discovery_error
  - Structure de fichiers mise à jour: discovery/, analysis.rs, chord.rs, waveforms.json cache
  - Optimisations UI: playback_progress réduit à 250ms, dual-canvas WaveformDisplay, batched duration updates
- **v1.3.0** (2026-01-25):
  - Phase 8 implémentée: Profile Duplication ✅, Undo/Redo ✅, Combined Shortcuts (backend ✅)
  - Ajout section 14.2.1 (UI KeyCaptureSlot)
  - Ajout section 14.4 (Multi-Key Chords)
  - Ajout section 14.5 (Momentum Modifier Configurable)
  - Ajout section 14.6 (Limitations Connues - Numpad+Shift)
- **v1.2.0** (2025-01-25): Ajout de la section 14 (Features: Profile Duplication, Combined Shortcuts, Undo/Redo)
- **v1.1.0** (2025-01-25): Ajout de l'implémentation macOS CGEventTap (section 6.7), ConfirmDialog (section 9.4.3), dépendances conditionnelles par plateforme
