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
}

// Configuration d'une piste
export interface Track {
  id: TrackId;
  name: string;
  volume: number;             // 0.0 à 1.0 (volume de la piste)
  currentlyPlaying: SoundId | null;
  playbackPosition: number;   // Position actuelle de lecture en secondes
  isPlaying: boolean;
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
  masterStopShortcut: KeyCode[];  // Combinaison de touches (ex: ["ControlLeft", "ShiftLeft", "KeyS"])
  autoMomentumShortcut: KeyCode[];  // Shortcut pour toggle auto-momentum
  keyDetectionShortcut: KeyCode[];  // Shortcut pour toggle key detection (fonctionne même si désactivé)
  crossfadeDuration: number;      // Durée du crossfade en millisecondes (défaut: 500)
  keyCooldown: number;            // Cooldown global entre pressions en millisecondes (défaut: 1500)
  currentProfileId: ProfileId | null;
  audioDevice: string | null;     // null = follow system default, string = force specific device
  chordWindowMs: number;          // Multi-key chord detection window in ms (default: 30, range: 20-100)
  momentumModifier: MomentumModifier; // Modifier key to trigger momentum (default: "Shift")
  playlistImportEnabled: boolean;     // Remember "download entire playlist" checkbox state
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
