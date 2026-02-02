# KeyToMusic - Tasks

> Dernière mise à jour: 2026-02-02

## Organisation

```
Tasks/
├── todo/              Ce qui reste à faire
├── done/
│   ├── features/      Features implémentées (doc technique consultable)
│   ├── fixes/         Bug fixes & corrections de perf
│   └── infrastructure/  Setup projet, config, tooling
└── post-dev/          Tests & Release
```

### Où ajouter une nouvelle tâche ?

**Toujours dans `todo/`.** Un template est dispo : `todo/_TEMPLATE.md`

### Où ranger une tâche terminée ?

| La tâche...                                  | Va dans              |
|----------------------------------------------|----------------------|
| **Ajoute** quelque chose de nouveau           | `done/features/`     |
| **Corrige/améliore** un truc existant         | `done/fixes/`        |
| C'est du setup, tests, build, release        | `done/infrastructure/` |

---

## Todo

| Nom | Statut | Fichier |
|-----|--------|---------|
| Polish & Optimisations | 🔄 Partiel | [POLISH_OPTIMIZATIONS.md](./todo/POLISH_OPTIMIZATIONS.md) |
| Multi-Selection | ⏳ Planifié | [MULTI_SELECTION.md](./todo/MULTI_SELECTION.md) |
| Discovery — Sons Locaux | ⏳ Planifié | [DISCOVERY_LOCAL_SOUNDS.md](./todo/DISCOVERY_LOCAL_SOUNDS.md) |
| Discovery — Switch Profil & Preload | 🔄 Partiel (P0+P1 done, P2 optionnel) | [DISCOVERY_PROFILE_SWITCH_PRELOAD.md](./todo/DISCOVERY_PROFILE_SWITCH_PRELOAD.md) |
| Discovery — Volume de Preview | ✅ Completed | [DISCOVERY_PREVIEW_VOLUME.md](./todo/DISCOVERY_PREVIEW_VOLUME.md) |
| YouTube Search Preview (Streaming) | ✅ Completed | [YOUTUBE_SEARCH_PREVIEW.md](./todo/YOUTUBE_SEARCH_PREVIEW.md) |
| Sliders — Contrôle Molette | ⏳ Planifié | [SLIDER_MOUSE_WHEEL.md](./todo/SLIDER_MOUSE_WHEEL.md) |
| Waveform & Momentum — Perf Fix | ✅ Completed | [WAVEFORM_PERF_FIX.md](./done/fixes/WAVEFORM_PERF_FIX.md) |
| Momentum — Détection & Visibilité | ✅ Completed | [MOMENTUM_SUGGESTION_FIX.md](./done/fixes/MOMENTUM_SUGGESTION_FIX.md) |
| Startup Performance | ✅ Completed | [STARTUP_PERFORMANCE.md](./todo/STARTUP_PERFORMANCE.md) |
| Recherche & Filtre KeyGrid | ✅ Completed | [SEARCH_FILTER_KEYGRID.md](./todo/SEARCH_FILTER_KEYGRID.md) |
| Nettoyage champs Track morts | ✅ Completed | [TRACK_DEAD_FIELDS_CLEANUP.md](./done/fixes/TRACK_DEAD_FIELDS_CLEANUP.md) |
| Modal raccourcis clavier | ✅ Completed | [KEYBOARD_SHORTCUTS_MODAL.md](./done/features/KEYBOARD_SHORTCUTS_MODAL.md) |
| Discovery — Système de Dislike Permanent | ✅ Completed | [DISCOVERY_DISLIKE_SYSTEM.md](./done/features/DISCOVERY_DISLIKE_SYSTEM.md) |
| Onboarding — Empty States Progressifs | ✅ Completed | [ONBOARDING_EMPTY_STATES.md](./done/features/ONBOARDING_EMPTY_STATES.md) |
| Multi-Track par Touche | ✅ Completed | [MULTI_TRACK_PER_KEY.md](./done/features/MULTI_TRACK_PER_KEY.md) |

## Post-dev

| Nom | Statut | Fichier |
|-----|--------|---------|
| Tests & Validation | ⏳ Planifié | [TESTS_VALIDATION.md](./post-dev/TESTS_VALIDATION.md) |
| Build & Release | ⏳ Planifié | [BUILD_RELEASE.md](./post-dev/BUILD_RELEASE.md) |

---

## Done — Features

