# KeyToMusic - Liste Exhaustive des Tâches

> Document généré depuis la spécification technique complète
> Dernière mise à jour: 2025-01-25
> **Phase 0 COMPLÉTÉE** - 2026-01-20
> **Phase 1 COMPLÉTÉE** - 2026-01-23
> **Phase 2 COMPLÉTÉE** - 2026-01-23
> **Phase 3 COMPLÉTÉE** - 2026-01-23
> **Phase 4 COMPLÉTÉE** - 2026-01-23
> **Phase 4.5 COMPLÉTÉE** - 2026-01-23 (Bug Fixes & Améliorations)
> **Phase 4.6 COMPLÉTÉE** - 2026-01-23 (UX Enhancements & Key Management)
> **Phase 5 COMPLÉTÉE** - 2026-01-24 (YouTube: ffmpeg, retry, M4A, canonical URLs, cache cleanup)
> **Phase 6 COMPLÉTÉE** - 2026-01-24 (Import/Export: .ktm ZIP format, rfd file dialogs, frontend UI)
> **Phase 6.5 COMPLÉTÉE** - 2026-01-24 (Concurrent YouTube Downloads & Key Cycling)
> **Phase 7 COMPLÉTÉE** - 2026-01-24 (Error Handling: logging, error sound, FileNotFoundModal, verification, toasts)
> **Phase 7.5 COMPLÉTÉE** - 2026-01-24 (Legacy Import: conversion des saves de l'ancienne version)
> **Phase 8 COMPLÉTÉE** - 2026-01-25 (Profile Duplication, Combined Shortcuts, Undo/Redo, KeyGrid display)

---

## Table des Matières

1. [Phase 0 - Initialisation du Projet](#phase-0---initialisation-du-projet)
2. [Phase 1 - Fondations Backend (Rust)](#phase-1---fondations-backend-rust)
3. [Phase 2 - Moteur Audio](#phase-2---moteur-audio)
4. [Phase 3 - Système de Détection des Touches](#phase-3---système-de-détection-des-touches)
5. [Phase 4 - Interface Utilisateur (React)](#phase-4---interface-utilisateur-react)
6. [Phase 4.5 - Bug Fixes & Améliorations](#phase-45---bug-fixes--améliorations)
7. [Phase 4.6 - UX Enhancements & Key Management](#phase-46---ux-enhancements--key-management)
8. [Phase 5 - Téléchargement YouTube](#phase-5---téléchargement-youtube)
9. [Phase 6 - Import/Export](#phase-6---importexport)
10. [Phase 6.5 - Concurrent YouTube Downloads & Key Cycling](#phase-65---concurrent-youtube-downloads--key-cycling)
11. [Phase 7 - Gestion des Erreurs](#phase-7---gestion-des-erreurs)
12. [Phase 7.5 - Legacy Import](#phase-75---legacy-import)
13. [Phase 8 - Nouvelles Features](#phase-8---nouvelles-features) 🔄
    - [8.1 Duplication de Profil](#81-duplication-de-profil-) ✅
    - [8.2 Raccourcis Combinés](#82-raccourcis-clavier-combinés-modificateurs--partiellement-complété) 🔄
    - [8.3 Undo/Redo](#83-système-undoredo-) ✅
    - [8.4 Multi-Key Chords](#84-multi-key-chords--complete) ✅
    - [8.5 Momentum Modifier](#85-modificateur-momentum-configurable--en-discussion) ⏳
14. [Phase 9 - Polish & Optimisations](#phase-9---polish--optimisations)
15. [Phase 10 - Tests & Validation](#phase-10---tests--validation)
16. [Phase 11 - Build & Release](#phase-11---build--release-bonus)

---

## Phase 0 - Initialisation du Projet ✅ COMPLÉTÉE

### 0.1 Configuration Initiale

- [x] **0.1.1** Créer le projet Tauri avec template React + TypeScript
  ```bash
  npm create tauri-app@latest keytomusic -- --template react-ts
  ```
  **✅ Complété** - Structure créée manuellement avec tous les fichiers de configuration

- [x] **0.1.2** Installer les dépendances npm
  - [x] Installer `zustand` pour le state management
  - [x] Installer `tailwindcss`, `postcss`, `autoprefixer`
  - [x] Initialiser Tailwind CSS (`npx tailwindcss init -p`)
  **✅ Complété** - Toutes les dépendances configurées dans package.json

- [x] **0.1.3** Configurer Tailwind CSS
  - [x] Créer `tailwind.config.js` avec le content path
  - [x] Ajouter les directives Tailwind dans `src/index.css`
  - [x] Définir les variables CSS du design system (couleurs, fonts)
  **✅ Complété** - Tailwind configuré avec le design system complet

- [x] **0.1.4** Configurer les dépendances Rust dans `src-tauri/Cargo.toml`
  - [x] Ajouter `tauri = { version = "2", features = ["shell-open"] }`
  - [x] Ajouter `serde = { version = "1", features = ["derive"] }`
  - [x] Ajouter `serde_json = "1"`
  - [x] Ajouter `rodio = "0.19"`
  - [x] Ajouter `symphonia = { version = "0.5", features = ["mp3", "flac", "ogg", "wav", "pcm", "aac"] }`
  - [x] Ajouter `rdev = "0.5"`
  - [x] Ajouter `tokio = { version = "1", features = ["full"] }`
  - [x] Ajouter `uuid = { version = "1", features = ["v4"] }`
  - [x] Ajouter `walkdir = "2"`
  - [x] Ajouter `sanitize-filename = "0.5"`
  - [x] Ajouter `thiserror = "1"`
  - [x] Ajouter `chrono = { version = "0.4", features = ["serde"] }`
  **✅ Complété** - Cargo.toml créé avec toutes les dépendances

### 0.2 Structure des Dossiers

- [x] **0.2.1** Créer la structure frontend
  ```bash
  mkdir -p src/components/{Layout,Tracks,Sounds,Keys,Controls,Profiles,Settings}
  mkdir -p src/stores
  mkdir -p src/hooks
  mkdir -p src/types
  mkdir -p src/utils
  ```
  **✅ Complété** - Tous les dossiers frontend créés

- [x] **0.2.2** Créer la structure backend
  ```bash
  mkdir -p src-tauri/src/audio
  mkdir -p src-tauri/src/keys
  mkdir -p src-tauri/src/youtube
  mkdir -p src-tauri/src/storage
  ```
  **✅ Complété** - Tous les dossiers backend créés avec fichiers mod.rs

- [x] **0.2.3** Créer la structure des ressources
  ```bash
  mkdir -p resources/sounds
  mkdir -p resources/icons
  ```
  **✅ Complété** - Dossiers resources créés avec README.md explicatifs

- [x] **0.2.4** Ajouter les ressources de base
  - [x] Ajouter le son d'erreur système (`resources/sounds/error.mp3`)
  - [x] Créer les icônes de l'application (32x32, 128x128, icns, ico)
  **⚠️ Partiellement complété** - Dossiers créés avec instructions, fichiers à ajouter manuellement

### 0.3 Configuration Tauri

- [x] **0.3.1** Configurer `src-tauri/tauri.conf.json`
  - [x] Définir `productName` comme "KeyToMusic"
  - [x] Définir `version` à "1.0.0"
  - [x] Configurer `identifier` comme "com.keytomusic.app"
  - [x] Configurer la fenêtre principale (1200x800, min: 800x600)
  - [x] Activer les permissions nécessaires (fs, dialog, path, globalShortcut)
  - [x] Configurer le system tray
  - [x] Définir les chemins d'icônes
  **✅ Complété** - tauri.conf.json créé avec toute la configuration

- [x] **0.3.2** Configurer les scripts package.json
  - [x] Vérifier script `dev`
  - [x] Vérifier script `build`
  - [x] Vérifier script `tauri dev`
  - [x] Vérifier script `tauri build`
  **✅ Complété** - Tous les scripts npm configurés

---

## Phase 1 - Fondations Backend (Rust) ✅ COMPLÉTÉE

### 1.1 Types et Structures de Données

- [x] **1.1.1** Créer `src-tauri/src/types.rs`
  - [x] Définir les type aliases (`SoundId`, `TrackId`, `ProfileId`, `KeyCode`)
  - [x] Définir l'enum `SoundSource` (Local, YouTube)
  - [x] Définir l'enum `LoopMode` (Off, Random, Single, Sequential)
  - [x] Définir la struct `Sound` avec tous les champs
  - [x] Définir la struct `KeyBinding`
  - [x] Définir la struct `Track`
  - [x] Définir la struct `Profile`
  - [x] Définir la struct `AppConfig` avec impl Default
  - [x] Ajouter les annotations serde appropriées (rename_all = "camelCase")
  **✅ Complété** - Tous les types définis avec serde annotations

- [x] **1.1.2** Créer `src-tauri/src/errors.rs`
  - [x] Définir l'enum `AppError` avec thiserror
  - [x] Ajouter les variantes pour erreurs audio (SoundFileNotFound, UnsupportedFormat, PlaybackFailed)
  - [x] Ajouter les variantes pour erreurs YouTube (InvalidYouTubeUrl, YouTubeDownloadFailed, YtDlpNotFound)
  - [x] Ajouter les variantes pour erreurs storage (ProfileNotFound, SaveFailed, LoadFailed)
  - [x] Ajouter les variantes pour erreurs import/export (InvalidExportFile, ExportFailed)
  - [x] Ajouter les variantes pour erreurs keys (KeyAlreadyAssigned, InvalidMasterStopShortcut)
  - [x] Implémenter les messages d'erreur avec #[error("...")]
  **✅ Complété** - AppError enum avec From<AppError> pour String et Serialize impl

### 1.2 Système de Stockage

- [x] **1.2.1** Créer `src-tauri/src/storage/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter les sous-modules (config, profile)
  **✅ Complété**

- [x] **1.2.2** Créer `src-tauri/src/storage/config.rs`
  - [x] Implémenter `get_app_data_dir()` multi-plateforme (Windows, macOS, Linux)
  - [x] Implémenter `init_app_directories()` pour créer data/, profiles/, cache/, logs/
  - [x] Implémenter `load_config()` depuis config.json
  - [x] Implémenter `save_config()` vers config.json
  - [x] Implémenter `get_default_config()` utilisant AppConfig::default()
  - [x] Gérer les erreurs de lecture/écriture JSON
  **✅ Complété** - Utilise la crate `dirs` pour les chemins multi-plateforme

- [x] **1.2.3** Créer `src-tauri/src/storage/profile.rs`
  - [x] Implémenter `list_profiles()` retournant Vec<ProfileSummary>
  - [x] Implémenter `create_profile(name: String)` générant un UUID
  - [x] Implémenter `load_profile(id: String)` depuis profiles/{id}.json
  - [x] Implémenter `save_profile(profile: Profile)` vers profiles/{id}.json
  - [x] Implémenter `delete_profile(id: String)` avec suppression du fichier
  - [x] Implémenter `profile_exists(id: String)` pour vérification
  - [x] Gérer les timestamps (createdAt, updatedAt)
  - [x] Gérer les erreurs de sérialisation/désérialisation
  **✅ Complété** - CRUD complet avec ProfileSummary, timestamps via chrono

### 1.3 Commandes Tauri de Base

- [x] **1.3.1** Créer `src-tauri/src/commands.rs` (partie config)
  - [x] Implémenter `get_config() -> Result<AppConfig, String>`
  - [x] Implémenter `update_config(updates: serde_json::Value) -> Result<(), String>`
  - [x] Implémenter la fusion partielle des updates avec la config existante
  **✅ Complété** - Merge partiel de tous les champs dont masterStopShortcut et currentProfileId (nullable)

- [x] **1.3.2** Créer les commandes de profils dans `commands.rs`
  - [x] Implémenter `list_profiles() -> Result<Vec<ProfileSummary>, String>`
  - [x] Implémenter `create_profile(name: String) -> Result<Profile, String>`
  - [x] Implémenter `load_profile(id: String) -> Result<Profile, String>`
  - [x] Implémenter `save_profile(profile: Profile) -> Result<(), String>`
  - [x] Implémenter `delete_profile(id: String) -> Result<(), String>`
  **✅ Complété**

- [x] **1.3.3** Enregistrer les commandes dans `src-tauri/src/main.rs`
  - [x] Importer les modules nécessaires
  - [x] Ajouter toutes les commandes dans `.invoke_handler()`
  - [x] Initialiser les répertoires de données au démarrage
  - [x] Charger la config au démarrage
  **✅ Complété** - Init avant le Builder, config chargée et passée à AppState

### 1.4 State Management Rust

- [x] **1.4.1** Créer `src-tauri/src/state.rs`
  - [x] Définir `AppState` avec Mutex<> pour config
  - [x] Implémenter des méthodes helper pour accéder au state de manière thread-safe
  - [x] Gérer l'initialisation du state dans main.rs
  **✅ Complété** - AppState avec get_config() et update_config() thread-safe, managé via Tauri .manage()

---

## Phase 2 - Moteur Audio ✅ COMPLÉTÉE

### 2.1 Structures Audio de Base

- [x] **2.1.1** Créer `src-tauri/src/audio/mod.rs`
  - [x] Définir la structure du module audio
  - [x] Exporter engine, track, crossfade, buffer, symphonia_source
  **✅ Complété** - Module avec exports de tous les sous-modules

- [x] **2.1.2** Types audio intégrés dans les modules respectifs
  - [x] AudioCommand et AudioEvent dans engine.rs
  - [x] CrossfadeState dans crossfade.rs
  - [x] AudioMetadata et BufferedSound dans buffer.rs
  **✅ Complété** - Types définis directement dans leurs modules

### 2.2 Moteur Audio Principal

- [x] **2.2.1** Créer `src-tauri/src/audio/engine.rs` (structure de base)
  - [x] Définir AudioEngineHandle avec command channel
  - [x] Ajouter les champs: command_tx, events, master_volume, last_trigger_time
  - [x] Implémenter `new()` pour démarrer le thread audio
  - [x] Implémenter `set_master_volume(volume: f32)`
  **✅ Complété** - Architecture thread + channel avec AudioEngineHandle

- [x] **2.2.2** Implémenter la lecture audio basique
  - [x] Intégrer `rodio` avec création du `OutputStream` et `OutputStreamHandle`
  - [x] Implémenter `play_sound(track_id, sound_id, start_position)` basique
  - [x] Implémenter le décodage des formats (MP3, WAV, OGG, FLAC via rodio::Decoder)
  - [x] Implémenter le calcul du volume final (sound × track × master)
  - [x] Gérer les erreurs de lecture (fichier non trouvé, format non supporté)
  **✅ Complété** - Lecture via rodio avec volume hiérarchique

- [x] **2.2.3** Implémenter l'arrêt des sons
  - [x] Implémenter `stop_sound(track_id)` pour arrêter le son d'une piste
  - [x] Implémenter `stop_all_sounds()` pour arrêter tous les sons
  - [x] Nettoyer les ressources audio correctement
  **✅ Complété** - StopTrack et StopAll avec nettoyage des sinks

- [x] **2.2.4** Implémenter le threading audio
  - [x] Créer un thread séparé pour le moteur audio
  - [x] Utiliser des channels (std::sync::mpsc) pour la communication
  - [x] Définir les messages (PlaySound, StopSound, SetVolume, CreateTrack, RemoveTrack, Shutdown)
  - [x] Gérer le cycle de vie du thread audio (recv_timeout loop, shutdown)
  - [x] Timeout dynamique: 200ms quand idle, 16ms quand playback actif (réduit CPU)
  **✅ Complété** - Thread dédié avec boucle de commandes et timeout dynamique

### 2.3 Gestion des Pistes

- [x] **2.3.1** Créer `src-tauri/src/audio/track.rs`
  - [x] Définir la struct `AudioTrack` (différente de la struct Track du modèle)
  - [x] Ajouter les champs: id, volume, currently_playing, sink, outgoing_sink, crossfade
  - [x] Implémenter `new()` pour créer une piste
  - [x] Implémenter `play()` pour jouer un son sur la piste (SymphoniaSource pour start_position > 0)
  - [x] Implémenter `stop()` pour arrêter le son actuel
  - [x] Implémenter `set_volume()` pour ajuster le volume de la piste
  - [x] Implémenter `is_playing()` et `has_finished()` pour vérifier l'état
  **✅ Complété** - AudioTrack complet avec double-sink pour crossfade

- [x] **2.3.2** Gérer plusieurs pistes dans AudioEngine
  - [x] Implémenter la création de nouvelles pistes (auto-create on play + CreateTrack command)
  - [x] Implémenter la suppression de pistes (RemoveTrack command)
  - [x] Limiter à 20 pistes maximum
  - [x] Mixer les sorties via sinks indépendants sur le même OutputStreamHandle
  **✅ Complété** - HashMap<TrackId, AudioTrack> avec limite de 20

### 2.4 Système de Crossfade

- [x] **2.4.1** Créer `src-tauri/src/audio/crossfade.rs`
  - [x] Définir la struct `CrossfadeState`
  - [x] Définir les champs: outgoing_sound_id, incoming_sound_id, start_time, duration
  - [x] Implémenter `new()` pour initialiser un crossfade
  **✅ Complété**

- [x] **2.4.2** Implémenter la courbe de crossfade
  - [x] Implémenter `get_volumes() -> (f32, f32)`
  - [x] Calculer le progress (0.0 à 1.0)
  - [x] Appliquer la courbe custom (0-35%: fade-out à 30%, 35-65%: gap, 65-100%: fade-in depuis 30%)
  - [x] Retourner (outgoing_volume, incoming_volume)
  **✅ Complété** - Courbe exacte de la spec implémentée

- [x] **2.4.3** Intégrer le crossfade dans AudioEngine
  - [x] Modifier `play()` dans AudioTrack pour détecter si un son joue déjà
  - [x] Démarrer le crossfade si nécessaire (move sink to outgoing_sink)
  - [x] Utiliser deux sinks (rodio) simultanément pour le crossfade
  - [x] Appliquer les volumes calculés en temps réel (update_crossfade() dans la boucle)
  - [x] Nettoyer le son sortant après le crossfade (is_complete() check)
  - [x] Gérer la durée configurable du crossfade (passée via AudioCommand)
  **✅ Complété** - Crossfade temps réel avec double-sink

### 2.5 Système de Seeking et Streaming (Symphonia)

- [x] **2.5.1** Créer `src-tauri/src/audio/buffer.rs`
  - [x] Définir la struct `BufferManager`
  - [x] Définir la struct `BufferedSound` avec métadonnées audio
  - [x] Utiliser HashMap<SoundId, BufferedSound>
  **✅ Complété** - BufferManager avec register/unregister et metadata

- [x] **2.5.2** Implémenter le pré-chargement
  - [x] Implémenter `read_audio_metadata(path)` pour obtenir durée, sample_rate, channels
  - [x] Implémenter `get_audio_duration(path)` exposé via commande Tauri
  - [x] Implémenter `register_sound()` pour enregistrer un son avec ses métadonnées
  **✅ Complété** - Metadata via rodio::Decoder, registration system

- [x] **2.5.3** Streaming depuis le disque
  - [x] Utiliser rodio::Decoder avec BufReader<File> pour le streaming (position 0)
  - [x] Utiliser SymphoniaSource pour le seeking instantané byte-level (momentum > 0)
  - [x] Remplacé skip_duration (O(n) lent) par symphonia seek (O(1) instantané)
  **✅ Complété** - SymphoniaSource pour momentum, rodio Decoder pour position 0

- [x] **2.5.4** Intégration dans AudioEngine
  - [x] get_audio_duration disponible comme commande Tauri
  - [x] preload_profile_sounds calcule les durées en batch (2 threads parallèles)
  - [x] SymphoniaSource intégré dans AudioTrack.play() pour seeking instantané
  **✅ Complété**

### 2.6 Logique de Lecture Avancée

- [x] **2.6.1** Support de la sélection des sons selon Loop Mode
  - [x] start_position passé au moteur audio pour le momentum
  - [x] Logique de sélection prête à être utilisée par Phase 3 (key detection)
  **✅ Complété** - La sélection se fait côté frontend/key handler, le moteur reçoit le son choisi

- [x] **2.6.2** Implémenter le momentum
  - [x] start_position paramètre de play_sound()
  - [x] SymphoniaSource avec seeking byte-level pour start_position > 0
  - [x] Mini-player UI (seek bar + play button) pour tester le momentum
  - [x] Calcul batch des durées au chargement du profil (2 threads parallèles)
  **✅ Complété** - Seeking instantané via symphonia, mini-player dans SoundDetails

- [x] **2.6.3** Implémenter la gestion de fin de son
  - [x] Écouter les événements de fin de lecture (sink.empty() check dans la boucle)
  - [x] Émettre SoundEnded dans le vecteur d'events
  **✅ Complété** - Détection de fin via has_finished() + émission d'event

- [x] **2.6.4** Implémenter le cooldown global
  - [x] Ajouter `last_trigger_time: Arc<Mutex<Instant>>` dans AudioEngineHandle
  - [x] Implémenter check_cooldown() et update_trigger_time()
  **✅ Complété** - Cooldown prêt à être utilisé par Phase 3

### 2.7 Commandes Audio Tauri

- [x] **2.7.1** Ajouter les commandes audio dans `commands.rs`
  - [x] `play_sound(track_id, sound_id, file_path, start_position, sound_volume) -> Result<(), String>`
  - [x] `stop_sound(track_id) -> Result<(), String>`
  - [x] `stop_all_sounds() -> Result<(), String>`
  - [x] `set_master_volume(volume: f32) -> Result<(), String>`
  - [x] `set_track_volume(track_id, volume: f32) -> Result<(), String>`
  - [x] `get_audio_duration(path: String) -> Result<f64, String>`
  **✅ Complété** - 6 commandes audio avec vérification de fichier

- [x] **2.7.2** Enregistrer les commandes audio dans main.rs
  **✅ Complété** - Toutes les commandes dans generate_handler![]

### 2.8 Events Audio

- [x] **2.8.1** Implémenter l'émission d'events audio
  - [x] Émettre SoundStarted avec {track_id, sound_id}
  - [x] Émettre SoundEnded avec {track_id, sound_id}
  - [x] Émettre PlaybackProgress avec {track_id, position} (toutes les 100ms)
  - [x] Émettre Error avec {message}
  **✅ Complété** - Events stockés dans Arc<Mutex<Vec>> pour drain par le frontend

- [x] **2.8.2** Implémenter le système de progression
  - [x] Timer de 100ms dans la boucle audio (last_progress_emit)
  - [x] Calculer la position actuelle via get_position() (start_time elapsed + start_position)
  - [x] Émettre régulièrement les updates de position
  **✅ Complété** - Progress émis toutes les 100ms pour les pistes actives

---

## Phase 3 - Système de Détection des Touches ✅ COMPLÉTÉE

### 3.1 Détecteur de Touches

- [x] **3.1.1** Créer `src-tauri/src/keys/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter detector et mapping
  **✅ Complété** - Module avec exports de KeyDetector et KeyEvent

- [x] **3.1.2** Créer `src-tauri/src/keys/detector.rs` (structure)
  - [x] Définir la struct `KeyDetector`
  - [x] Ajouter les champs: enabled, last_key_time, cooldown, pressed_keys
  - [x] Ajouter le champ `master_stop_shortcut: Vec<String>`
  - [x] Utiliser Arc<Mutex<>> pour le thread safety
  **✅ Complété** - KeyDetector Clone-able avec tous les champs Arc<Mutex<>>

- [x] **3.1.3** Implémenter la détection globale avec rdev
  - [x] Implémenter `new(cooldown_ms: u32) -> KeyDetector`
  - [x] Implémenter `start<F>(callback: F)` où F est le callback
  - [x] Créer un thread séparé pour rdev::listen()
  - [x] Capturer les événements KeyPress et KeyRelease
  **✅ Complété** - Thread dédié avec rdev::listen, callback Fn(KeyEvent)

- [x] **3.1.4** Implémenter la logique de détection
  - [x] Vérifier si la détection est enabled
  - [x] Gérer les touches pressées (HashSet pour éviter les répétitions)
  - [x] Vérifier le cooldown global avant de déclencher
  - [x] Mettre à jour last_key_time après déclenchement
  **✅ Complété** - Logique complète avec tracking des releases même quand disabled

- [x] **3.1.5** Implémenter la détection du Master Stop
  - [x] Implémenter `is_shortcut_pressed(pressed_keys, shortcut_keys) -> bool`
  - [x] Vérifier si toutes les touches du shortcut sont pressées
  - [x] Émettre un event spécial MasterStop
  - [x] Bloquer les autres événements de touches pendant le Master Stop
  **✅ Complété** - Master Stop vérifié avant le cooldown, bloque les events normaux

- [x] **3.1.6** Détecter les modificateurs
  - [x] Détecter si Shift (Left ou Right) est pressé
  - [x] Passer l'info `with_shift` dans le callback
  - [x] Utiliser with_shift pour activer le momentum
  **✅ Complété** - Shift détecté via pressed_keys HashSet, modifier-only presses ignorées

### 3.2 Mapping des Touches

- [x] **3.2.1** Créer `src-tauri/src/keys/mapping.rs`
  - [x] Définir l'enum `KeyEvent` (KeyPressed, MasterStop)
  - [x] Implémenter `key_to_code(key: rdev::Key) -> String`
  - [x] Mapper toutes les lettres (A-Z → KeyA-KeyZ)
  - [x] Mapper tous les chiffres (0-9 → Digit0-Digit9)
  - [x] Mapper le pavé numérique (Numpad0-Numpad9 + operators)
  - [x] Mapper les touches fonction (F1-F12)
  - [x] Mapper les flèches (ArrowUp, ArrowDown, ArrowLeft, ArrowRight)
  - [x] Mapper les touches spéciales (Space, Enter, Tab, Escape, Backspace, etc.)
  - [x] Mapper les modificateurs (ShiftLeft/Right, ControlLeft/Right, AltLeft/Right, Meta)
  - [x] Mapper la ponctuation (Semicolon, Comma, Period, Slash, etc.)
  **✅ Complété** - Mapping complet compatible Web KeyboardEvent.code format

- [x] **3.2.2** Implémenter la conversion inverse
  - [x] Implémenter `code_to_key(code: &str) -> Option<rdev::Key>`
  - [x] Gérer tous les codes mappés
  **✅ Complété** - Conversion bidirectionnelle complète + helper is_modifier()

### 3.3 Intégration avec AudioEngine

- [x] **3.3.1** Créer le handler de touches dans AudioEngine
  - [x] Le callback dans main.rs setup() gère les key events
  - [x] Le cooldown est géré directement dans KeyDetector
  - [x] Les events sont émis vers le frontend qui gère la logique de binding/son
  **✅ Complété** - Architecture: KeyDetector → Tauri events → Frontend gère les bindings

- [x] **3.3.2** Gérer le Master Stop
  - [x] Implémenter `handle_master_stop()` dans le callback setup()
  - [x] Arrêter tous les sons de toutes les pistes via audio_engine.stop_all()
  - [x] Émettre un event `master_stop_triggered`
  **✅ Complété** - Master Stop arrête l'audio et émet l'event

### 3.4 Commandes et State

- [x] **3.4.1** Intégrer KeyDetector dans AppState
  - [x] Ajouter un champ `key_detector: KeyDetector` dans AppState (Clone-able)
  - [x] Initialiser le KeyDetector au démarrage avec config
  - [x] Passer le callback qui émet des events Tauri (via setup())
  **✅ Complété** - KeyDetector dans AppState, callback Tauri dans Builder.setup()

- [x] **3.4.2** Créer les commandes de touches dans `commands.rs`
  - [x] `set_key_detection(enabled: bool) -> Result<(), String>`
  - [x] `set_master_stop_shortcut(keys: Vec<String>) -> Result<(), String>`
  - [x] `set_key_cooldown(cooldown_ms: u32) -> Result<(), String>` (bonus)
  **✅ Complété** - 3 commandes avec validation et sync config+detector

- [x] **3.4.3** Enregistrer les commandes dans main.rs
  **✅ Complété** - Toutes les commandes dans generate_handler![]

### 3.5 Events de Touches

- [x] **3.5.1** Émettre les events de touches
  - [x] Émettre `key_pressed` avec {keyCode, withShift}
  - [x] Émettre `master_stop_triggered`
  **✅ Complété** - Events émis via tauri::Emitter dans le callback

---

## Phase 4 - Interface Utilisateur (React) ✅ COMPLÉTÉE

### 4.1 Types TypeScript

- [x] **4.1.1** Créer `src/types/index.ts`
  - [x] Définir les type aliases (SoundId, TrackId, ProfileId, KeyCode)
  - [x] Définir le type `SoundSource` (union type)
  - [x] Définir le type `LoopMode` (union type)
  - [x] Définir l'interface `Sound`
  - [x] Définir l'interface `KeyBinding`
  - [x] Définir l'interface `Track`
  - [x] Définir l'interface `Profile`
  - [x] Définir l'interface `AppConfig`
  - [x] Définir l'interface `NowPlayingState`
  - [x] Définir le type `BackendEvent` (union type pour tous les events)

### 4.2 Design System

- [x] **4.2.1** Configurer Tailwind avec les couleurs custom
  - [x] Éditer `tailwind.config.js` pour ajouter les couleurs du design system
  - [x] Définir bg-primary, bg-secondary, bg-tertiary, bg-hover
  - [x] Définir text-primary, text-secondary, text-muted
  - [x] Définir accent-primary, accent-secondary, success, warning, error
  - [x] Définir border-color, border-focus

- [x] **4.2.2** Créer les variables CSS dans `src/index.css`
  - [x] Définir toutes les couleurs en variables CSS
  - [x] Définir les tailles de police
  - [x] Définir la font-family
  - [x] Ajouter les styles de base pour le dark theme

### 4.3 Stores Zustand

- [x] **4.3.1** Créer `src/stores/audioStore.ts`
  - [x] Définir l'interface du state (tracks, sounds, nowPlaying)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setTracks, addTrack, removeTrack, updateTrack
  - [x] Implémenter les actions: setSounds, addSound, removeSound, updateSound
  - [x] Implémenter les actions: setNowPlaying, clearNowPlaying

- [x] **4.3.2** Créer `src/stores/profileStore.ts`
  - [x] Définir l'interface du state (profiles, currentProfile, keyBindings)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setProfiles, addProfile, removeProfile
  - [x] Implémenter les actions: setCurrentProfile, loadProfile
  - [x] Implémenter les actions: setKeyBindings, addKeyBinding, updateKeyBinding, removeKeyBinding

- [x] **4.3.3** Créer `src/stores/settingsStore.ts`
  - [x] Définir l'interface du state (config: AppConfig)
  - [x] Créer le store avec zustand
  - [x] Implémenter les actions: setConfig, updateConfig
  - [x] Implémenter les actions spécifiques: setMasterVolume, toggleAutoMomentum, toggleKeyDetection
  - [x] Implémenter: setMasterStopShortcut, setCrossfadeDuration, setKeyCooldown

### 4.4 Hooks Custom

- [x] **4.4.1** Créer `src/hooks/useKeyDetection.ts`
  - [x] Écouter l'event `key_pressed` via Tauri
  - [x] Mettre à jour le state quand une touche est détectée
  - [x] Déclencher les actions appropriées (highlight UI, etc.)
  - [x] Cleanup des listeners au unmount

- [x] **4.4.2** Créer `src/hooks/useAudioEngine.ts`
  - [x] Écouter les events `sound_started`, `sound_ended`, `playback_progress`
  - [x] Mettre à jour le state audioStore avec les infos de lecture
  - [x] Gérer le nowPlaying state
  - [x] Cleanup des listeners

- [x] **4.4.3** Créer `src/hooks/useTauriCommand.ts`
  - [x] Créer un hook générique pour invoquer des commandes Tauri
  - [x] Gérer le loading state
  - [x] Gérer les erreurs
  - [x] Retourner {execute, loading, error, data}

- [x] **4.4.4** Créer `src/hooks/useTextInputFocus.ts`
  - [x] Détecter le focus sur les input/textarea
  - [x] Désactiver la détection des touches via `invoke('set_key_detection', {enabled: false})`
  - [x] Réactiver la détection au blur
  - [x] Utiliser focusin/focusout events

### 4.5 Utils

- [x] **4.5.1** Créer `src/utils/fileHelpers.ts`
  - [x] Implémenter `formatDuration(seconds: number) -> string` (MM:SS)
  - [x] Implémenter `formatFileSize(bytes: number) -> string` (KB, MB, GB)
  - [x] Implémenter `getFileExtension(path: string) -> string`
  - [x] Implémenter `isAudioFile(path: string) -> boolean`

- [x] **4.5.2** Créer `src/utils/keyMapping.ts`
  - [x] Implémenter `keyCodeToDisplay(code: string) -> string` (KeyA → "A")
  - [x] Implémenter `parseKeyCombination(keys: string) -> string[]` ("adgk" → ["KeyA", "KeyD", "KeyG", "KeyK"])
  - [x] Implémenter `isValidKeyCode(code: string) -> boolean`

### 4.6 Layout Components

- [x] **4.6.1** Créer `src/components/Layout/Header.tsx`
  - [x] Props: masterVolume, onVolumeChange, onSettingsClick, onMinimize, onClose
  - [x] Afficher le logo et le nom "KeyToMusic"
  - [x] Créer le slider de volume master (horizontal, toujours visible)
  - [x] Créer le bouton paramètres (icône gear)
  - [x] Créer les boutons de fenêtre (minimize, close)
  - [x] Appeler les handlers appropriés
  - [x] Styling avec Tailwind (dark theme)

- [x] **4.6.2** Créer `src/components/Layout/Sidebar.tsx`
  - [x] Diviser en trois sections: Profiles, Controls, NowPlaying
  - [x] Créer la structure de layout (vertical flex)
  - [x] Styling avec Tailwind

- [x] **4.6.3** Créer `src/components/Layout/MainContent.tsx`
  - [x] Créer la zone principale pour afficher le contenu
  - [x] Diviser en deux sections: Track View (haut) et Sound Details (bas)
  - [x] Gérer le responsive layout
  - [x] Styling avec Tailwind

- [x] **4.6.4** Créer `src/App.tsx` avec le layout complet
  - [x] Importer tous les composants de layout
  - [x] Assembler Header + Sidebar + MainContent
  - [x] Utiliser les stores pour passer les props
  - [x] Gérer le routing/state de l'app si nécessaire
  - [x] Appliquer les styles globaux (min-width: 800px, min-height: 600px)

### 4.7 Profile Components

- [x] **4.7.1** Créer `src/components/Profiles/ProfileSelector.tsx`
  - [x] Props: profiles, currentProfileId, onSelect
  - [x] Afficher la liste des profils
  - [x] Highlight le profil actif
  - [x] Gérer le click pour sélectionner
  - [x] Styling avec Tailwind (liste verticale avec hover)

- [x] **4.7.2** Créer `src/components/Profiles/ProfileManager.tsx`
  - [x] Props: profiles, onProfileCreate, onProfileDelete, onProfileRename, onProfileExport, onProfileImport
  - [x] Bouton "+ New Profile" qui ouvre un modal
  - [x] Menu contextuel (click droit) pour rename/delete/export
  - [x] Appeler les commandes Tauri appropriées
  - [x] Gérer les confirmations (modal de confirmation pour delete)

- [x] **4.7.3** Créer le modal de création de profil
  - [x] Input pour le nom du profil
  - [x] Validation (nom non vide, max 50 caractères)
  - [x] Boutons Cancel/Create
  - [x] Appeler `invoke('create_profile', {name})`
  - [x] Fermer le modal et mettre à jour le state

- [x] **4.7.4** Créer le modal de rename de profil
  - [x] Input pré-rempli avec le nom actuel
  - [x] Validation
  - [x] Boutons Cancel/Save
  - [x] Appeler `invoke('save_profile')` avec le profil modifié

### 4.8 Controls Components

- [x] **4.8.1** Créer `src/components/Controls/MasterVolume.tsx`
  - [x] Props: volume, onChange
  - [x] Slider vertical ou horizontal
  - [x] Afficher le % à côté
  - [x] Appeler onChange qui invoque `invoke('set_master_volume')`
  - [x] Styling avec accent colors

- [x] **4.8.2** Créer `src/components/Controls/GlobalToggles.tsx`
  - [x] Props: autoMomentum, keyDetectionEnabled, onToggleAutoMomentum, onToggleKeyDetection
  - [x] Toggle switch pour Auto-Momentum avec label
  - [x] Toggle switch pour Key Detection avec label
  - [x] Indicateur visuel ON/OFF (couleur, icône)
  - [x] Appeler les handlers qui invoquent `update_config`

- [x] **4.8.3** Créer `src/components/Controls/MasterStopButton.tsx`
  - [x] Props: onClick
  - [x] Gros bouton rouge "Master Stop"
  - [x] Icône stop
  - [x] Au click: invoke('stop_all_sounds')
  - [x] Animation/feedback visuel au click

- [x] **4.8.4** Assembler dans Sidebar
  - [x] Créer une section "Controls" dans Sidebar
  - [x] Inclure GlobalToggles et MasterStopButton
  - [x] Organiser verticalement avec espacement

### 4.9 Now Playing Component

- [x] **4.9.1** Créer `src/components/Controls/NowPlaying.tsx`
  - [x] Props: nowPlayingState (peut être null)
  - [x] Afficher le nom de la piste
  - [x] Afficher le nom du son en cours
  - [x] Afficher la barre de progression (visual progress bar)
  - [x] Afficher currentTime / duration (formaté en MM:SS)
  - [x] Gérer le cas où rien ne joue (afficher un message ou vide)
  - [x] Styling compact pour la sidebar

- [x] **4.9.2** Connecter au store
  - [x] Lire le nowPlaying state depuis audioStore
  - [x] Mettre à jour en temps réel avec les events playback_progress

### 4.10 Track Components

- [x] **4.10.1** Créer `src/components/Tracks/TrackList.tsx`
  - [x] Props: tracks, selectedTrackId, onSelectTrack
  - [x] Afficher un dropdown pour sélectionner la piste
  - [x] Lister les tracks avec leur nom
  - [x] Highlight la piste sélectionnée
  - [x] Option "+ New Track" dans le dropdown

- [x] **4.10.2** Créer `src/components/Tracks/TrackItem.tsx`
  - [x] Props: track, isSelected, onSelect, onVolumeChange, onRename, onDelete
  - [x] Afficher le nom de la piste
  - [x] Slider de volume de la piste
  - [x] Icône/bouton pour rename
  - [x] Icône/bouton pour delete
  - [x] Styling avec hover states

- [x] **4.10.3** Créer `src/components/Tracks/TrackVolumeSlider.tsx`
  - [x] Props: volume, onChange
  - [x] Slider horizontal
  - [x] Afficher le % ou valeur
  - [x] Appeler onChange qui invoque `set_track_volume`

- [x] **4.10.4** Gérer la création de tracks
  - [x] Modal pour créer une nouvelle piste
  - [x] Input pour le nom
  - [x] Validation (nom non vide, limite de 20 pistes)
  - [x] Générer un UUID pour la nouvelle piste
  - [x] Ajouter au profil et sauvegarder

- [x] **4.10.5** Gérer la suppression de tracks
  - [x] Modal de confirmation
  - [x] Vérifier si des sons sont assignés à cette piste
  - [x] Avertir l'utilisateur si des sons seront affectés
  - [x] Retirer du profil et sauvegarder

### 4.11 Key Assignment Components

- [x] **4.11.1** Créer `src/components/Keys/KeyGrid.tsx`
  - [x] Props: keyBindings, sounds, selectedKey, onKeySelect
  - [x] Afficher une grille de cartes (flexbox grid)
  - [x] Chaque carte représente un key binding
  - [x] Afficher: [Touche] + nom du premier son + "(+N)" si plusieurs sons
  - [x] Highlight la carte si le son est en cours de lecture (depuis nowPlaying)
  - [x] Highlight la carte si elle est sélectionnée
  - [x] Au click sur une carte: onKeySelect(keyCode)
  - [x] Styling avec Tailwind (cards avec border, hover, active states)

- [x] **4.11.2** Créer `src/components/Keys/KeyItem.tsx`
  - [x] Props: keyBinding, sound, isPlaying, isSelected, onClick
  - [x] Afficher le key code (formaté: "KeyA" → "A")
  - [x] Afficher le nom du son
  - [x] Indicateur "(+N)" si plusieurs sons
  - [x] Indicateur de lecture (icône animée si isPlaying)
  - [x] Styling avec différentes couleurs pour différents états

- [x] **4.11.3** Créer le bouton "+ Add Sound"
  - [x] Bouton en bas de la grille
  - [x] Au click: ouvrir le modal d'ajout de son
  - [x] Styling avec accent color

### 4.12 Sound Detail Components

- [x] **4.12.1** Créer `src/components/Sounds/SoundList.tsx`
  - [x] Props: sounds (sons assignés à la touche sélectionnée)
  - [x] Afficher une liste des sons (vertical)
  - [x] Pour chaque son: utiliser SoundItem component
  - [x] Message si aucun son ("No sounds assigned")

- [x] **4.12.2** Créer `src/components/Sounds/SoundItem.tsx`
  - [x] Props: sound, onEdit, onRemove
  - [x] Afficher le nom du son
  - [x] Afficher le momentum (formaté en MM:SS)
  - [x] Afficher le volume individuel (%)
  - [x] Afficher la durée totale (formaté)
  - [x] Afficher la source (Local ou YouTube avec icône)
  - [x] Bouton "Edit" qui ouvre le modal de settings
  - [x] Bouton "Remove" qui demande confirmation
  - [x] Styling en card avec hover

- [x] **4.12.3** Créer `src/components/Sounds/SoundSettings.tsx` (modal)
  - [x] Props: sound, onSave, onCancel
  - [x] Input pour le nom
  - [x] Input pour le momentum (number, décimales autorisées)
  - [x] Slider pour le volume individuel
  - [x] Preview du fichier (si possible)
  - [x] Boutons Cancel/Save
  - [x] Validation
  - [x] Appeler onSave qui invoque `update_sound`

- [x] **4.12.4** Créer le sélecteur de Loop Mode
  - [x] Props: loopMode, onChange
  - [x] Dropdown ou boutons radio
  - [x] Options: Off, Random, Single, Sequential
  - [x] Descriptions pour chaque mode
  - [x] Appeler onChange qui met à jour le keyBinding dans le profil

- [x] **4.12.5** Assembler le panneau Sound Details
  - [x] Afficher seulement si une touche est sélectionnée
  - [x] Header: "Sounds for key [X]"
  - [x] SoundList
  - [x] Loop Mode selector
  - [x] Bouton "+ Add Sound to Key"

### 4.13 Add Sound Modal

- [x] **4.13.1** Créer `src/components/Sounds/AddSoundModal.tsx` (structure)
  - [x] Props: isOpen, onClose, existingTracks, onAdd
  - [x] Modal overlay avec backdrop
  - [x] Fermeture sur ESC ou click outside

- [x] **4.13.2** Étape 1: Choix de la source
  - [x] Boutons radio ou tabs: "Local File" / "YouTube URL"
  - [x] State pour tracker la source choisie

- [x] **4.13.3** Étape 2a: Si Local File
  - [x] Drag & Drop zone
  - [x] Highlight au drag over
  - [x] Bouton "Browse" qui ouvre le file picker
  - [x] Appeler `invoke('pick_audio_files')` ou utiliser input[type="file"]
  - [x] Validation du format (MP3, WAV, OGG, FLAC)
  - [x] Afficher les fichiers sélectionnés
  - [x] Support multi-fichiers

- [x] **4.13.4** Étape 2b: Si YouTube URL
  - [x] Input pour l'URL YouTube
  - [x] Validation de l'URL (format youtube.com/watch ou youtu.be)
  - [x] Bouton "Download"
  - [x] Appeler `invoke('add_sound_from_youtube', {url})`
  - [x] Écouter les events `download_progress`, `download_complete`, `download_error`
  - [x] Afficher une progress bar pendant le téléchargement
  - [x] Afficher le titre une fois téléchargé
  - [x] Gérer les erreurs (afficher message d'erreur)

- [x] **4.13.5** Étape 3: Configuration
  - [x] Input pour les touches à assigner (texte: "adgk")
  - [x] Parser les touches entrées (voir utils/keyMapping)
  - [x] Validation (touches valides, non déjà assignées au Master Stop)
  - [x] Dropdown pour choisir la piste (existantes + "New Track")
  - [x] Si "New Track": input pour le nom de la nouvelle piste
  - [x] Input number pour le momentum (secondes, décimales ok)
  - [x] Slider pour le volume individuel (0-100%)
  - [x] Dropdown pour le Loop Mode (défaut: "off")

- [x] **4.13.6** Étape 4: Ajout
  - [x] Bouton "Add" en bas du modal
  - [x] Validation de tous les champs
  - [x] Si plusieurs fichiers ET plusieurs touches: assignation cyclique
    - [x] Fichier 1 → Touche 1, Fichier 2 → Touche 2, etc.
    - [x] Si plus de fichiers que de touches: cycler les touches
  - [x] Créer les sons avec UUID
  - [x] Appeler `invoke('get_audio_duration', {path})` pour chaque son
  - [x] Créer les KeyBindings
  - [x] Créer la Track si nouvelle
  - [x] Mettre à jour le profil
  - [x] Appeler `invoke('save_profile', {profile})`
  - [x] Fermer le modal
  - [x] Afficher un toast de succès

### 4.14 Settings Modal

- [x] **4.14.1** Créer `src/components/Settings/SettingsModal.tsx`
  - [x] Props: isOpen, onClose, config, onConfigUpdate
  - [x] Modal overlay
  - [x] Titre "Settings"

- [x] **4.14.2** Section: Master Stop Shortcut
  - [x] Afficher la combinaison actuelle (formaté: "Ctrl+Shift+S")
  - [x] Bouton "Change"
  - [x] Au click: mode capture
  - [x] Afficher "Press the key combination..."
  - [x] Capturer les touches pressées (via event listeners sur window)
  - [x] Afficher les touches capturées en temps réel
  - [x] Bouton "Save" pour confirmer
  - [x] Validation (au moins 2 touches, combinaison valide)
  - [x] Appeler `invoke('set_master_stop_shortcut', {keys})`

- [x] **4.14.3** Section: Crossfade Duration
  - [x] Slider (100ms à 2000ms)
  - [x] Afficher la valeur actuelle en ms
  - [x] Appeler `update_config` au changement

- [x] **4.14.4** Section: Key Cooldown
  - [x] Slider (500ms à 5000ms)
  - [x] Afficher la valeur actuelle en ms
  - [x] Appeler `update_config` au changement

- [x] **4.14.5** Section: Import/Export
  - [x] Bouton "Export Current Profile"
  - [x] Au click: ouvrir save dialog via `invoke('pick_save_location')`
  - [x] Appeler `invoke('export_profile', {profileId, outputPath})`
  - [x] Afficher un toast de succès/erreur
  - [x] Bouton "Import Profile"
  - [x] Au click: ouvrir file picker (filtre .ktm)
  - [x] Appeler `invoke('import_profile', {ktmPath})`
  - [x] Afficher un toast et recharger la liste des profils

- [x] **4.14.6** Section: About
  - [x] Afficher le nom de l'app "KeyToMusic"
  - [x] Afficher la version (lire depuis package.json ou Tauri config)
  - [x] Liens vers la documentation ou GitHub
  - [x] Informations de licence si applicable

- [x] **4.14.7** Boutons du modal
  - [x] Bouton "Close" en bas
  - [x] Fermer sur ESC

### 4.15 Error Modals

- [x] **4.15.1** Créer `src/components/Modals/FileNotFoundModal.tsx`
  - [x] Props: soundName, expectedPath, onUpdatePath, onRemoveSound, onCancel
  - [x] Afficher le message: "Le fichier audio n'a pas été trouvé"
  - [x] Afficher le nom du son et le chemin attendu
  - [x] Bouton "Update Path"
    - [x] Ouvrir file picker
    - [x] Appeler onUpdatePath(newPath)
    - [x] Mettre à jour le sound dans le profil
    - [x] Sauvegarder
  - [x] Bouton "Remove Sound"
    - [x] Confirmation
    - [x] Appeler onRemoveSound()
    - [x] Retirer du profil et sauvegarder
  - [x] Bouton "Cancel"
    - [x] Fermer le modal sans action

- [x] **4.15.2** Créer `src/components/Modals/ErrorModal.tsx` (générique)
  - [x] Props: title, message, onClose
  - [x] Afficher le titre et message d'erreur
  - [x] Icône d'erreur
  - [x] Bouton "OK" pour fermer
  - [x] Styling avec couleur error

- [x] **4.15.3** Gérer l'affichage des modals d'erreur
  - [x] Écouter l'event `sound_not_found`
  - [x] Afficher FileNotFoundModal avec les infos du son
  - [x] Écouter l'event `download_error`
  - [x] Afficher ErrorModal avec le message

### 4.16 Notifications/Toasts

- [x] **4.16.1** Créer un système de notifications toast
  - [x] Créer `src/components/Toast/ToastContainer.tsx`
  - [x] Gérer un state de toasts (array)
  - [x] Afficher les toasts en overlay (coin haut-droit ou bas-droit)
  - [x] Auto-dismiss après 3-5 secondes
  - [x] Types: success, error, info, warning

- [x] **4.16.2** Créer `src/components/Toast/Toast.tsx`
  - [x] Props: type, message, onClose
  - [x] Icône selon le type
  - [x] Message
  - [x] Bouton close (X)
  - [x] Animation d'entrée/sortie
  - [x] Styling avec les couleurs appropriées

- [x] **4.16.3** Créer un hook `useToast`
  - [x] Exposer une fonction `showToast(message, type)`
  - [x] Ajouter le toast au state
  - [x] Gérer l'auto-dismiss
  - [x] Retourner la fonction showToast

- [x] **4.16.4** Intégrer les toasts dans l'app
  - [x] Ajouter ToastContainer dans App.tsx
  - [x] Utiliser showToast pour les succès (profil créé, son ajouté, etc.)
  - [x] Utiliser showToast pour les erreurs
  - [x] Utiliser showToast pour les infos (téléchargement en cours, etc.)

### 4.17 Intégration des Commandes Tauri

- [x] **4.17.1** Créer les wrappers de commandes dans `src/utils/tauriCommands.ts`
  - [x] Wrapper pour toutes les commandes config
  - [x] Wrapper pour toutes les commandes profile
  - [x] Wrapper pour toutes les commandes audio
  - [x] Wrapper pour toutes les commandes sounds
  - [x] Wrapper pour toutes les commandes keys
  - [x] Wrapper pour toutes les commandes import/export
  - [x] Typage TypeScript pour les paramètres et retours
  - [x] Gestion des erreurs avec try/catch

- [x] **4.17.2** Utiliser les wrappers dans les composants
  - [x] Remplacer les `invoke()` par les wrappers typés
  - [x] Gérer les erreurs de manière consistante
  - [x] Afficher les toasts appropriés

### 4.18 Event Listeners

- [x] **4.18.1** Créer `src/utils/tauriEvents.ts`
  - [x] Créer des fonctions pour écouter chaque type d'event
  - [x] `onSoundStarted(callback)`
  - [x] `onSoundEnded(callback)`
  - [x] `onPlaybackProgress(callback)`
  - [x] `onKeyPressed(callback)`
  - [x] `onDownloadProgress(callback)`
  - [x] `onDownloadComplete(callback)`
  - [x] `onDownloadError(callback)`
  - [x] `onSoundNotFound(callback)`
  - [x] Retourner les fonctions unlisten pour cleanup

- [x] **4.18.2** Utiliser les event listeners dans les composants
  - [x] Écouter dans useEffect
  - [x] Cleanup au unmount
  - [x] Mettre à jour les stores appropriés

---

## Phase 4.5 - Bug Fixes & Améliorations ✅ COMPLÉTÉE

### 4.5.1 Corrections UI

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

### 4.5.2 Master Stop & Key Detection

- [x] **4.5.2.1** Fix Master Stop not working when app is focused
  - [x] Added browser keyboard handler with pressed keys tracking (useRef<Set<string>>)
  - [x] On keydown: checks if all masterStopShortcut keys are pressed
  - [x] On keyup: removes key from set
  **✅ Complété** - useKeyDetection.ts

### 4.5.3 Loop Mode & Sound Selection

- [x] **4.5.3.1** Change loop mode "off" to random selection
  - [x] When multiple sounds on same key with mode "off", picks random sound (avoids repeat)
  - [x] Sound stops when finished (no auto-play next)
  - [x] Updated currentIndex tracking to include "off" mode
  **✅ Complété** - useKeyDetection.ts

### 4.5.4 Key Binding Names

- [x] **4.5.4.1** Add custom name field to KeyBinding
  - [x] Added `name?: string` to TypeScript KeyBinding interface
  - [x] Added `#[serde(default)] pub name: Option<String>` to Rust KeyBinding struct
  - [x] KeyGrid displays `kb.name || firstSound?.name` and total sound count
  - [x] SoundDetails has editable name input with debounced save
  **✅ Complété** - types/index.ts, types.rs, KeyGrid.tsx, SoundDetails.tsx

### 4.5.5 AddSoundModal Improvements

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

### 4.5.6 Duration Computation

- [x] **4.5.6.1** Replace rodio duration with symphonia header reading
  - [x] Uses symphonia to probe format and read `n_frames` from track params
  - [x] Returns `n_frames / sample_rate` — instant without decoding
  - [x] Falls back to rodio sample-counting if headers lack frame count
  **✅ Complété** - audio/buffer.rs

### 4.5.7 Now Playing Seekable Slider

- [x] **4.5.7.1** Add interactive seek slider
  - [x] Drag-then-release pattern (onChange sets local state, onMouseUp triggers seek)
  - [x] Stop button (■) per active track
  - [x] `updateProgress()` called before async `playSound` to prevent slider jump-back
  **✅ Complété** - NowPlaying.tsx

### 4.5.8 Real-time Sound Volume

- [x] **4.5.8.1** Add SetSoundVolume command through full stack
  - [x] Added `SetSoundVolume { track_id, sound_id, volume }` to AudioCommand enum
  - [x] Handler updates sound_volumes map and recalculates sink volume
  - [x] Added `set_sound_volume` Tauri command in commands.rs
  - [x] Added `setSoundVolume` wrapper in tauriCommands.ts
  - [x] SoundDetails volume slider calls `commands.setSoundVolume` on change
  **✅ Complété** - engine.rs, commands.rs, main.rs, tauriCommands.ts, SoundDetails.tsx

---

## Phase 4.6 - UX Enhancements & Key Management ✅ COMPLÉTÉE

### 4.6.1 Resizable Panel Divider

- [x] **4.6.1.1** Add resizable divider bar above SoundDetails panel
  - [x] `panelHeight` state (default 256px) with `isResizing` ref
  - [x] Divider bar: `h-1.5 cursor-ns-resize` with hover highlight
  - [x] Mouse/touch drag handlers (mousedown/mousemove/mouseup)
  - [x] Body cursor override during drag, constraints (min 120px, max container-100px)
  **✅ Complété** - MainContent.tsx

### 4.6.2 Track Management

- [x] **4.6.2.1** Change track of existing key binding
  - [x] Added track dropdown selector in SoundDetails panel
  - [x] `handleTrackChange` updates binding's trackId with auto-save
  **✅ Complété** - SoundDetails.tsx

- [x] **4.6.2.2** Rename tracks with double-click
  - [x] `editingTrackId` and `editingName` state in TrackView
  - [x] Double-click on track name enters edit mode (input with autoFocus)
  - [x] Confirm on blur or Enter, cancel on Escape
  **✅ Complété** - TrackView.tsx

### 4.6.3 Profile Switch & Shortcuts

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

### 4.6.4 AZERTY Layout Support

- [x] **4.6.4.1** Fix AZERTY key display (showing QWERTY letters)
  - [x] Dynamic `layoutMap: Map<string, string>` populated from keydown events
  - [x] `recordKeyLayout(code, key)` records actual character for physical keys
  - [x] `keyCodeToDisplay` checks layoutMap first, falls back to QWERTY map
  - [x] Shortcut capture uses `charToKeyCode(e.key) || e.code` for layout-independent codes
  - [x] Browser handler and settings both call `recordKeyLayout` on keydown
  **✅ Complété** - keyMapping.ts, useKeyDetection.ts, SettingsModal.tsx

### 4.6.5 Key Reassignment

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

### 4.6.6 Global Shortcuts Consistency

- [x] **4.6.6.1** All global shortcuts work regardless of key detection state
  - [x] Moved master stop and auto-momentum shortcut checks before the `enabled` guard in detector.rs
  - [x] All three shortcuts (key detection, master stop, auto-momentum) now fire even when detection is off, both in foreground and background
  **✅ Complété** - detector.rs

- [x] **4.6.6.2** Fix sticky modifier keys (Alt/Ctrl stuck after window switch)
  - [x] Added `blur` event listener that clears `pressedKeysRef` when window loses focus
  - [x] Prevents phantom modifier keys from triggering shortcuts after Alt+Tab
  **✅ Complété** - useKeyDetection.ts

---

## Phase 5 - Téléchargement YouTube ✅

### 5.1 Module YouTube Backend ✅

- [x] **5.1.1** Créer `src-tauri/src/youtube/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter downloader et cache

- [x] **5.1.2** Créer `src-tauri/src/youtube/cache.rs` (structures)
  - [x] Définir `CacheEntry` struct
  - [x] Définir `SoundReference` struct
  - [x] Définir `YouTubeCache` struct avec HashMap<String, CacheEntry>
  - [x] Ajouter les champs: index_path, cache_dir

### 5.2 Système de Cache ✅

- [x] **5.2.1** Implémenter YouTubeCache (lecture/écriture)
  - [x] Implémenter `new() -> YouTubeCache` (utilise get_app_data_dir)
  - [x] Implémenter `load_index() -> Result<()>` pour charger cache_index.json
  - [x] Implémenter `save_index() -> Result<()>` pour sauvegarder cache_index.json
  - [x] Gérer les erreurs JSON

- [x] **5.2.2** Implémenter la logique de cache
  - [x] Implémenter `get(url: &str) -> Option<&CacheEntry>`
  - [x] Implémenter `add_entry(url, cached_path, title, file_size) -> CacheEntry`
  - [x] Implémenter `add_usage(url, profile_id, sound_id)`
  - [x] Implémenter `remove_usage(url, profile_id, sound_id)`
  - [x] Implémenter la suppression automatique si plus d'usage

- [x] **5.2.3** Vérification de l'intégrité du cache
  - [x] Implémenter `verify_integrity()` au démarrage
  - [x] Vérifier que tous les fichiers référencés existent
  - [x] Retirer les entrées dont les fichiers sont manquants
  - [x] Sauvegarder le cache nettoyé

### 5.3 Downloader YouTube ✅

- [x] **5.3.1** Créer `src-tauri/src/youtube/downloader.rs`
  - [x] Fonctions standalone (pas de struct, approche fonctionnelle)
  - [x] Utilise Arc<Mutex<YouTubeCache>> pour thread-safety

- [x] **5.3.2** Implémenter la validation des URLs
  - [x] Implémenter `is_valid_youtube_url(url: &str) -> bool`
  - [x] Vérifier youtube.com/watch?v=, youtu.be/, youtube.com/shorts/
  - [x] Extraire le video ID

- [x] **5.3.3** Implémenter l'extraction du video ID
  - [x] Implémenter `extract_video_id(url: &str) -> Option<String>`
  - [x] Parser l'URL pour extraire l'ID (11 caractères)
  - [x] Gérer les 3 formats d'URL YouTube

- [x] **5.3.4** Implémenter le sanitization des noms de fichiers
  - [x] Implémenter `sanitize_title(title: &str) -> String`
  - [x] Remplacer les caractères invalides (<>:"/\|?*)
  - [x] Limiter la longueur à 200 caractères
  - [x] Trim les espaces

- [x] **5.3.5** Implémenter le téléchargement avec yt-dlp
  - [x] Implémenter `download_audio(url, cache) -> Result<CacheEntry>`
  - [x] Vérifier si yt-dlp est installé (`check_yt_dlp_installed`)
  - [x] Construire la commande yt-dlp: `-x --audio-format mp3 --audio-quality 0 --no-playlist --no-warnings`
  - [x] Exécuter la commande (tokio::process::Command)
  - [x] Capturer stdout/stderr
  - [x] Chercher le fichier téléchargé par video_id si le chemin attendu n'existe pas

- [x] **5.3.6** Implémenter l'extraction du titre
  - [x] Exécuter `yt-dlp --get-title --no-warnings URL`
  - [x] Capturer la sortie
  - [x] Fallback sur video_id si erreur
  - [x] Gérer les erreurs

- [x] **5.3.7** Implémenter le parsing d'erreurs
  - [x] `parse_yt_dlp_error(stderr)` → messages utilisateur clairs
  - [x] Vidéo privée/sign-in, indisponible, URL invalide, réseau, géo-restriction

- [x] **5.3.8** Implémenter la logique cache-first
  - [x] Vérifier le cache en premier dans `download_audio`
  - [x] Si en cache: retourner l'entrée immédiatement
  - [x] Si pas en cache: télécharger, ajouter au cache, sauvegarder l'index

### 5.4 Commandes YouTube Tauri ✅

- [x] **5.4.1** Ajouter les commandes YouTube dans `commands.rs`
  - [x] `add_sound_from_youtube(url: String) -> Result<Sound, String>`
    - [x] Valider l'URL (via downloader)
    - [x] Appeler `download_audio()` avec le cache
    - [x] Créer un Sound avec SoundSource::YouTube
    - [x] Obtenir la durée via symphonia (spawn_blocking)
    - [x] Retourner le Sound
  - [x] `check_yt_dlp_installed() -> Result<bool, String>`
    - [x] Tenter d'exécuter `yt-dlp --version`
    - [x] Retourner true si succès, false sinon

- [x] **5.4.2** Intégrer avec le système de cache
  - [x] Cache initialisé au démarrage dans main.rs
  - [x] Vérification d'intégrité au démarrage
  - [x] `add_usage` et `remove_usage` disponibles pour utilisation future

- [x] **5.4.3** Enregistrer les commandes dans main.rs

### 5.5 Gestion des Erreurs YouTube ✅

- [x] **5.5.1** Gérer les erreurs spécifiques
  - [x] yt-dlp non installé → message clair avec lien
  - [x] URL invalide → "Invalid YouTube URL"
  - [x] Vidéo privée/indisponible → parser l'erreur de yt-dlp
  - [x] Erreur réseau → "Network error. Check your internet connection"
  - [x] Géo-restriction → "Not available in your region"
  - [x] Mapping vers des messages utilisateur clairs

- [x] **5.5.2** UI Frontend YouTube
  - [x] Toggle Local/YouTube dans AddSoundModal
  - [x] Input URL avec bouton Download
  - [x] État de chargement pendant le téléchargement
  - [x] Vérification yt-dlp installé avec message d'erreur
  - [x] Intégration du Sound retourné dans le flux existant
  - [x] Toast de succès/erreur

### 5.6 YouTube Fixes & Improvements ✅ (2026-01-24)

- [x] **5.6.1** Fix DASH M4A format non-lisible
  - [x] Créer `ffmpeg_manager.rs` pour auto-download ffmpeg
  - [x] Télécharger ffmpeg depuis `yt-dlp/FFmpeg-Builds` GitHub releases
  - [x] Extraire `ffmpeg.exe` depuis l'archive ZIP
  - [x] Passer `--ffmpeg-location` à yt-dlp pour remux automatique
  - [x] Ajouter dépendance `zip` au Cargo.toml

- [x] **5.6.2** Fix extraction du titre
  - [x] Remplacer `--print "%(title)s"` par `--write-info-json` (yt-dlp 2025.12.08 skip le download avec --print)
  - [x] Lire le titre depuis `{video_id}.info.json`
  - [x] Cleanup du fichier info.json après lecture

- [x] **5.6.3** Fix cache lookups
  - [x] Implémenter `canonical_url()` pour normaliser les URLs (strip list=, pp=, etc.)
  - [x] Utiliser video ID comme nom de fichier (`%(id)s.%(ext)s`)
  - [x] Nettoyer les entrées stale du cache

- [x] **5.6.4** Implémenter retry logic pour erreurs réseau
  - [x] Boucle retry jusqu'à 3 tentatives
  - [x] Délai de 2 secondes entre les retries
  - [x] `is_retryable_error()` pour identifier les erreurs transientes
  - [x] Nettoyage des fichiers partiels entre retries
  - [x] Émission du statut "Retrying..." vers le frontend

- [x] **5.6.5** Playback M4A/AAC
  - [x] Utiliser SymphoniaSource pour TOUT le playback (pas seulement momentum)
  - [x] Ajouter feature `isomp4` à symphonia pour support M4A

- [x] **5.6.6** Commandes Tauri ffmpeg
  - [x] `check_ffmpeg_installed()` → commande Tauri
  - [x] `install_ffmpeg()` → commande Tauri
  - [x] Enregistrer dans main.rs invoke_handler
  - [x] Ajouter wrappers TypeScript dans tauriCommands.ts

- [x] **5.6.7** Cache cleanup automatique
  - [x] Supprimer `SoundReference` et `usedBy` (jamais populé, approche fragile)
  - [x] Supprimer `add_usage()` et `remove_usage()`
  - [x] Implémenter `cleanup_unused()` : scan tous les profils → collecte `cachedPath` → supprime les non-référencés
  - [x] Implémenter `collect_used_cached_paths()` : parse les JSONs profils via serde_json::Value
  - [x] Appeler cleanup au démarrage (main.rs, après verify_integrity)
  - [x] Appeler cleanup après `save_profile` (commands.rs, ajout param state)
  - [x] Appeler cleanup après `delete_profile` (commands.rs, ajout param state)

---

## Phase 6 - Import/Export ✅ COMPLÉTÉE

### 6.1 Module Import/Export Backend

- [x] **6.1.1** Créer `src-tauri/src/import_export/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter les fonctions export et import
  **✅ Complété** - Module avec export/import submodules et ExportMetadata struct

- [x] **6.1.2** Créer les structures pour les métadonnées
  - [x] Définir `ExportMetadata` struct
  - [x] Champs: version, exported_at, app_version, platform
  **✅ Complété** - Struct avec serde Serialize/Deserialize

### 6.2 Export de Profil

- [x] **6.2.1** Créer `src-tauri/src/import_export/export.rs`
  - [x] Implémenter `export_profile(profile_id, output_path) -> Result<()>`
  **✅ Complété**

- [x] **6.2.2** Implémenter la logique d'export
  - [x] Charger le profil depuis storage
  - [x] Créer le sous-dossier `sounds/` dans le ZIP
  - [x] Copier tous les fichiers audio vers `sounds/`
  - [x] Mettre à jour les chemins dans le profil (chemins relatifs)
  - [x] Gérer les sons YouTube (copier depuis cache)
  - [x] Sérialiser le profil modifié en JSON
  - [x] Écrire `profile.json`
  - [x] Créer les métadonnées
  - [x] Écrire `metadata.json`
  **✅ Complété** - Écrit directement dans le ZIP sans répertoire temporaire (plus efficace)

- [x] **6.2.3** Créer le fichier ZIP
  - [x] Utiliser la crate `zip` pour créer le .ktm
  - [x] Ajouter tous les fichiers (profile.json, metadata.json, sounds/*)
  - [x] Compresser avec Deflate
  - [x] Écrire le fichier final à output_path
  - [x] Gestion des noms de fichiers dupliqués (make_unique_filename)
  **✅ Complété**

- [x] **6.2.4** Gérer les erreurs d'export
  - [x] Profil non trouvé (via storage::load_profile)
  - [x] Fichier audio manquant (vérifié avant ajout au ZIP)
  - [x] Erreur d'écriture fichier/zip
  - [x] Retourner des erreurs claires
  **✅ Complété**

### 6.3 Import de Profil

- [x] **6.3.1** Créer `src-tauri/src/import_export/import.rs`
  - [x] Implémenter `import_profile(ktm_path) -> Result<ProfileId>`
  **✅ Complété**

- [x] **6.3.2** Implémenter la logique d'import
  - [x] Vérifier que le fichier .ktm existe
  - [x] Ouvrir et lire le ZIP
  - [x] Vérifier la présence de profile.json
  - [x] Charger et parser metadata.json (optionnel, pour compatibilité future)
  - [x] Charger et parser profile.json
  **✅ Complété**

- [x] **6.3.3** Gérer les IDs et noms
  - [x] Générer un nouvel UUID pour le profil
  - [x] Mettre à jour profile.id
  - [x] Ajouter "(Imported)" au nom du profil
  - [x] Générer de nouveaux UUIDs pour tous les sons (éviter conflits)
  - [x] Mettre à jour les références dans keyBindings
  **✅ Complété**

- [x] **6.3.4** Copier les fichiers audio
  - [x] Créer le dossier `imported_sounds/{new_profile_id}/`
  - [x] Extraire les fichiers depuis le ZIP
  - [x] Mettre à jour les chemins dans les sons (chemins absolus)
  - [x] Fallback: essayer chemin complet puis nom de fichier seul
  **✅ Complété**

- [x] **6.3.5** Finaliser l'import
  - [x] Mettre à jour les timestamps (createdAt, updatedAt)
  - [x] Sauvegarder le nouveau profil via storage
  - [x] Retourner le nouveau ProfileId
  **✅ Complété**

- [x] **6.3.6** Gérer les erreurs d'import
  - [x] Fichier .ktm invalide ou corrompu (ZIP parsing error)
  - [x] Fichiers manquants dans le ZIP
  - [x] Erreur de parsing JSON
  - [x] Erreur de copie fichiers
  - [x] Retourner des erreurs claires
  **✅ Complété**

### 6.4 Commandes Import/Export Tauri

- [x] **6.4.1** Ajouter les commandes dans `commands.rs`
  - [x] `export_profile(profile_id: String, output_path: String) -> Result<(), String>`
  - [x] `import_profile(ktm_path: String) -> Result<String, String>`
  **✅ Complété** - Async commands avec tokio::spawn_blocking

- [x] **6.4.2** Ajouter les commandes de file dialogs
  - [x] `pick_save_location(default_name: String) -> Result<Option<String>, String>`
    - [x] Utiliser rfd (Rust File Dialog) pour dialogs natifs
    - [x] Filtre pour fichier .ktm
    - [x] Nom par défaut: "ProfileName.ktm"
  - [x] `pick_ktm_file() -> Result<Option<String>, String>`
    - [x] File picker pour .ktm
    - [x] Retourner le chemin sélectionné (ou null si annulé)
  **✅ Complété** - Utilise la crate `rfd` pour les dialogs natifs cross-platform

- [x] **6.4.3** Enregistrer les commandes dans main.rs
  **✅ Complété** - 4 nouvelles commandes enregistrées dans invoke_handler

### 6.5 Intégration Frontend

- [x] **6.5.1** Bouton Export dans Settings
  - [x] Bouton "Export Profile" dans SettingsModal
  - [x] Flow: pick save location → export → success/error message
  - [x] Désactivé si aucun profil sélectionné
  - [x] Status messages (Choosing location... / Exporting... / Success / Error)
  **✅ Complété**

- [x] **6.5.2** Bouton Import dans Settings
  - [x] Bouton "Import Profile" dans SettingsModal
  - [x] Flow: pick .ktm file → import → reload profiles → select imported
  - [x] Recharger la liste des profils après import
  - [x] Sélectionner automatiquement le profil importé
  - [x] Status messages
  **✅ Complété**

### 6.6 Export UX Improvements

- [x] **6.6.1** Barre de progression Export
  - [x] Émettre des events `export_progress` depuis le backend (current, total, filename)
  - [x] Créer `ExportProgress.tsx` - barre de progression flottante (bottom-right)
  - [x] Créer `exportStore.ts` - store Zustand global pour l'état d'export
  - [x] Afficher le compteur (current/total) et le nom du fichier en cours
  **✅ Complété** - Progress callback dans export.rs, event Tauri, composant flottant

- [x] **6.6.2** Bouton annulation d'export
  - [x] Ajouter `EXPORT_CANCELLED: AtomicBool` static dans export.rs
  - [x] Vérifier le flag entre chaque fichier copié dans la boucle d'export
  - [x] Sur annulation: supprimer le fichier temp, le tracking file, retourner erreur
  - [x] Ajouter commande Tauri `cancel_export`
  - [x] Bouton "x" sur le composant ExportProgress
  - [x] Toast "Export cancelled" (info, pas error)
  **✅ Complété**

- [x] **6.6.3** Interception fermeture fenêtre pendant export
  - [x] Handler `onCloseRequested` avec confirmation dialog
  - [x] Pattern `forceCloseRef` pour éviter boucle infinie
  - [x] Ajouter permissions `core:window:allow-destroy` et `core:window:allow-close`
  - [x] Appeler `cleanupExportTemp()` avant fermeture confirmée
  **✅ Complété**

- [x] **6.6.4** Nettoyage fichiers temporaires orphelins
  - [x] Écrire le chemin temp dans `export_in_progress.txt` avant export
  - [x] Supprimer le tracking file après export réussi
  - [x] `cleanup_interrupted_export()` au démarrage de l'app
  - [x] Commande Tauri `cleanup_export_temp`
  **✅ Complété**

---

## Phase 6.5 - Concurrent YouTube Downloads & Key Cycling ✅

### 6.7 Téléchargements YouTube Concurrents ✅

- [x] **6.7.1** Backend: Ajouter `download_id` au command `add_sound_from_youtube`
  - [x] Paramètre `download_id: String` dans la signature
  - [x] Inclure `downloadId` dans le payload de l'event `youtube_download_progress`
  **✅ Complété**

- [x] **6.7.2** Frontend: Remplacer l'état single-download par multi-download
  - [x] Remplacer `isDownloading`/`downloadStatus`/`downloadProgress` par `activeDownloads` Map
  - [x] Chaque download trackée avec son propre ID, URL, status, progress
  - [x] URL input reste actif pendant les téléchargements (jamais disabled)
  - [x] Bouton Download uniquement disabled si URL vide
  - [x] Chaque download complété s'ajoute à la liste des fichiers
  **✅ Complété**

- [x] **6.7.3** Frontend: Affichage progression individuelle
  - [x] Barre de progression par téléchargement actif
  - [x] Spinner et status text par download
  - [x] Downloads retirés de la Map une fois terminés (succès ou erreur)
  **✅ Complété**

- [x] **6.7.4** Mise à jour `tauriCommands.ts`
  - [x] `addSoundFromYoutube(url, downloadId)` - nouveau paramètre downloadId
  **✅ Complété**

### 6.8 Key Cycling pour Assignation Multi-Sons ✅

- [x] **6.8.1** Supprimer la limitation de longueur du champ keys
  - [x] Retirer `.slice(0, files.length)` dans `handleKeyInput`
  - [x] L'utilisateur peut taper moins de touches que de fichiers
  **✅ Complété**

- [x] **6.8.2** Affichage cycling dans la liste des fichiers
  - [x] Indicateur de touche par fichier utilise `keysInput[i % keysInput.length]`
  - [x] Reflète le cycling en temps réel pendant la saisie
  **✅ Complété**

- [x] **6.8.3** Logique de submit avec cycling (déjà implémentée)
  - [x] `keyCodes[i % keyCodes.length]` regroupe les sons par touche
  - [x] Un seul caractère "a" → tous les sons sur la même touche
  - [x] "ab" avec 5 sons → a,b,a,b,a
  **✅ Complété**

---

## Phase 7 - Gestion des Erreurs ✅ COMPLÉTÉE

### 7.1 Son d'Erreur

- [x] **7.1.1** Fichier `resources/sounds/error.mp3` existe déjà
  - [x] Son d'erreur court (< 1 seconde, format MP3)
  - [x] Bundled via tauri.conf.json resources
  **✅ Complété** - error.mp3 déjà présent, ajouté au bundle Tauri

- [x] **7.1.2** Implémenter la lecture du son d'erreur
  - [x] Créé `SetErrorSoundPath` et `PlayErrorSound` commands dans AudioEngine
  - [x] Chargé error.mp3 depuis resource_dir dans setup()
  - [x] Joué en one-shot via sink.detach() (fire-and-forget)
  - [x] Ne pas affecter les autres sons en cours (piste séparée)
  **✅ Complété** - Error sound joue via SymphoniaSource avec volume master * 0.5

- [x] **7.1.3** Déclencher le son d'erreur
  - [x] play_error_sound() appelé dans play_sound quand fichier manquant
  - [x] Événement sound_not_found émis simultanément
  **✅ Complété** - Error sound + event émis dans commands.rs play_sound

### 7.2 Messages d'Erreur Utilisateur

- [x] **7.2.1** Créer un mapping erreurs → messages dans le frontend
  - [x] Créé `src/utils/errorMessages.ts`
  - [x] Fonction `formatErrorMessage(rawError: string) -> string`
  - [x] Mapping par pattern regex vers messages user-friendly
  **✅ Complété** - Patterns couvrent les erreurs audio, device, YouTube

- [x] **7.2.2** Afficher les messages d'erreur
  - [x] Toast pour les erreurs non-bloquantes (audio_error events)
  - [x] FileNotFoundModal pour les fichiers manquants (queue-based)
  - [x] Toast dans useKeyDetection pour erreurs de lecture non-file-related
  **✅ Complété** - Dual system: toast + modal selon le type d'erreur

### 7.3 Vérification des Fichiers au Chargement

- [x] **7.3.1** Implémenter `verify_profile_sounds(profile) -> Vec<MissingSoundInfo>`
  - [x] Commande Tauri dans commands.rs
  - [x] Vérifie chaque son Local: fichier existe
  - [x] Vérifie chaque son YouTube: fichier caché existe
  - [x] Retourne la liste des sons manquants avec soundId, name, path, sourceType
  **✅ Complété** - Verification complète au niveau backend

- [x] **7.3.2** Vérifier au chargement d'un profil
  - [x] Appel verifyProfileSounds() dans profileStore.loadProfile()
  - [x] Sons manquants ajoutés à errorStore.missingQueue
  - [x] FileNotFoundModal affiche les erreurs une par une
  **✅ Complété** - Queue-based modal avec Skip/Skip All

- [x] **7.3.3** Vérifier avant de jouer un son
  - [x] Dans play_sound(), vérification que le fichier existe
  - [x] Si manquant: play_error_sound() + événement sound_not_found
  - [x] Retourne Err() sans crasher l'app
  **✅ Complété** - Vérifié dans commands.rs avec error sound + event

### 7.4 Gestion du Cache Corrompu

- [x] **7.4.1** Détecter un cache corrompu
  - [x] verify_profile_sounds détecte les fichiers YouTube manquants
  - [x] sound_not_found event émis avec sourceType "youtube"
  **✅ Complété** - Détection via le même système de vérification

- [x] **7.4.2** Proposer de re-télécharger
  - [x] FileNotFoundModal avec bouton "Re-download" pour type youtube
  - [x] Bouton "Remove" pour supprimer le son
  - [x] Bouton "Locate File" pour les sons locaux
  - [x] Appel addSoundFromYoutube pour re-download
  **✅ Complété** - Modal adaptatif selon sourceType (local vs youtube)

### 7.5 Logs et Debugging

- [x] **7.5.1** Implémenter un système de logging
  - [x] Utilise la crate `tracing` + `tracing-subscriber` + `tracing-appender`
  - [x] Configurable via RUST_LOG env var (défaut: info)
  - [x] Logger vers fichier daily rolling dans `{app_data}/logs/keytomusic.log`
  - [x] Rotation automatique des logs (fichiers journaliers)
  **✅ Complété** - tracing avec rolling daily appender

- [x] **7.5.2** Logger les événements importants
  - [x] Démarrage de l'app (info)
  - [x] Erreurs audio (error via audio_error event)
  - [x] Sons manquants (warn)
  - [x] Erreurs de config/storage (warn)
  - [x] Chargement error sound (info)
  **✅ Complété** - eprintln remplacés par tracing macros

- [x] **7.5.3** Ajouter une commande pour ouvrir les logs
  - [x] `get_logs_folder() -> Result<String, String>`
  - [x] Bouton "Open Logs Folder" dans Settings → About
  - [x] Utilise @tauri-apps/plugin-shell open() pour ouvrir le dossier
  **✅ Complété** - Bouton dans SettingsModal section About

### 7.6 AddSoundModal - File Picker & Drag-Drop UX

- [x] **7.6.1** Ajouter un bouton "Add Files" avec file picker natif
  - [x] Commande `pick_audio_files()` backend (rfd multi-file picker avec filtre audio)
  - [x] Wrapper frontend dans tauriCommands.ts
  - [x] Bouton "Add Files" dans AddSoundModal qui appelle pickAudioFiles()
  **✅ Complété** - Bouton natif, pas de champ de texte manuel

- [x] **7.6.2** Supprimer le champ de texte et bouton "Add" manuel
  - [x] Supprimé pathInput state, handleAddPath, isAudioFile import
  - [x] Interface simplifiée: uniquement "Add Files" (natif) + drag & drop
  **✅ Complété** - UX épurée, 99% des utilisateurs utilisent browse ou drag & drop

- [x] **7.6.3** Corriger le comportement drag & drop avec le modal ouvert
  - [x] Drop quand modal ouvert → append les fichiers au modal existant (pas remplacer)
  - [x] Utilisation de processedFilesRef pour distinguer mount vs drop subséquent
  - [x] Safe en React StrictMode (pas de double-ajout)
  - [x] Fermer le modal puis ré-ouvrir → pas de fichiers résiduels (useState initializer)
  **✅ Complété** - Ref-based deduplication pattern

---

## Phase 7.5 - Legacy Import ✅ COMPLÉTÉE

### 7.5.1 Backend - Commande de conversion

- [x] **7.5.1.1** Définir les structs de parsing du format legacy
  - [x] `LegacySave` avec champ `Sounds: Vec<LegacyKeyEntry>`
  - [x] `LegacyKeyEntry` avec `Key` (u32), `UserKeyChar` (String), `SoundInfos` (Vec)
  - [x] `LegacySoundInfo` avec `uniqueId`, `soundPath`, `soundName`, `soundMomentum`
  **✅ Complété** - Structs avec `#[derive(serde::Deserialize)]` et `#[allow(non_snake_case)]`

- [x] **7.5.1.2** Implémenter `vk_to_keycode()` pour convertir les codes VK Windows en KeyCode web
  - [x] 65-90 → KeyA-KeyZ
  - [x] 48-57 → Digit0-Digit9
  - [x] 112-123 → F1-F12
  - [x] OEM keys (186-222) → Semicolon, Equal, Comma, etc.
  - [x] Touches spéciales (Space, Enter)
  **✅ Complété** - Mapping complet dans `commands.rs`

- [x] **7.5.1.3** Implémenter la commande `pick_legacy_file`
  - [x] File picker filtré sur `.json`
  **✅ Complété** - Utilise `rfd::FileDialog` avec filtre "Legacy Save" (*.json)

- [x] **7.5.1.4** Implémenter la commande `import_legacy_save`
  - [x] Lire et parser le fichier JSON legacy
  - [x] Créer un profil avec UUID, timestamps, nom dérivé du fichier
  - [x] Créer un track "OST" par défaut
  - [x] Convertir chaque entrée: VK code → keyCode, SoundInfos → Sound + KeyBinding
  - [x] Normaliser les chemins (`/` → `\` sur Windows)
  - [x] Sauvegarder le profil via `storage::save_profile`
  - [x] Logger le résultat (nombre de sons, bindings)
  **✅ Complété** - Conversion complète avec gestion des clés inconnues (skip avec warning)

- [x] **7.5.1.5** Enregistrer les commandes dans `main.rs`
  **✅ Complété** - `pick_legacy_file` et `import_legacy_save` dans `invoke_handler`

### 7.5.2 Frontend - Wrapper et UI

- [x] **7.5.2.1** Ajouter les fonctions dans `tauriCommands.ts`
  - [x] `pickLegacyFile(): Promise<string | null>`
  - [x] `importLegacySave(path: string): Promise<Profile>`
  **✅ Complété**

- [x] **7.5.2.2** Ajouter le bouton "Import Legacy Save" dans `SettingsModal.tsx`
  - [x] Bouton stylé en jaune (distinctif par rapport à l'import standard)
  - [x] Flow: pick file → convert → loadProfiles → loadProfile
  - [x] Affichage du status (converting, success, error) via `importStatus`
  **✅ Complété** - Bouton intégré dans la section Import/Export

---

## Phase 8 - Nouvelles Features 🔄 EN COURS

Cette phase ajoute des fonctionnalités demandées pour améliorer l'UX sans alourdir l'interface.

> **Note**: Phase 8.1 (Duplication) et 8.3 (Undo/Redo) sont complétées. Phase 8.2 (Combined Shortcuts) est partiellement implémentée - le backend fonctionne mais l'UI nécessite une refonte.

### 8.1 Duplication de Profil ✅

- [x] **8.1.1** Ajouter la commande backend `duplicate_profile`
  - [x] Créer `src-tauri/src/storage/profile.rs::duplicate_profile(id: String, new_name: String)`
  - [x] Charger le profil source
  - [x] Générer un nouvel UUID pour le profil dupliqué
  - [x] Mettre à jour `createdAt` et `updatedAt`
  - [x] Copier tous les sons, tracks, et key bindings
  - [x] Sauvegarder le nouveau profil
  - [x] Retourner le profil dupliqué
  **✅ Complété** - Fonction `duplicate_profile` ajoutée dans `storage/profile.rs`

- [x] **8.1.2** Ajouter la commande Tauri `duplicate_profile`
  - [x] Créer dans `commands.rs`: `duplicate_profile(id: String, new_name: Option<String>) -> Result<Profile, String>`
  - [x] Si `new_name` est None, utiliser "{original_name} (Copy)"
  - [x] Enregistrer la commande dans `main.rs`
  **✅ Complété** - Commande Tauri créée et enregistrée

- [x] **8.1.3** Ajouter l'option dans le menu contextuel du ProfileSelector
  - [x] Ajouter "Duplicate" dans le menu (bouton ⎘ avant Delete)
  - [x] Appeler `duplicateProfile` du profileStore
  - [x] Rafraîchir la liste des profils après duplication
  - [x] Sélectionner automatiquement le profil dupliqué
  **✅ Complété** - Bouton "Duplicate" ajouté avec icône ⎘

- [x] **8.1.4** Ajouter `duplicateProfile` dans `profileStore.ts`
  - [x] Appeler `commands.duplicateProfile(id)`
  - [x] Ajouter le nouveau profil à la liste
  - [x] Sélectionner le nouveau profil
  **✅ Complété** - Fonction ajoutée au store et à tauriCommands.ts

### 8.2 Raccourcis Clavier Combinés (Modificateurs) ✅ COMPLÉTÉ

Permettre l'utilisation de combinaisons comme Ctrl+A, Shift+F1, Alt+1 comme triggers de sons.

**Backend : ✅ Complété** | **Frontend UI : ⏳ En attente**

> **Note**: Le backend envoie les codes combinés mais l'UI AddSoundModal utilise encore un input texte "aze" qui ne supporte pas les combinaisons. Une refonte UI est nécessaire (voir 8.2.6).

- [x] **8.2.1** Modifier le type `KeyBinding` pour supporter les modificateurs
  - [x] Utiliser une notation combinée dans `keyCode` (ex: "Ctrl+KeyA")
  - [x] Approche choisie: notation string combinée (plus simple, backward compatible)
  **✅ Complété** - Notation combinée "Ctrl+Shift+KeyA" utilisée

- [x] **8.2.2** Modifier le détecteur de touches backend (`detector.rs`)
  - [x] Lors d'un KeyPress, vérifier si des modificateurs sont maintenus
  - [x] Construire le code combiné (ex: si Ctrl+Shift maintenus et KeyA pressé → "Ctrl+Shift+KeyA")
  - [x] Émettre l'événement avec le code combiné
  - [x] Ne pas bloquer les touches modificateurs seules
  **✅ Complété** - Ordre: Ctrl > Shift > Alt > Key

- [x] **8.2.3** Modifier le frontend pour supporter les combinaisons
  - [x] Mettre à jour `useKeyDetection.ts` pour construire le code combiné
  - [x] Matcher d'abord le code combiné, puis fallback sur la touche de base
  - [x] Shift+X sur binding "X" applique le momentum (comportement existant préservé)
  **✅ Complété** - Logique de matching avec fallback implémentée

- [x] **8.2.4** Mettre à jour `keyMapping.ts` pour l'affichage
  - [x] Fonction `keyCodeToDisplay` mise à jour pour gérer "Ctrl+Shift+A"
  - [x] Fonctions `buildKeyCombo` et `parseKeyCombo` ajoutées
  - [x] Gérer l'ordre d'affichage (Ctrl avant Shift avant Alt avant la touche)
  **✅ Complété** - Affichage correct des combinaisons

- [x] **8.2.5** Mettre à jour les validations
  - [x] Fonction `checkKeyComboConflict` ajoutée
  - [x] Vérifie les conflits avec Ctrl+C/V/X/Z/Y/A/S/W/Q/N/T, Alt+F4
  - [x] Avertit pour Ctrl+chiffre (tabs) et Alt+lettre (menus Windows)
  **✅ Complété** - Validation des conflits système implémentée

- [x] **8.2.6** Validation étendue des raccourcis réservés
  - [x] Étendre `checkKeyComboConflict` → `checkShortcutConflicts(combo, config)`
  - [x] Bloquer les raccourcis app (Ctrl+Z, Ctrl+Y pour Undo/Redo)
  - [x] Bloquer les global shortcuts configurés par l'utilisateur:
    - [x] `config.masterStopShortcut` (Master Stop)
    - [x] `config.autoMomentumShortcut` (Auto-Momentum Toggle)
    - [x] `config.keyDetectionShortcut` (Key Detection Toggle)
  - [x] Bloquer les raccourcis système (Ctrl+C/V/X/A/S/W/Q/N/T, Alt+F4)
  - [x] Warning (pas blocage) pour Ctrl+1-9 (tabs) et Alt+lettre (menus Windows)
  - [x] Retourner un objet `{ type: 'error'|'warning', message, conflictWith }`
  - [x] Message explicite: "Reserved for Undo", "System shortcut (Copy)"
  **✅ Complété** - `checkShortcutConflicts()` ajoutée dans keyMapping.ts

- [x] **8.2.7** Refonte UI AddSoundModal pour key assignment
  - [x] Créer composant `KeyCaptureSlot` réutilisable (click → capture mode → press keys)
  - [x] Remplacer l'input texte "aze" par une liste de slots de capture
  - [x] Chaque slot capture une combinaison de touches (ex: Ctrl+A, Shift+F1)
  - [x] Bouton "+" pour ajouter un slot si `nombre de keys < nombre de sons`
  - [x] Bouton "×" pour supprimer un slot
  - [x] Preview du cycling en temps réel (Sound 1 → Key 1, Sound 2 → Key 2, Sound 3 → Key 1...)
  - [x] Même pattern de capture que les global shortcuts dans Settings
  - [x] Afficher erreurs/warnings de `checkShortcutConflicts()` lors de la capture
  - [x] UI feedback: message rouge pour erreur, orange pour warning
  **✅ Complété** - `KeyCaptureSlot.tsx` créé, AddSoundModal refactoré

- [x] **8.2.8** Mise à jour KeyGrid et SoundDetails pour afficher les combinaisons
  - [x] Gérer les noms plus longs ("Ctrl+Shift+A" vs "A")
  - [x] Truncate avec max-width et tooltip au survol dans KeyGrid
  - [x] SoundDetails: capture avec support modifiers (keydown/keyup pattern)
  - [x] Affichage en temps réel des touches pressées dans les boutons Move/Change Key
  **✅ Complété** - KeyGrid et SoundDetails mis à jour

### 8.3 Système Undo/Redo ✅

Implémenter Ctrl+Z (Undo) et Ctrl+Y (Redo) pour les modifications de profil.

- [x] **8.3.1** Créer le store d'historique `historyStore.ts`
  - [x] Définir le type `HistoryEntry` (timestamp, action, previousState, newState)
  - [x] Stack `past: HistoryEntry[]` pour undo
  - [x] Stack `future: HistoryEntry[]` pour redo
  - [x] Limite de 50 entrées maximum (éviter la mémoire excessive)
  - [x] Actions: `pushState(entry)`, `undo()`, `redo()`, `clear()`
  - [x] Helpers: `captureProfileState()`, `applyHistoryState()`
  **✅ Complété** - Store d'historique complet créé

- [x] **8.3.2** Définir les actions annulables
  - [x] Suppression de son (`removeSound`)
  - [x] Suppression de binding (`removeKeyBinding`)
  - [x] Suppression de track (`removeTrack`)
  - [x] Modification de binding (loopMode, name, soundIds, trackId)
  - [x] Modification de son (volume, momentum, nom)
  - [x] Ajout de son/track/binding
  - [x] **Non annulable**: création de profil, suppression de profil, téléchargements YouTube, currentIndex playback
  **✅ Complété** - Actions annulables identifiées et filtrées

- [x] **8.3.3** Intégrer avec `profileStore.ts`
  - [x] Avant chaque action annulable, capturer l'état via `captureProfileState()`
  - [x] Après l'action, pusher l'entrée dans l'historique
  - [x] `undo()`: restaurer l'état précédent, déplacer l'entrée vers `future`
  - [x] `redo()`: restaurer l'état suivant, déplacer l'entrée vers `past`
  - [x] Clear history au changement de profil
  **✅ Complété** - Intégration complète avec profileStore

- [x] **8.3.4** Implémenter les raccourcis clavier
  - [x] Créer `useUndoRedo.ts` hook
  - [x] Ctrl+Z / Cmd+Z → undo
  - [x] Ctrl+Y / Cmd+Shift+Z → redo
  - [x] Désactiver quand un champ de texte est focus
  - [x] Feedback toast: "Undo: {action}" / "Redo: {action}"
  - [x] Sauvegarde automatique du profil après undo/redo
  **✅ Complété** - Hook créé et intégré dans App.tsx

- [ ] **8.3.5** Indicateur visuel (optionnel)
  - [ ] Griser Undo si `past` est vide
  - [ ] Griser Redo si `future` est vide
  - [ ] Possibilité d'afficher le nom de la prochaine action annulable dans un tooltip
  **⏳ Optionnel** - Non implémenté (UI non alourdie)

### 8.4 Multi-Key Chords ✅ COMPLETE

Permettre des combinaisons de touches non-modifier pressées simultanément (comme un accord de piano).
Système inspiré des combos de jeux de combat (Street Fighter, Tekken).

> **Voir aussi**: `docs/PHASE_8_COMBINED_SHORTCUTS_PLAN.md` section 3 pour les détails complets.

**Principe : Arbre préfixe (Trie) + Trigger intelligent**
- Trigger immédiat si le combo actuel est une "feuille" (pas d'extensions possibles)
- Sinon attendre timer ou prochaine touche
- Latence 0ms pour les touches sans extensions possibles

**Exemple avec bindings A, A+Z, A+Z+E :**
```
A pressé → Extensions possibles (A+Z, A+Z+E) → Timer 30ms démarre
Z pressé → Extensions possibles (A+Z+E) → Timer continue
E pressé → Feuille (pas de A+Z+E+*) → TRIGGER IMMÉDIAT "A+Z+E"
```

- [x] **8.4.1** Implémenter la structure Trie (arbre préfixe)
  - [x] Construire le Trie à partir des keyBindings du profil
  - [x] Reconstruire le Trie quand le profil change
  - [x] Méthodes: `find(combo)`, `is_leaf(combo)`, `has_extensions(combo)`

- [x] **8.4.2** Implémenter le ChordDetector dans `detector.rs`
  - [x] Tracker `current_combo: Vec<String>` (touches pressées, triées)
  - [x] Sur key press: ajouter à combo, vérifier si feuille → trigger ou timer
  - [x] Sur timer expire: trigger le meilleur match actuel
  - [x] Sur key release: retirer de combo

- [x] **8.4.3** Fenêtre de détection configurable
  - [x] Nouveau champ `config.chordWindowMs: u32` (défaut: 30ms)
  - [x] Range: 20-100ms dans les Settings
  - [x] Timer reset à chaque nouvelle touche pressée

- [x] **8.4.4** Optimisation latence conditionnelle
  - [x] 0ms si la touche est une feuille (pas d'extensions dans le profil)
  - [x] 0ms si le combo actuel est une feuille (trigger immédiat)
  - [x] Timer seulement si des extensions sont possibles

- [x] **8.4.5** Format et normalisation des combos
  - [x] Ordre: Modifiers d'abord (Ctrl > Shift > Alt), puis base keys alphabétiques
  - [x] "KeyZ+KeyA" → normalisé en "KeyA+KeyZ"
  - [x] "Ctrl+KeyZ+KeyA" → "Ctrl+KeyA+KeyZ"

- [x] **8.4.6** UI pour capturer les multi-key chords
  - [x] KeyCaptureSlot: déjà supporte multi-key via pressedKeysRef
  - [x] Afficher preview: "A + Z" pendant la capture
  - [x] `keyCodeToDisplay("KeyA+KeyZ")` → "A+Z"

- [x] **8.4.7** Frontend `useKeyDetection.ts`
  - [x] Parser les combos multi-key reçus du backend
  - [x] Chercher le binding correspondant dans le profil

**Avantage combinatoire:**
| Type | Combinaisons (~50 touches) |
|------|----------------------------|
| 1 touche | 50 |
| 2 touches | 1,225 |
| 3 touches | 19,600 |
| + Modifiers (×8) | ×8 pour chaque |

### 8.5 Modificateur Momentum Configurable ⏳ EN DISCUSSION

Permettre à l'utilisateur de choisir quel modificateur déclenche le momentum.

> **Note**: En discussion. Cette feature résoudrait le problème Numpad+Shift (limitation hardware où Shift+Numpad4 envoie ArrowLeft au lieu de Numpad4).

- [ ] **8.5.1** Ajouter le champ config `momentumModifier`
  - [ ] Type: `"Shift" | "Alt" | "Ctrl" | "None"`
  - [ ] Défaut: "Shift" (comportement actuel)
  - [ ] "None" = momentum désactivé (utiliser Auto-Momentum toggle)

- [ ] **8.5.2** Ajouter dropdown dans Settings
  - [ ] Sous la section "Key Detection"
  - [ ] Label: "Momentum Modifier"
  - [ ] Options: Shift (default), Alt, Ctrl, None

- [ ] **8.5.3** Mettre à jour backend (`detector.rs`)
  - [ ] Lire le modifier configuré au lieu de hardcoder Shift
  - [ ] `with_momentum: bool` basé sur le modifier configuré

- [ ] **8.5.4** Mettre à jour frontend (`useKeyDetection.ts`)
  - [ ] Lire `config.momentumModifier`
  - [ ] Vérifier le modifier correspondant pour déclencher momentum

**Options:**
| Modifier | Avantage | Inconvénient |
|----------|----------|--------------|
| Shift (défaut) | Intuitif | Conflit Numpad |
| Alt | Fonctionne partout | Moins naturel |
| Ctrl | Fonctionne partout | Conflits système possibles |
| None | Simple | Perd la flexibilité |

---

## Phase 9 - Polish & Optimisations

### 9.1 Optimisations Audio

- [x] **9.1.1** Optimiser le seeking/momentum ✅
  - [x] Remplacé rodio skip_duration (O(n) lent) par symphonia seeking (O(1) instantané)
  - [x] Créé SymphoniaSource custom implémentant rodio::Source
  - [x] Supprimé le système de pre-caching (momentum_cache, momentum_source) devenu inutile
  - [x] Audio thread: timeout dynamique (200ms idle, 16ms quand actif) pour réduire CPU
  **✅ Complété** - Latence de lecture négligeable à n'importe quelle position

- [x] **9.1.2** Optimiser le chargement du profil ✅
  - [x] Calcul batch des durées via preload_profile_sounds (2 threads parallèles)
  - [x] Ne traite que les sons dont la durée est manquante (duration == 0)
  - [x] Utilise std::thread::scope pour le parallélisme contrôlé
  **✅ Complété** - Chargement rapide sans CPU spike

- [x] **9.1.3** Seamless Audio Device Switching ✅
  - [x] Store `file_path: Option<String>` in AudioTrack to enable resume
  - [x] Create `TrackResumeInfo` struct (track_id, sound_id, file_path, position, volumes)
  - [x] Capture playback state before device switch (position via elapsed time)
  - [x] Rebuild OutputStream on new device, then resume all tracks at captured positions
  - [x] No `SoundEnded` events emitted during switch (frontend sees continuous playback)
  - [x] Works for both `SetAudioDevice` command (Settings dropdown) and device polling (OS default change)
  **✅ Complété** - Sounds continue playing on new device with <50ms gap

- [ ] **9.1.4** Optimiser le crossfade
  - [ ] Profiler les performances du crossfade
  - [ ] Optimiser les calculs de volume
  - [ ] Tester avec différentes durées de crossfade

### 9.2 Optimisations UI

- [ ] **9.2.1** Optimiser le rendering React
  - [ ] Utiliser React.memo pour les composants fréquemment re-rendus
  - [ ] Utiliser useMemo et useCallback pour éviter les recalculs
  - [ ] Profiler avec React DevTools
  - [ ] Optimiser les listes (virtualisation si nécessaire)

- [ ] **9.2.2** Optimiser les animations
  - [ ] Utiliser CSS transitions plutôt que JS animations
  - [ ] Optimiser les animations de progress bar
  - [ ] Utiliser transform et opacity pour les animations (GPU-accelerated)

- [ ] **9.2.3** Lazy loading des modals
  - [ ] Charger les modals uniquement quand ouvertes
  - [ ] Utiliser React.lazy et Suspense si applicable

### 9.3 Sauvegarde Automatique

- [ ] **9.3.1** Implémenter le debouncing pour auto-save
  - [ ] Créer un `AutoSaver` dans le backend
  - [ ] Attendre 1 seconde après la dernière modification avant de sauvegarder
  - [ ] Éviter les sauvegardes excessives

- [ ] **9.3.2** Implémenter la sauvegarde périodique
  - [ ] Timer qui sauvegarde toutes les 5 minutes
  - [ ] Sauvegarder uniquement si des changements ont eu lieu
  - [ ] Logger les sauvegardes pour debug

- [ ] **9.3.3** Sauvegarder à la fermeture
  - [ ] Hook Tauri `on_window_event` pour CloseRequested
  - [ ] Sauvegarder le profil actuel
  - [ ] Sauvegarder la config
  - [ ] Attendre la fin des sauvegardes avant de fermer

### 9.4 UX Improvements

- [ ] **9.4.1** Indicateurs de chargement
  - [ ] Spinner lors du chargement d'un profil
  - [ ] Skeleton loaders pour les composants
  - [ ] Progress bar pour les téléchargements YouTube

- [ ] **9.4.2** Feedback visuel
  - [ ] Animation au click des boutons
  - [ ] Highlight des touches quand pressées
  - [ ] Animation du volume slider
  - [ ] Pulsation de l'icône de lecture

- [ ] **9.4.3** Keyboard shortcuts
  - [ ] Implémenter des raccourcis clavier pour l'UI
  - [ ] Ctrl+N: Nouveau profil
  - [ ] Ctrl+S: Sauvegarder (manuel)
  - [ ] Ctrl+E: Export
  - [ ] Ctrl+I: Import
  - [ ] ESC: Fermer le modal actif
  - [ ] Documenter les raccourcis

- [ ] **9.4.4** Drag & Drop amélioré
  - [ ] Animation au drag over
  - [ ] Preview des fichiers draggés
  - [ ] Feedback visuel pendant le drop

### 9.5 Accessibilité

- [ ] **9.5.1** ARIA labels
  - [ ] Ajouter aria-label sur tous les boutons sans texte
  - [ ] Ajouter aria-describedby pour les tooltips
  - [ ] Assurer la navigation au clavier

- [ ] **9.5.2** Focus management
  - [ ] Focus automatique sur les inputs de modals
  - [ ] Retour du focus après fermeture de modal
  - [ ] Focus visible (outline)

- [ ] **9.5.3** Contraste et lisibilité
  - [ ] Vérifier le contraste des couleurs (WCAG AA minimum)
  - [ ] Tester avec des outils d'accessibilité
  - [ ] Assurer une taille de police lisible

### 9.6 Documentation Utilisateur

- [ ] **9.6.1** Créer un README.md
  - [ ] Description du projet
  - [ ] Fonctionnalités principales
  - [ ] Installation (pour utilisateurs)
  - [ ] Prérequis (yt-dlp)
  - [ ] Screenshots
  - [ ] FAQ

- [ ] **9.6.2** Créer un guide utilisateur
  - [ ] Comment créer un profil
  - [ ] Comment ajouter des sons
  - [ ] Comment assigner des touches
  - [ ] Comment utiliser le momentum
  - [ ] Comment gérer les pistes
  - [ ] Comment télécharger depuis YouTube
  - [ ] Comment importer/exporter

- [ ] **9.6.3** Tooltips dans l'UI
  - [ ] Ajouter des tooltips sur les éléments complexes
  - [ ] Expliquer le momentum
  - [ ] Expliquer les loop modes
  - [ ] Expliquer le crossfade

### 9.7 Configuration Avancée

- [ ] **9.7.1** Exporter les settings vers un fichier
  - [ ] Permettre l'export de la config globale
  - [ ] Permettre l'import de config

- [ ] **9.7.2** Reset aux valeurs par défaut
  - [ ] Bouton "Reset to Default" dans Settings
  - [ ] Confirmation avant reset
  - [ ] Appliquer AppConfig::default()

---

## Phase 10 - Tests & Validation

### 10.1 Tests Backend (Rust)

- [ ] **10.1.1** Tests unitaires pour types
  - [ ] Tester les sérialisations/désérialisations JSON
  - [ ] Tester les valeurs par défaut (AppConfig::default)
  - [ ] Tester les validations

- [ ] **10.1.2** Tests unitaires pour storage
  - [ ] Tester load_config/save_config
  - [ ] Tester create_profile/load_profile/save_profile/delete_profile
  - [ ] Tester avec des données invalides
  - [ ] Tester les cas d'erreur (fichier manquant, JSON corrompu)

- [ ] **10.1.3** Tests unitaires pour audio
  - [ ] Tester les calculs de volume final
  - [ ] Tester la logique de sélection des sons (loop modes)
  - [ ] Tester la courbe de crossfade (get_volumes)
  - [ ] Tester le cooldown

- [ ] **10.1.4** Tests unitaires pour keys
  - [ ] Tester key_to_code et code_to_key
  - [ ] Tester is_shortcut_pressed
  - [ ] Tester la détection des modificateurs

- [ ] **10.1.5** Tests unitaires pour YouTube
  - [ ] Tester is_valid_youtube_url
  - [ ] Tester extract_video_id
  - [ ] Tester sanitize_filename
  - [ ] Tester la logique de cache (mock)

- [ ] **10.1.6** Tests unitaires pour import/export
  - [ ] Tester l'export d'un profil (mock filesystem)
  - [ ] Tester l'import d'un profil
  - [ ] Tester avec des données invalides

- [ ] **10.1.7** Tests d'intégration
  - [ ] Tester le flow complet: create profile → add sound → save → load
  - [ ] Tester le flow audio: play sound → crossfade → stop
  - [ ] Tester le flow YouTube: download → cache → play

### 10.2 Tests Frontend (React)

- [ ] **10.2.1** Tests unitaires pour utils
  - [ ] Tester formatDuration
  - [ ] Tester formatFileSize
  - [ ] Tester keyCodeToDisplay
  - [ ] Tester parseKeyCombination

- [ ] **10.2.2** Tests unitaires pour stores
  - [ ] Tester les actions de audioStore
  - [ ] Tester les actions de profileStore
  - [ ] Tester les actions de settingsStore

- [ ] **10.2.3** Tests de composants
  - [ ] Tester les composants simples (Header, Sidebar)
  - [ ] Tester les interactions (clicks, inputs)
  - [ ] Utiliser React Testing Library
  - [ ] Mock les commandes Tauri

- [ ] **10.2.4** Tests d'intégration frontend
  - [ ] Tester les flows complets (create profile → add sound)
  - [ ] Tester les modals (open → input → save → close)

### 10.3 Tests Manuels

- [ ] **10.3.1** Test des fonctionnalités audio
  - [ ] Tester la lecture de chaque format (MP3, WAV, OGG, FLAC)
  - [ ] Tester le crossfade avec différentes durées
  - [ ] Tester le momentum
  - [ ] Tester les loop modes (off, random, single, sequential)
  - [ ] Tester le volume (master, track, sound individual)
  - [ ] Tester avec plusieurs pistes simultanées
  - [ ] Tester avec des fichiers longs (2-3 heures)

- [ ] **10.3.2** Test des touches
  - [ ] Tester la détection en arrière-plan (fenêtre non focusée)
  - [ ] Tester le cooldown
  - [ ] Tester le Master Stop
  - [ ] Tester avec Shift pour momentum
  - [ ] Tester la désactivation lors du focus d'input

- [ ] **10.3.3** Test des profils
  - [ ] Créer plusieurs profils
  - [ ] Basculer entre profils
  - [ ] Renommer/supprimer des profils
  - [ ] Sauvegarder/charger

- [ ] **10.3.4** Test YouTube
  - [ ] Télécharger plusieurs vidéos
  - [ ] Vérifier le cache
  - [ ] Tester avec des URLs invalides
  - [ ] Tester avec des vidéos privées/indisponibles
  - [ ] Tester le progress

- [ ] **10.3.5** Test Import/Export
  - [ ] Exporter un profil
  - [ ] Vérifier le contenu du .ktm (unzip)
  - [ ] Importer sur la même machine
  - [ ] Importer sur une autre machine (si possible)
  - [ ] Tester avec des profils complexes (nombreux sons, tracks)

- [ ] **10.3.6** Test des erreurs
  - [ ] Supprimer un fichier audio référencé → vérifier le modal
  - [ ] Tester l'update du chemin
  - [ ] Tester la suppression du son
  - [ ] Vérifier que error.mp3 joue
  - [ ] Tester avec yt-dlp non installé

### 10.4 Tests Multi-Plateformes

- [ ] **10.4.1** Tests sur Windows
  - [ ] Compiler et lancer l'app
  - [ ] Tester toutes les fonctionnalités
  - [ ] Vérifier les chemins de fichiers (backslashes)
  - [ ] Tester le system tray
  - [ ] Tester l'installeur

- [ ] **10.4.2** Tests sur macOS
  - [ ] Compiler et lancer l'app
  - [ ] Tester toutes les fonctionnalités
  - [ ] Vérifier les permissions (keyboard access)
  - [ ] Tester le system tray
  - [ ] Tester le .dmg

- [ ] **10.4.3** Tests sur Linux
  - [ ] Compiler et lancer l'app sur Ubuntu
  - [ ] Tester sur Fedora (si possible)
  - [ ] Tester sur Arch (si possible)
  - [ ] Vérifier les permissions
  - [ ] Tester le system tray (peut varier selon le DE)
  - [ ] Tester les packages (.deb, .AppImage)

### 10.5 Tests de Performance

- [ ] **10.5.1** Benchmark audio
  - [ ] Mesurer la latence de déclenchement
  - [ ] Mesurer l'utilisation CPU pendant la lecture
  - [ ] Mesurer l'utilisation mémoire avec plusieurs pistes
  - [ ] Identifier les bottlenecks

- [ ] **10.5.2** Benchmark UI
  - [ ] Mesurer le temps de rendu des composants
  - [ ] Profiler avec React DevTools
  - [ ] Identifier les re-renders inutiles

- [ ] **10.5.3** Stress tests
  - [ ] Tester avec 20 pistes simultanées
  - [ ] Tester avec 100+ sons dans un profil
  - [ ] Tester avec des fichiers très longs (10+ heures)
  - [ ] Tester le spam de touches (ignorer cooldown en test)

### 10.6 Tests de Sécurité

- [ ] **10.6.1** Valider les inputs utilisateur
  - [ ] Vérifier que les chemins de fichiers sont sûrs (pas d'injection)
  - [ ] Vérifier que les URLs YouTube sont validées
  - [ ] Vérifier les limites de taille (noms, durées, etc.)

- [ ] **10.6.2** Tester les permissions Tauri
  - [ ] Vérifier que seules les permissions nécessaires sont activées
  - [ ] Tester l'accès filesystem (scope limité à AppData)

### 10.7 Validation Finale

- [ ] **10.7.1** Checklist des fonctionnalités
  - [ ] Toutes les fonctionnalités de la spec sont implémentées
  - [ ] Toutes les commandes Tauri sont fonctionnelles
  - [ ] Tous les events sont émis correctement
  - [ ] Toutes les erreurs sont gérées

- [ ] **10.7.2** Checklist UX
  - [ ] L'UI est cohérente et intuitive
  - [ ] Les animations sont fluides
  - [ ] Les messages d'erreur sont clairs
  - [ ] Les tooltips sont présents où nécessaire

- [ ] **10.7.3** Checklist de polish
  - [ ] Pas de console.log en production
  - [ ] Pas de TODOs dans le code
  - [ ] Code formaté (rustfmt, prettier)
  - [ ] Pas de warnings de compilation

---

## Phase 11 - Build & Release (Bonus)

### 11.1 Préparation du Build

- [ ] **11.1.1** Configurer les icônes
  - [ ] Créer toutes les tailles d'icônes requises
  - [ ] Optimiser les icônes

- [ ] **11.1.2** Configurer le bundle
  - [ ] Vérifier tauri.conf.json (identifier, version, etc.)
  - [ ] Configurer les targets (Windows, macOS, Linux)
  - [ ] Configurer les ressources à inclure

- [ ] **11.1.3** Optimiser le build
  - [ ] Build en mode release
  - [ ] Vérifier la taille du bundle
  - [ ] Strip les symboles de debug si nécessaire

### 11.2 Build par Plateforme

- [ ] **11.2.1** Build Windows
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .exe et l'installeur .msi
  - [ ] Tester l'installation

- [ ] **11.2.2** Build macOS
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .app et le .dmg
  - [ ] Signer l'app (si certificat disponible)
  - [ ] Tester l'installation

- [ ] **11.2.3** Build Linux
  - [ ] `npm run tauri build`
  - [ ] Vérifier le .deb et .AppImage
  - [ ] Tester l'installation sur Ubuntu

### 11.3 Documentation de Release

- [ ] **11.3.1** Changelog
  - [ ] Lister toutes les fonctionnalités
  - [ ] Lister les bugs connus (si applicable)

- [ ] **11.3.2** Instructions d'installation
  - [ ] Documenter l'installation pour chaque plateforme
  - [ ] Documenter l'installation de yt-dlp

- [ ] **11.3.3** Licence
  - [ ] Choisir une licence (MIT, GPL, etc.)
  - [ ] Ajouter LICENSE file

### 11.4 Distribution

- [ ] **11.4.1** Hébergement des releases
  - [ ] GitHub Releases
  - [ ] Uploader les binaires pour chaque plateforme

- [ ] **11.4.2** Auto-update (optionnel)
  - [ ] Configurer Tauri updater
  - [ ] Héberger le fichier de métadonnées d'update

---

## Récapitulatif des Tâches

**Total estimé : 400+ tâches**

- Phase 0: ~20 tâches (Initialisation)
- Phase 1: ~35 tâches (Fondations Backend)
- Phase 2: ~60 tâches (Moteur Audio)
- Phase 3: ~25 tâches (Détection Touches)
- Phase 4: ~120 tâches (Interface Utilisateur)
- Phase 5: ~25 tâches (YouTube)
- Phase 6: ~20 tâches (Import/Export)
- Phase 7: ~20 tâches (Gestion Erreurs)
- Phase 8: ~30 tâches (Polish & Optimisations)
- Phase 9: ~40 tâches (Tests & Validation)
- Phase 10: ~15 tâches (Build & Release)

---

## Notes

- Chaque tâche peut être décomposée en sous-tâches plus granulaires selon les besoins
- L'ordre des phases est recommandé mais peut être adapté
- Certaines tâches peuvent être réalisées en parallèle
- Les tests devraient être écrits au fur et à mesure, pas seulement en Phase 9
- Cette liste est exhaustive basée sur la spécification technique fournie

---

**Document généré le:** 2026-01-20
**Basé sur:** KeyToMusic_Technical_Specification.md v1.0.0
