# Smart Discovery & Auto-Setup

> **Statut:** ⏳ PLANIFIÉ
> **Type:** Update — Nouvelles features post-release
> **Objectif:** Transformer le setup de "30 minutes de travail manuel" en "2 minutes de clics". Ajouter la recherche YouTube intégrée, l'import de playlists, la waveform visuelle avec auto-momentum, et un système de découverte musicale basé sur les YouTube Mix (recommandations croisées).

---

## Vue d'ensemble

L'app actuelle est solide côté playback et key detection, mais le **setup** reste le gros point de friction : trouver des sons en externe, les importer un par un, configurer le momentum manuellement pour chaque son. Cette phase élimine ces frictions en 5 sous-phases indépendantes, chacune livrant de la valeur seule.

## Sous-phases

| # | Feature | Dépendance | Détails |
|---|---------|------------|---------|
| 1 | Recherche YouTube intégrée | Aucune | [SMART_DISCOVERY_01_YouTube_Search.md](./SMART_DISCOVERY_01_YouTube_Search.md) |
| 2 | Import de playlists YouTube | Section 1 | [SMART_DISCOVERY_02_Playlist_Import.md](./SMART_DISCOVERY_02_Playlist_Import.md) |
| 3 | Waveform RMS (visualisation d'énergie) | Aucune | [SMART_DISCOVERY_03_Waveform_RMS.md](./SMART_DISCOVERY_03_Waveform_RMS.md) |
| 4 | Auto-Momentum (marqueur suggéré) | Section 3 | [SMART_DISCOVERY_04_Auto_Momentum.md](./SMART_DISCOVERY_04_Auto_Momentum.md) |
| 5 | Découverte musicale (YouTube Mix croisés) | Sections 1-4 | [SMART_DISCOVERY_05_YouTube_Mix_Discovery.md](./SMART_DISCOVERY_05_YouTube_Mix_Discovery.md) |

## Ordre d'implémentation recommandé

1. **Section 1** — Recherche YouTube intégrée (indépendant, valeur immédiate)
2. **Section 2** — Playlists YouTube (extension naturelle de la section 1, réutilise `fetch_playlist` pour les Mix)
3. **Section 3** — Waveform RMS (indépendant, valeur immédiate, améliore l'éditeur de momentum)
4. **Section 4** — Auto-Momentum marqueur (extension de la section 3, ajoute le marqueur suggéré sur la waveform)
5. **Section 5** — Découverte YouTube Mix (utilise yt-dlp de section 1-2 pour fetch les Mix, sections 3-4 pour le momentum des sons découverts)

Chaque section est déployable seule et apporte de la valeur indépendamment. Les sections 3 et 4 peuvent être implémentées en parallèle des sections 1-2.

## Résumé des fichiers

### Nouveaux fichiers

| Fichier | Description | Section |
|---------|-------------|---------|
| `src-tauri/src/audio/analysis.rs` | Analyse RMS waveform + détection pré-pic | 3, 4 |
| `src/components/common/WaveformDisplay.tsx` | Composant canvas waveform avec marqueurs | 3, 4 |
| `src-tauri/src/discovery/mod.rs` | Module découverte musicale | 5 |
| `src-tauri/src/discovery/mix_fetcher.rs` | Fetch des YouTube Mix via yt-dlp | 5 |
| `src-tauri/src/discovery/engine.rs` | Moteur de découverte (croisement des Mix, scoring) | 5 |
| `src-tauri/src/discovery/cache.rs` | Cache des suggestions par profil | 5 |
| `src/components/Discovery/DiscoveryPanel.tsx` | UI carrousel découverte sidebar | 5 |
| `src/stores/discoveryStore.ts` | Store Zustand pour la découverte | 5 |
| `src/hooks/useDiscovery.ts` | Hook pour écouter les events de découverte | 5 |

### Fichiers à modifier

| Fichier | Modification | Section |
|---------|-------------|---------|
| `src-tauri/src/main.rs` | Enregistrer nouvelles commandes, lancer découverte au démarrage, nettoyage temp | 1-5 |
| `src-tauri/src/commands.rs` | Ajouter commandes search_youtube, fetch_playlist, get_waveform, discovery_* | 1-5 |
| `src-tauri/src/types.rs` | Ajouter types YoutubeSearchResult, YoutubePlaylist, WaveformData, DiscoverySuggestion, etc. | 1-5 |
| `src-tauri/src/audio/mod.rs` | Exporter analysis | 3 |
| `src/components/Sounds/AddSoundModal.tsx` | Champ recherche YouTube, sélecteur playlist, waveform momentum | 1, 2, 3, 4 |
| `src/components/Sounds/SoundDetails.tsx` | Waveform momentum avec marqueur auto | 3, 4 |
| `src/components/Layout/Sidebar.tsx` | Ajouter DiscoveryPanel | 5 |
| `src/components/Controls/NowPlaying.tsx` | Filtrer le track "__preview__" | 5 |
| `src/types/index.ts` | Ajouter types frontend | 1-5 |
| `src/utils/tauriCommands.ts` | Ajouter wrappers pour les nouvelles commandes | 1-5 |
| `src/App.tsx` | Initialiser useDiscovery | 5 |

## Risques et mitigations

| Risque | Impact | Section | Mitigation |
|--------|--------|---------|------------|
| yt-dlp search est lent (2-5s par requête) | UX de la recherche | 1 | Loading state clair, résultats conservés, pas de re-fetch inutile |
| Playlist très grande (100+ vidéos) | Temps de chargement long | 2 | Avertissement + sélection manuelle, pas de download auto |
| Waveform lente sur fichiers > 10 min | UX de l'éditeur de momentum | 3 | Décodage partiel / échantillonnage, objectif < 1.5s |
| Canvas performance sur vieux PC | Rendu saccadé | 3 | 250 points seulement, pas d'animation lourde |
| Auto-momentum imprécis | Marqueur mal placé | 4 | Marqueur visuel sur waveform = l'utilisateur voit et corrige en un clic |
| YouTube Mix vide ou limité (~25 vidéos) | Moins de recommandations par seed | 5 | Compensé par le croisement de multiples seeds (10-15 seeds = 250-375 entrées brutes) |
| Mix fetch lent (~3-5s par seed) | Génération longue pour gros profils | 5 | Background thread + parallélisation prudente (2 max) + limite à 15 seeds |
| Profil sans sons YouTube | Pas de seeds pour la découverte | 5 | Message clair "Ajoutez des sons YouTube pour activer", feature désactivée gracieusement |
| YouTube bloque certaines régions | Certains Mix incomplets | 5 | Le seed est skipé, les autres continuent, pas d'erreur bloquante |
| Suggestions peu pertinentes (score 1) | Bruit dans les résultats | 5 | Trier par score, afficher visuellement le niveau de confiance |
| Cache obsolète après ajout de sons | Suggestions périmées | 5 | Bouton refresh + re-génération auto quand le seed_hash change |
| yt-dlp change de comportement avec les Mix | Feature cassée | 5 | Les Mix utilisent `--flat-playlist` standard, même syntaxe que les playlists normales |