| Nom | Fichier |
|-----|---------|
| Fondations Backend (Rust) | [BACKEND_FOUNDATIONS.md](./done/features/BACKEND_FOUNDATIONS.md) |
| Moteur Audio | [AUDIO_ENGINE.md](./done/features/AUDIO_ENGINE.md) |
| Détection des Touches | [KEY_DETECTION.md](./done/features/KEY_DETECTION.md) |
| Interface Utilisateur (React) | [USER_INTERFACE.md](./done/features/USER_INTERFACE.md) |
| Téléchargement YouTube | [YOUTUBE_DOWNLOADS.md](./done/features/YOUTUBE_DOWNLOADS.md) |
| Import/Export | [IMPORT_EXPORT.md](./done/features/IMPORT_EXPORT.md) |
| Gestion des Erreurs | [ERROR_HANDLING.md](./done/features/ERROR_HANDLING.md) |
| UX & Key Management | [UX_KEY_MANAGEMENT.md](./done/features/UX_KEY_MANAGEMENT.md) |
| Concurrent Downloads & Key Cycling | [CONCURRENT_DOWNLOADS.md](./done/features/CONCURRENT_DOWNLOADS.md) |
| Legacy Import | [LEGACY_IMPORT.md](./done/features/LEGACY_IMPORT.md) |
| Chords, Undo/Redo & Momentum | [CHORDS_UNDO_MOMENTUM.md](./done/features/CHORDS_UNDO_MOMENTUM.md) |
| Smart Discovery | [SMART_DISCOVERY.md](./done/features/Smart_Discovery/SMART_DISCOVERY.md) |
| Discovery — Système de Dislike Permanent | [DISCOVERY_DISLIKE_SYSTEM.md](./done/features/DISCOVERY_DISLIKE_SYSTEM.md) |
| Onboarding — Empty States Progressifs | [ONBOARDING_EMPTY_STATES.md](./done/features/ONBOARDING_EMPTY_STATES.md) |
| Modal raccourcis clavier | [KEYBOARD_SHORTCUTS_MODAL.md](./done/features/KEYBOARD_SHORTCUTS_MODAL.md) |
| Multi-Track par Touche | [MULTI_TRACK_PER_KEY.md](./done/features/MULTI_TRACK_PER_KEY.md) |

## Done — Fixes

| Nom | Fichier |
|-----|---------|
| Bug Fixes & Améliorations | [BUG_FIXES.md](./done/fixes/BUG_FIXES.md) |
| Audio MicroFreeze Fix | [AUDIO_MICROFREEZE_FIX.md](./done/fixes/Audio_MicroFreeze_Fix/AUDIO_MICROFREEZE_FIX.md) |
| Momentum — Détection & Visibilité | [MOMENTUM_SUGGESTION_FIX.md](./done/fixes/MOMENTUM_SUGGESTION_FIX.md) |
| Waveform & Momentum — Perf Fix | [WAVEFORM_PERF_FIX.md](./done/fixes/WAVEFORM_PERF_FIX.md) |
| Nettoyage champs Track morts | [TRACK_DEAD_FIELDS_CLEANUP.md](./done/fixes/TRACK_DEAD_FIELDS_CLEANUP.md) |

## Done — Infrastructure

| Nom | Fichier |
|-----|---------|
| Initialisation du Projet | [PROJECT_SETUP.md](./done/infrastructure/PROJECT_SETUP.md) |

---

## Historique

- **2026-01-20** — Setup projet
- **2026-01-23** — Backend, Audio, Key Detection, UI, Bug Fixes, UX
- **2026-01-24** — YouTube, Import/Export, Concurrent Downloads, Error Handling, Legacy Import
- **2026-01-25** — Chords, Undo/Redo, Momentum Modifier
- **2026-01-31** — Smart Discovery
- **2026-02-01** — Audio MicroFreeze Fix
- **2026-02-02** — YouTube Search Preview (Streaming)
- **2026-02-02** — Momentum Suggestion Detection & UX Overhaul
- **2026-02-02** — Waveform & Momentum Performance Fix
- **2026-02-02** — Startup Performance Optimization (7 phases)
- **2026-02-02** — Track dead fields cleanup (removed currentlyPlaying, playbackPosition, isPlaying)
- **2026-02-02** — Discovery Dislike System (permanent blacklist with Settings UI management)
- **2026-02-02** — Onboarding Empty States (progressive CTAs for no-profile, no-tracks, no-bindings)
- **2026-02-02** — Keyboard Shortcuts Modal (? button in header, ? / F1 global shortcut, dynamic config shortcuts)
- **2026-02-02** — Multi-Track par Touche (one key triggers sounds on multiple tracks simultaneously)
