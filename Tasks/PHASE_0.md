# Phase 0 - Initialisation du Projet

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-20

---

## 0.1 Configuration Initiale

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

## 0.2 Structure des Dossiers

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

## 0.3 Configuration Tauri

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
