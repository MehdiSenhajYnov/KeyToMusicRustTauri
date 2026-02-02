# KeyToMusic

Application de soundboard conçue pour accompagner la lecture de mangas avec des musiques/OST adaptées à l'ambiance des planches.

## Fonctionnalités Principales

- Détection globale des touches clavier (fonctionne en arrière-plan)
- Assignation de sons à des touches avec support multi-sons par touche
- Système de pistes multiples (jusqu'à 20) pour superposer différents types de sons
- Crossfade fluide entre les sons d'une même piste
- Mode Momentum pour démarrer les sons à une position spécifique
- Modes de boucle variés (Off, Random, Single, Sequential)
- Téléchargement de sons depuis YouTube avec système de cache
- Recherche YouTube intégrée et import de playlists
- Multi-profils/playlists sauvegardables avec duplication
- Import/Export de configurations (.ktm) + import legacy (Unity)
- Sélection du périphérique audio avec reprise transparente
- Visualisation waveform avec marqueur de momentum
- Système de découverte (YouTube Mix) avec pré-téléchargement et auto-assignation
- Multi-touches combinées (accords style jeu de combat)
- Undo/Redo (Ctrl+Z / Ctrl+Y)
- Modificateur momentum configurable (Shift/Ctrl/Alt)

## Stack Technique

- **Framework** : Tauri 2.x (Rust + React)
- **Frontend** : React 18 + TypeScript + Tailwind CSS + Zustand
- **Backend** : Rust avec rodio/cpal/symphonia (audio)
- **Détection touches** : Windows Raw Input API, macOS CGEventTap, Linux rdev
- **Outils externes** : yt-dlp + ffmpeg (auto-téléchargés dans `data/bin/`)

## Installation

### Prérequis

1. **Node.js** (v18 ou supérieur) et npm
2. **Rust** (dernière version stable)

> **Note :** yt-dlp et ffmpeg sont téléchargés automatiquement par l'application au premier lancement. Aucune installation manuelle n'est nécessaire.

### Étapes d'installation

1. Cloner le dépôt
```bash
git clone <url-du-repo>
cd KeyToMusicRust
```

2. Installer les dépendances npm
```bash
npm install
```

3. Ajouter les ressources (optionnel)
   - Placer un fichier `error.mp3` dans `resources/sounds/`
   - Générer les icônes ou placer des icônes dans `resources/icons/`

4. Lancer en mode développement
```bash
npm run tauri dev
```

## Commandes de développement

```bash
# Développement avec hot-reload
npm run tauri dev

# Build production
npm run tauri build

# Lint Rust
cargo clippy --manifest-path src-tauri/Cargo.toml

# Format Rust
cargo fmt --manifest-path src-tauri/Cargo.toml

# Tests Rust
cargo test --manifest-path src-tauri/Cargo.toml
```

## Structure du Projet

```
keytomusic/
├── src/                    # Frontend React/TypeScript
│   ├── components/         # Composants UI (Layout, Tracks, Sounds, Keys, Discovery, Errors, common)
│   ├── stores/             # State management (10 Zustand stores)
│   ├── hooks/              # useAudioEvents, useKeyDetection, useDiscovery, useTrackPosition, etc.
│   ├── types/              # Types TypeScript
│   └── utils/              # tauriCommands, keyMapping, profileAnalysis, errorMessages, etc.
├── src-tauri/              # Backend Rust
│   └── src/
│       ├── audio/          # Moteur audio, crossfade, waveform analysis
│       ├── keys/           # Détection touches, accords multi-touches
│       ├── discovery/      # YouTube Mix discovery engine
│       ├── youtube/        # Téléchargement YT, recherche, playlists
│       ├── import_export/  # Import/Export .ktm
│       └── storage/        # Persistance profils & config
├── data/                   # Runtime: profiles/, cache/, discovery/, bin/, imported_sounds/, logs/
└── resources/              # Ressources statiques (icônes, error.mp3)
```

## État du Projet

| Phase | Nom | Statut |
|-------|-----|--------|
| 0 | Initialisation du Projet | Complété |
| 1 | Fondations Backend (Rust) | Complété |
| 2 | Moteur Audio | Complété |
| 3 | Détection des Touches | Complété |
| 4 | Interface Utilisateur (React) | Complété |
| 4.5 | Bug Fixes & Améliorations | Complété |
| 4.6 | UX Enhancements & Key Management | Complété |
| 5 | Téléchargement YouTube | Complété |
| 6 | Import/Export | Complété |
| 6.5 | Concurrent Downloads & Key Cycling | Complété |
| 7 | Gestion des Erreurs | Complété |
| 7.5 | Legacy Import | Complété |
| 8 | Nouvelles Features (Chords, Undo/Redo, Momentum) | Complété |
| SD | Smart Discovery & Auto-Setup | Complété |
| 9 | Polish & Optimisations | Partiel |
| 10 | Tests & Validation | Planifié |
| 11 | Build & Release | Planifié |

Voir `Tasks/README.md` pour le suivi détaillé des tâches.

## Documentation

- `CLAUDE.md` : Guide pour Claude Code (architecture complète)
- `Tasks/README.md` : Suivi des tâches de développement
- `KeyToMusic_Technical_Specification.md` : Spécification technique complète

## Licence

À définir

## Auteurs

KeyToMusic Team
