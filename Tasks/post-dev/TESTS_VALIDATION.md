# Phase 10 - Tests & Validation

> **Statut:** ⏳ À FAIRE

---

## 10.1 Tests Backend (Rust)

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

## 10.2 Tests Frontend (React)

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

## 10.3 Tests Manuels

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

## 10.4 Tests Multi-Plateformes

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

## 10.5 Tests de Performance

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

## 10.6 Tests de Sécurité

- [ ] **10.6.1** Valider les inputs utilisateur
  - [ ] Vérifier que les chemins de fichiers sont sûrs (pas d'injection)
  - [ ] Vérifier que les URLs YouTube sont validées
  - [ ] Vérifier les limites de taille (noms, durées, etc.)

- [ ] **10.6.2** Tester les permissions Tauri
  - [ ] Vérifier que seules les permissions nécessaires sont activées
  - [ ] Tester l'accès filesystem (scope limité à AppData)

## 10.7 Validation Finale

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
