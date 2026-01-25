# Phase 2 - Moteur Audio

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-23

---

## 2.1 Structures Audio de Base

- [x] **2.1.1** Créer `src-tauri/src/audio/mod.rs`
  - [x] Définir la structure du module audio
  - [x] Exporter engine, track, crossfade, buffer, symphonia_source
  **✅ Complété** - Module avec exports de tous les sous-modules

- [x] **2.1.2** Types audio intégrés dans les modules respectifs
  - [x] AudioCommand et AudioEvent dans engine.rs
  - [x] CrossfadeState dans crossfade.rs
  - [x] AudioMetadata et BufferedSound dans buffer.rs
  **✅ Complété** - Types définis directement dans leurs modules

## 2.2 Moteur Audio Principal

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

## 2.3 Gestion des Pistes

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

## 2.4 Système de Crossfade

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

## 2.5 Système de Seeking et Streaming (Symphonia)

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

## 2.6 Logique de Lecture Avancée

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

## 2.7 Commandes Audio Tauri

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

## 2.8 Events Audio

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
