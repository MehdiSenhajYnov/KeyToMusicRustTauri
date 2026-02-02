# Phase 7 - Gestion des Erreurs

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-24

---

## 7.1 Son d'Erreur

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

## 7.2 Messages d'Erreur Utilisateur

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

## 7.3 Vérification des Fichiers au Chargement

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

## 7.4 Gestion du Cache Corrompu

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

## 7.5 Logs et Debugging

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

## 7.6 AddSoundModal - File Picker & Drag-Drop UX

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
