# Phase 1 - Fondations Backend (Rust)

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-23

---

## 1.1 Types et Structures de Données

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
  - [x] Ajouter les variantes pour erreurs keys (KeyAlreadyAssigned, InvalidStopAllShortcut)
  - [x] Implémenter les messages d'erreur avec #[error("...")]
  **✅ Complété** - AppError enum avec From<AppError> pour String et Serialize impl

## 1.2 Système de Stockage

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

## 1.3 Commandes Tauri de Base

- [x] **1.3.1** Créer `src-tauri/src/commands.rs` (partie config)
  - [x] Implémenter `get_config() -> Result<AppConfig, String>`
  - [x] Implémenter `update_config(updates: serde_json::Value) -> Result<(), String>`
  - [x] Implémenter la fusion partielle des updates avec la config existante
  **✅ Complété** - Merge partiel de tous les champs dont StopAllShortcut et currentProfileId (nullable)

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

## 1.4 State Management Rust

- [x] **1.4.1** Créer `src-tauri/src/state.rs`
  - [x] Définir `AppState` avec Mutex<> pour config
  - [x] Implémenter des méthodes helper pour accéder au state de manière thread-safe
  - [x] Gérer l'initialisation du state dans main.rs
  **✅ Complété** - AppState avec get_config() et update_config() thread-safe, managé via Tauri .manage()
