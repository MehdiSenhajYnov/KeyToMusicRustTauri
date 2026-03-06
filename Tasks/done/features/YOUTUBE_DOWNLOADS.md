# Phase 5 - Téléchargement YouTube

> **Statut:** ✅ COMPLÉTÉE
> **Date de complétion:** 2026-01-24

---

## 5.1 Module YouTube Backend

- [x] **5.1.1** Créer `src-tauri/src/youtube/mod.rs`
  - [x] Définir la structure du module
  - [x] Exporter downloader et cache

- [x] **5.1.2** Créer `src-tauri/src/youtube/cache.rs` (structures)
  - [x] Définir `CacheEntry` struct
  - [x] Définir `SoundReference` struct
  - [x] Définir `YouTubeCache` struct avec HashMap<String, CacheEntry>
  - [x] Ajouter les champs: index_path, cache_dir

## 5.2 Système de Cache

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

## 5.3 Downloader YouTube

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

## 5.4 Commandes YouTube Tauri

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

## 5.5 Gestion des Erreurs YouTube

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

## 5.6 YouTube Fixes & Improvements (2026-01-24)

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
