// Identifiants uniques
export type SoundId = string;      // UUID v4
export type TrackId = string;      // UUID v4
export type ProfileId = string;    // UUID v4
export type KeyCode = string;      // Ex: "KeyA", "Digit1", "F5", "ShiftLeft"

// Source d'un son
export type SoundSource =
  | { type: "local"; path: string }
  | { type: "youtube"; url: string; cachedPath: string };

// Mode de boucle
export type LoopMode = "off" | "random" | "single" | "sequential";

// Base mood axis (8 values)
export type BaseMood =
  | "epic"
  | "tension"
  | "sadness"
  | "comedy"
  | "romance"
  | "horror"
  | "peaceful"
  | "mystery";

// Intensity axis (3 levels)
export type MoodIntensity = 1 | 2 | 3;

// Combined mood tag
export interface MoodTag {
  mood: BaseMood;
  intensity: MoodIntensity;
}

// Legacy alias
export type MoodCategory = BaseMood;

// Modificateur pour déclencher le momentum
export type MomentumModifier = "Shift" | "Ctrl" | "Alt" | "None";

// Configuration d'un son individuel
export interface Sound {
  id: SoundId;
  name: string;
  source: SoundSource;
  momentum: number;           // Position de départ en secondes (décimales autorisées)
  volume: number;             // 0.0 à 1.0 (volume individuel du son)
  duration: number;           // Durée totale en secondes (calculée au chargement)
  resolvedVideoId?: string;   // YouTube video ID resolved from local file metadata/name
}

// Configuration d'une touche
export interface KeyBinding {
  keyCode: KeyCode;
  trackId: TrackId;
  soundIds: SoundId[];        // Liste des sons assignés à cette touche
  loopMode: LoopMode;
  currentIndex: number;       // Index actuel pour le mode sequential
  name?: string;              // Nom personnalisé (défaut: nom du premier son)
  mood?: BaseMood;             // Mood tag for AI-triggered playback
  moodIntensity?: MoodIntensity; // Minimum intensity to trigger (undefined = any)
}

// Configuration d'une piste
export interface Track {
  id: TrackId;
  name: string;
  volume: number;             // 0.0 à 1.0 (volume de la piste)
}

// Profil utilisateur (une "playlist" / configuration complète)
export interface Profile {
  id: ProfileId;
  name: string;
  createdAt: string;          // ISO 8601
  updatedAt: string;          // ISO 8601
  sounds: Sound[];
  tracks: Track[];
  keyBindings: KeyBinding[];
}

// Configuration globale de l'application
export interface AppConfig {
  masterVolume: number;           // 0.0 à 1.0
  autoMomentum: boolean;          // Si true, tous les sons démarrent au momentum
  keyDetectionEnabled: boolean;   // Si true, les touches sont détectées
  stopAllShortcut: KeyCode[];     // Combinaison de touches (ex: ["ControlLeft", "ShiftLeft", "KeyS"])
  autoMomentumShortcut: KeyCode[];  // Shortcut pour toggle auto-momentum
  keyDetectionShortcut: KeyCode[];  // Shortcut pour toggle key detection (fonctionne même si désactivé)
  crossfadeDuration: number;      // Durée du crossfade en millisecondes (défaut: 500)
  keyCooldown: number;            // Cooldown global entre pressions en millisecondes (défaut: 1500)
  currentProfileId: ProfileId | null;
  audioDevice: string | null;     // null = follow system default, string = force specific device
  chordWindowMs: number;          // Multi-key chord detection window in ms (default: 30, range: 20-100)
  momentumModifier: MomentumModifier; // Modifier key to trigger momentum (default: "Shift")
  playlistImportEnabled: boolean;     // Remember "download entire playlist" checkbox state
  moodAiEnabled: boolean;             // Enable Manga Mood AI integration
  moodApiPort: number;                // HTTP API port for external tools (default: 8765)
  moodEntryThreshold: number;         // Score threshold to enter a new mood (default: 0.55)
  moodExitThreshold: number;          // Score threshold to exit current mood (default: 0.25)
  moodDwellPages: number;             // Min pages before mood can change (default: 2)
  moodWindowSize: number;             // Sliding window size for mood averaging (default: 5)
}

export interface InputRuntime {
  isLinux: boolean;
  isWayland: boolean;
  browserKeyFallback: boolean;
}

export interface LinuxInputAccessStatus {
  supported: boolean;
  sessionType: string;
  backgroundDetectionAvailable: boolean;
  canAutoFix: boolean;
  reloginRecommended: boolean;
  accessibleKeyboardDevices: string[];
  keyboardCandidates: string[];
  message: string | null;
}

export interface LinuxInputAccessFixResult {
  success: boolean;
  message: string;
  status: LinuxInputAccessStatus;
}

// Initial state returned by get_initial_state (unified startup command)
export interface InitialState {
  config: AppConfig;
  profiles: { id: string; name: string; createdAt: string; updatedAt: string }[];
  currentProfile: Profile | null;
  inputRuntime: InputRuntime;
}

// Waveform data
export interface WaveformData {
  points: number[];
  duration: number;
  sampleRate: number;
  suggestedMomentum: number | null;
}

// YouTube search result
export interface YoutubeSearchResult {
  videoId: string;
  title: string;
  duration: number;
  channel: string;
  thumbnailUrl: string;
  url: string;
  alreadyDownloaded: boolean;
}

// YouTube playlist
export interface YoutubePlaylist {
  title: string;
  entries: YoutubeSearchResult[];
  totalCount: number;
}

// YouTube stream URL result (for preview playback)
export interface StreamUrlResult {
  url: string;
  duration: number;
  format: string;
}

// État "Now Playing" pour l'affichage
export interface NowPlayingState {
  trackId: TrackId;
  trackName: string;
  soundName: string;
  currentTime: number;
  duration: number;
  isPlaying: boolean;
}

// Filtre pour le KeyGrid (recherche/filtre Spotlight-style)
export interface KeyGridFilter {
  searchText: string;
  trackName: string | null;    // partial match, case-insensitive
  loopMode: LoopMode | null;
  status: "playing" | "stopped" | null;
  mood: BaseMood | null;        // filter by mood tag
  intensity: MoodIntensity | null; // filter by intensity
}

// Événements émis par le backend vers le frontend
export type BackendEvent =
  | { type: "sound_started"; trackId: TrackId; soundId: SoundId }
  | { type: "sound_ended"; trackId: TrackId; soundId: SoundId }
  | { type: "playback_progress"; trackId: TrackId; position: number }
  | { type: "key_pressed"; keyCode: KeyCode }
  | { type: "download_progress"; url: string; progress: number }
  | { type: "download_complete"; url: string; cachedPath: string }
  | { type: "download_error"; url: string; error: string }
  | { type: "sound_not_found"; soundId: SoundId; path: string };
