# Smart Discovery & Auto-Setup

> **Statut:** ✅ COMPLÉTÉ
> **Date de complétion:** 2026-01-31
> **Type:** Update — Nouvelles features post-release
> **Objectif:** Transformer le setup de "30 minutes de travail manuel" en "2 minutes de clics". Ajouter la recherche YouTube intégrée, l'import de playlists, la waveform visuelle avec auto-momentum, et un système de découverte musicale basé sur les YouTube Mix (recommandations croisées).

---

## Vue d'ensemble

L'app actuelle est solide côté playback et key detection, mais le **setup** reste le gros point de friction : trouver des sons en externe, les importer un par un, configurer le momentum manuellement pour chaque son. Cette phase élimine ces frictions en 5 sous-phases indépendantes, chacune livrant de la valeur seule.

## Sous-phases

| # | Feature | Statut | Détails |
|---|---------|--------|---------|
| 1 | Recherche YouTube intégrée | ✅ Complété | [SMART_DISCOVERY_01_YouTube_Search.md](./SMART_DISCOVERY_01_YouTube_Search.md) |
| 2 | Import de playlists YouTube | ✅ Complété | [SMART_DISCOVERY_02_Playlist_Import.md](./SMART_DISCOVERY_02_Playlist_Import.md) |
| 3 | Waveform RMS (visualisation d'énergie) | ✅ Complété | [SMART_DISCOVERY_03_Waveform_RMS.md](./SMART_DISCOVERY_03_Waveform_RMS.md) |
| 4 | Auto-Momentum (marqueur suggéré) | ✅ Complété | [SMART_DISCOVERY_04_Auto_Momentum.md](./SMART_DISCOVERY_04_Auto_Momentum.md) |
| 5 | Découverte musicale (YouTube Mix croisés) | ✅ Complété | [SMART_DISCOVERY_05_YouTube_Mix_Discovery.md](./SMART_DISCOVERY_05_YouTube_Mix_Discovery.md) |

## Résumé de l'implémentation

### Section 1 - YouTube Search
- Commande backend `search_youtube(query, max_results)` via yt-dlp `ytsearch`
- Résultats avec titre, durée, channel, thumbnail
- One-click download depuis les résultats

### Section 2 - Playlist Import
- Commande backend `fetch_playlist(url)` pour metadata de playlist
- Support URL vidéo, vidéo-dans-playlist, playlist pure
- Toggle `playlistImportEnabled` dans la config

### Section 3 - Waveform RMS
- `compute_waveform_sampled()` (~40x plus rapide que full decode) dans `audio/analysis.rs`
- `WaveformDisplay.tsx` avec dual-canvas (static waveform + cursor overlay)
- WaveformCache LRU (50 entries) avec persistence disque et invalidation par mtime
- Commandes: `get_waveform()`, `get_waveforms_batch()`

### Section 4 - Auto-Momentum
- `detect_momentum_point()` analyse la waveform pour trouver le début du contenu significatif
- Skip 5% initial, cherche la première montée d'énergie après une zone calme
- Marqueur visuel sur la waveform, un clic pour accepter
- Champ `WaveformData.suggested_momentum`

### Section 5 - YouTube Mix Discovery
- `DiscoveryEngine` avec seeds extraites des sons YouTube du profil
- `mix_fetcher.rs` fetch les YouTube Mix via yt-dlp `--flat-playlist`
- Agrégation cross-seed : les vidéos trouvées dans plusieurs Mix sont mieux classées
- Top 30 suggestions, filtrées (30-900s, hors sons existants)
- Streaming partiel via `discovery_partial` events
- Cache par profil avec `seed_hash` pour détecter les changements
- Annulable via `cancel_discovery()`

### Bonus - Smart Auto-Assignment
- `profileAnalysis.ts` détecte le mode du profil (single-sound vs multi-sound)
- Single-sound: touche linéaire, piste la moins utilisée
- Multi-sound: clustering par seeds similaires
- `computeAutoAssign()` pour chaque suggestion

### Bonus - Pre-download Pipeline
- `useDiscoveryPredownload.ts` pré-télécharge autour de la position courante du carousel
- Fenêtre asymétrique [current-2, ..., current+3], max 3 concurrent
- `predownload_suggestion()` retourne audio + durée + waveform en un appel

## Fichiers créés

| Fichier | Description |
|---------|-------------|
| `src-tauri/src/audio/analysis.rs` | Analyse RMS waveform + détection momentum + WaveformCache |
| `src-tauri/src/discovery/engine.rs` | Moteur de découverte (croisement des Mix, scoring) |
| `src-tauri/src/discovery/mix_fetcher.rs` | Fetch des YouTube Mix via yt-dlp |
| `src-tauri/src/discovery/cache.rs` | Cache des suggestions par profil |
| `src/components/Discovery/DiscoveryPanel.tsx` | UI carrousel découverte sidebar |
| `src/components/common/WaveformDisplay.tsx` | Composant canvas waveform dual-layer |
| `src/stores/discoveryStore.ts` | Store Zustand pour la découverte |
| `src/hooks/useDiscovery.ts` | Hook pour écouter les events de découverte |
| `src/hooks/useDiscoveryPredownload.ts` | Hook pour le pré-téléchargement intelligent |
| `src/utils/profileAnalysis.ts` | Analyse de profil + auto-assignation |

## Fichiers modifiés

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/commands.rs` | Commandes search_youtube, fetch_playlist, get_waveform, discovery_*, predownload_suggestion |
| `src-tauri/src/state.rs` | Ajout waveform_cache, discovery_cancel |
| `src-tauri/src/types.rs` | Types WaveformData, YoutubeSearchResult, YoutubePlaylist, DiscoverySuggestion, PredownloadResult |
| `src/types/index.ts` | Types frontend correspondants |
| `src/utils/tauriCommands.ts` | Wrappers pour toutes les nouvelles commandes |
| `src/stores/profileStore.ts` | Discovery cache cleanup on profile delete, batched duration updates |
| `src/components/Sounds/SoundDetails.tsx` | Waveform integration, targeted Zustand subscription |
| `src/components/Keys/KeyGrid.tsx` | Performance optimization (usePlayingSoundIds) |
