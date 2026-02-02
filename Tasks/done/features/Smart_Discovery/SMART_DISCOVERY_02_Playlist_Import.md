# Section 2 — Import de playlists YouTube

> **Statut:** ✅ COMPLÉTÉ
> **Date de complétion:** 2026-01-31
> **Dépendance:** Section 1 (réutilise l'UI search et le type `YoutubeSearchResult`)
> **Parent:** [SMART_DISCOVERY.md](./SMART_DISCOVERY.md)

> **Objectif:** L'utilisateur colle une URL de playlist YouTube → l'app récupère tous les titres → l'utilisateur sélectionne lesquels télécharger → download en bulk. Si l'utilisateur veut juste télécharger une vidéo individuelle depuis une playlist, il peut le faire sans friction.

## Principe

yt-dlp supporte nativement les playlists. On détecte quand l'URL contient `list=`, et on propose à l'utilisateur de télécharger toute la playlist via une checkbox opt-in. L'état de cette checkbox est persisté dans `config.json` pour mémoriser la préférence de l'utilisateur.

### Gestion vidéo vs playlist

Il y a 3 types d'URLs YouTube possibles :

| Type d'URL | Exemple | Comportement |
|---|---|---|
| Vidéo simple | `youtube.com/watch?v=abc123` | Download direct (pas de checkbox affichée) |
| Vidéo dans une playlist | `youtube.com/watch?v=abc123&list=PLxxx` | Checkbox "Télécharger toute la playlist" visible. Si décochée → download juste la vidéo (on ignore `list=`). Si cochée → fetch playlist + sélecteur. |
| Playlist pure | `youtube.com/playlist?list=PLxxx` | Pas de vidéo individuelle → mode playlist direct (fetch + sélecteur, pas de checkbox) |

### Persistance de la préférence

- Nouveau champ dans `config.json` : `playlistImportEnabled: bool` (défaut: `false`)
- Cross-profil (préférence globale de l'utilisateur)
- Quand l'utilisateur coche/décoche, l'état est sauvegardé et réutilisé la prochaine fois qu'une URL avec `list=` est détectée

## Backend (Rust)

- [ ] **2.1** Créer la commande `fetch_playlist(url: String) -> Result<YoutubePlaylist, String>`
  - Utilise yt-dlp avec `--flat-playlist --dump-json` sur l'URL de la playlist
  - Parse chaque ligne JSON (une par vidéo)
  - Retourne la playlist avec ses entrées

- [ ] **2.2** Définir le type `YoutubePlaylist` dans `types.rs`
  ```rust
  pub struct YoutubePlaylist {
      pub title: String,           // nom de la playlist
      pub entries: Vec<YoutubeSearchResult>,  // réutilise le même type que la recherche
      pub total_count: usize,
  }
  ```

- [ ] **2.3** Enregistrer la commande dans `main.rs`

- [ ] **2.4** Ajouter le champ `playlist_import_enabled: bool` dans la config (défaut: `false`)
  - Ajout dans `storage/config.rs`
  - Sérialisation/désérialisation avec serde (avec `#[serde(default)]` pour rétrocompatibilité)

## Frontend

- [ ] **2.5** Ajouter le wrapper dans `tauriCommands.ts`
  ```typescript
  export async function fetchPlaylist(url: string): Promise<YoutubePlaylist>
  ```

- [ ] **2.6** Définir le type `YoutubePlaylist` dans `types/index.ts`

- [ ] **2.7** Ajouter `playlistImportEnabled` dans `settingsStore.ts`
  - Lecture depuis la config au démarrage
  - Action `setPlaylistImportEnabled(enabled: boolean)` qui persiste dans `config.json`

- [ ] **2.8** Modifier le tab YouTube de `AddSoundModal.tsx` pour gérer les playlists
  - **Détection d'URL** : quand l'utilisateur entre une URL, détecter la présence de `list=`
  - **Checkbox "Télécharger toute la playlist"** :
    - N'apparaît que si l'URL contient `list=` ET contient aussi `v=` (vidéo dans une playlist)
    - État initial = `playlistImportEnabled` depuis le store
    - Au changement : appeler `setPlaylistImportEnabled()` pour persister la préférence
    - Positionnée juste en dessous du champ URL
  - **Si checkbox décochée** (ou non affichée pour une vidéo simple) : download la vidéo normalement via `addSoundFromYoutube` (on strip le paramètre `list=` de l'URL pour éviter les interférences avec yt-dlp)
  - **Si checkbox cochée** OU **URL playlist pure** (`/playlist?list=` sans `v=`) : appeler `fetchPlaylist(url)` et afficher le sélecteur
  - **UI Sélecteur de playlist** :
    - Header : nom de la playlist + nombre total de vidéos
    - Checkbox "Tout sélectionner / Tout désélectionner"
    - Liste scrollable de toutes les vidéos, chacune avec :
      - Checkbox de sélection (checked par défaut)
      - Titre
      - Durée (MM:SS)
      - Badge "Déjà téléchargé" si dans le cache
    - Bouton "Ajouter X sons sélectionnés" en bas
  - **Au clic sur Ajouter** : lance `addSoundFromYoutube` pour chaque vidéo sélectionnée
    - Downloads concurrents (réutilise le système existant avec `downloadId` unique par vidéo)
    - Chaque son apparaît dans la liste des fichiers du modal au fur et à mesure
    - Progress bars individuelles comme actuellement

- [ ] **2.9** Gestion des grosses playlists
  - Si la playlist a plus de 50 vidéos, afficher un avertissement : "Cette playlist contient {N} vidéos. Sélectionnez celles que vous souhaitez ajouter."
  - Pas de limite hard, mais l'avertissement guide l'utilisateur

## Comportement détaillé

### Cas 1 : Vidéo copiée depuis une playlist (cas le plus courant)
```
Utilisateur colle "https://youtube.com/watch?v=abc123&list=PLxxxxx"
    → Frontend détecte "list=" + "v=" dans l'URL
    → Affiche checkbox "Télécharger toute la playlist" (état = config.playlistImportEnabled)
    → Checkbox décochée (défaut) → clic "Ajouter"
    → Strip "list=PLxxxxx" de l'URL → download juste la vidéo abc123
    → Comportement identique à un lien vidéo simple
```

### Cas 2 : Utilisateur veut la playlist entière depuis un lien vidéo
```
Utilisateur colle "https://youtube.com/watch?v=abc123&list=PLxxxxx"
    → Checkbox "Télécharger toute la playlist" affichée
    → Utilisateur coche la checkbox → préférence sauvée dans config
    → Appelle fetchPlaylist(url)
    → Backend lance: yt-dlp --flat-playlist --dump-json "URL"
    → Retourne YoutubePlaylist { title, entries, total_count }
    → Frontend affiche le sélecteur de playlist
    → Utilisateur sélectionne les vidéos voulues
    → Clic "Ajouter 12 sons" → 12 downloads concurrents
```

### Cas 3 : URL playlist pure
```
Utilisateur colle "https://youtube.com/playlist?list=PLxxxxx"
    → Frontend détecte "list=" SANS "v=" → playlist pure
    → Pas de checkbox (pas de vidéo individuelle à télécharger)
    → Appelle fetchPlaylist(url) directement
    → Affiche le sélecteur de playlist
```

### Cas 4 : Prochain lien après avoir activé la playlist
```
Utilisateur colle "https://youtube.com/watch?v=xyz789&list=PLyyyyy"
    → Checkbox affichée, déjà cochée (préférence sauvée précédemment)
    → Mode playlist activé automatiquement
    → Si l'utilisateur décoche → préférence mise à jour
```

## Fichiers impactés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/commands.rs` | Ajouter commande `fetch_playlist` |
| `src-tauri/src/types.rs` | Ajouter `YoutubePlaylist` |
| `src-tauri/src/main.rs` | Enregistrer la commande |
| `src-tauri/src/storage/config.rs` | Ajouter champ `playlist_import_enabled` |
| `src/utils/tauriCommands.ts` | Ajouter wrapper |
| `src/types/index.ts` | Ajouter type frontend + `playlistImportEnabled` dans Config |
| `src/stores/settingsStore.ts` | Ajouter état et action `setPlaylistImportEnabled` |
| `src/components/Sounds/AddSoundModal.tsx` | Checkbox playlist + détection URL + UI sélecteur |
