# Phase 9 - Polish & Optimisations

> **Statut:** 🔄 EN COURS (partiel)

---

## 9.1 Optimisations Audio

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

## 9.2 Optimisations UI

- [x] **9.2.1** Optimiser le rendering React ✅ (partiel)
  - [x] KeyGrid re-render optimization: `usePlayingSoundIds()` hook extracts Set<string> of playing sound IDs with shallow comparison, prevents re-renders from position-only progress updates
  - [x] SoundDetails targeted Zustand subscription: subscribes only to specific track's playback entry instead of full `playingTracks` map
  - [x] WaveformDisplay dual-canvas: static canvas (waveform bars + markers) + cursor canvas (playback position), decouples expensive rendering from progress ticks
  - [x] Batched duration updates: `computeProfileDurations()` returns all durations at once, applied in single `set()` call instead of N individual updates
  - [x] Progress emission rate reduced from 100ms to 250ms
  - [ ] Profiler avec React DevTools (complet)
  - [ ] Optimiser les listes (virtualisation si nécessaire)

- [ ] **9.2.2** Optimiser les animations
  - [ ] Utiliser CSS transitions plutôt que JS animations
  - [ ] Optimiser les animations de progress bar
  - [ ] Utiliser transform et opacity pour les animations (GPU-accelerated)

- [ ] **9.2.3** Lazy loading des modals
  - [ ] Charger les modals uniquement quand ouvertes
  - [ ] Utiliser React.lazy et Suspense si applicable

## 9.3 Sauvegarde Automatique

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

## 9.4 UX Improvements

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

## 9.5 Accessibilité

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

## 9.6 Documentation Utilisateur

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

## 9.7 Configuration Avancée

- [ ] **9.7.1** Exporter les settings vers un fichier
  - [ ] Permettre l'export de la config globale
  - [ ] Permettre l'import de config

- [ ] **9.7.2** Reset aux valeurs par défaut
  - [ ] Bouton "Reset to Default" dans Settings
  - [ ] Confirmation avant reset
  - [ ] Appliquer AppConfig::default()
