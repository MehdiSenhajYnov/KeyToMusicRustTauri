# KeyToMusic

Application de soundboard conçue pour accompagner la lecture de mangas avec des musiques/OST adaptées à l'ambiance des planches.

## Fonctionnalités Principales

- ✅ Détection globale des touches clavier (fonctionne en arrière-plan)
- ✅ Assignation de sons à des touches avec support multi-sons par touche
- ✅ Système de pistes multiples pour superposer différents types de sons
- ✅ Crossfade fluide entre les sons d'une même piste
- ✅ Mode Momentum pour démarrer les sons à une position spécifique
- ✅ Modes de boucle variés (Off, Random, Single, Sequential)
- ✅ Téléchargement de sons depuis YouTube avec système de cache
- ✅ Multi-profils/playlists sauvegardables
- ✅ Import/Export de configurations
- ✅ Sélection du périphérique audio avec reprise transparente lors du changement

## Stack Technique

- **Framework**: Tauri 2.x (Rust + React)
- **Frontend**: React 18 + TypeScript + Tailwind CSS + Zustand
- **Backend**: Rust avec rodio (audio) et rdev (détection touches)
- **Outils externes**: yt-dlp (téléchargement YouTube)

## Installation

### Prérequis

1. **Node.js** (v18 ou supérieur) et npm
2. **Rust** (dernière version stable)
3. **yt-dlp** (pour le téléchargement YouTube)
   - Windows: `winget install yt-dlp`
   - macOS: `brew install yt-dlp`
   - Linux: `sudo apt install yt-dlp` ou équivalent

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

3. Ajouter les ressources (optionnel pour Phase 0)
   - Placer un fichier `error.wav` dans `resources/sounds/`
   - Générer les icônes ou placer des icônes dans `resources/icons/`

4. Lancer en mode développement
```bash
npm run tauri:dev
```

## Commandes de développement

```bash
# Développement avec hot-reload
npm run tauri:dev

# Build production
npm run tauri:build

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
│   ├── components/         # Composants UI
│   ├── stores/            # State management (Zustand)
│   ├── types/             # Types TypeScript
│   └── utils/             # Utilitaires
├── src-tauri/             # Backend Rust
│   └── src/
│       ├── audio/         # Moteur audio
│       ├── keys/          # Détection touches
│       ├── youtube/       # Téléchargement YT
│       └── storage/       # Persistance
└── resources/             # Ressources statiques
```

## État du Projet

- Phase 0: Initialisation ✅
- Phase 1: Fondations Backend ✅
- Phase 2: Moteur Audio ✅
- Phase 3: Détection des Touches ✅
- Phase 4: Interface Utilisateur ✅
- Phase 4.5: Bug Fixes & Améliorations ✅
- Phase 4.6: UX Enhancements & Key Management ✅
- Phase 5: Téléchargement YouTube ✅
- Phase 6: Import/Export ✅
- Phase 7: Gestion des Erreurs (en cours)
- Phase 8: Polish & Optimisations (en cours - seeking symphonia, device switching done)
- Phase 9: Tests & Validation
- Phase 10: Build & Release

Voir `TASKS.md` pour la liste complète des tâches.

## Documentation

- `CLAUDE.md` : Guide pour Claude Code
- `TASKS.md` : Liste exhaustive des tâches
- `KeyToMusic_Technical_Specification.md` : Spécification technique complète

## Licence

À définir

## Auteurs

KeyToMusic Team
