# Section 1 — Recherche YouTube intégrée

> **Statut:** ✅ COMPLÉTÉ
> **Date de complétion:** 2026-01-31
> **Dépendance:** Aucune
> **Parent:** [SMART_DISCOVERY.md](./SMART_DISCOVERY.md)

> **Objectif:** L'utilisateur tape des mots-clés directement dans l'AddSoundModal et voit des résultats YouTube sans quitter l'app.

## Principe

Actuellement le tab YouTube de l'AddSoundModal demande une URL. On ajoute un **mode recherche** : l'utilisateur tape des mots-clés (ex: "naruto sad ost"), l'app cherche via yt-dlp et affiche les résultats. L'utilisateur clique sur un résultat pour le télécharger.

## Backend (Rust)

- [ ] **1.1** Créer la commande `search_youtube(query: String, max_results: u32) -> Result<Vec<YoutubeSearchResult>, String>`
  - Utilise yt-dlp avec `ytsearch{max_results}:{query}` et `--flat-playlist --dump-json`
  - Parse le JSON pour chaque résultat
  - Retourne une liste de résultats

- [ ] **1.2** Définir le type `YoutubeSearchResult` dans `types.rs`
  ```rust
  pub struct YoutubeSearchResult {
      pub video_id: String,
      pub title: String,
      pub duration: f64,        // secondes, depuis yt-dlp "duration" field
      pub channel: String,      // uploader name
      pub thumbnail_url: String, // URL de la miniature (optionnel, pour v2)
      pub url: String,          // URL canonique construite: https://www.youtube.com/watch?v={video_id}
  }
  ```

- [ ] **1.3** Vérifier le cache avant de retourner les résultats
  - Si un `video_id` est déjà dans le cache YouTube, marquer le résultat comme `already_downloaded: true`
  - L'UI peut afficher un badge "Déjà téléchargé" pour éviter les doublons

- [ ] **1.4** Enregistrer la commande dans `main.rs`
  - Ajouter `search_youtube` à la liste des commandes Tauri

## Frontend

- [ ] **1.5** Ajouter le wrapper dans `tauriCommands.ts`
  ```typescript
  export async function searchYoutube(query: string, maxResults?: number): Promise<YoutubeSearchResult[]>
  ```

- [ ] **1.6** Définir le type `YoutubeSearchResult` dans `types/index.ts`
  ```typescript
  interface YoutubeSearchResult {
    videoId: string;
    title: string;
    duration: number;
    channel: string;
    thumbnailUrl: string;
    url: string;
    alreadyDownloaded: boolean;
  }
  ```

- [ ] **1.7** Modifier le tab YouTube de `AddSoundModal.tsx`
  - **Champ de recherche** : input texte avec placeholder "Rechercher sur YouTube..." et un bouton "Rechercher" (ou Enter)
  - **Le champ URL existant reste** : l'utilisateur peut toujours coller une URL directe. Détecter automatiquement si l'input est une URL (commence par `http` ou `youtu`) ou des mots-clés
  - **Zone de résultats** : liste scrollable sous le champ de recherche, affichée quand des résultats existent
  - **Chaque résultat** affiche :
    - Titre (texte principal, truncate avec tooltip)
    - Channel name (texte secondaire, muted)
    - Durée formatée (MM:SS, à droite)
    - Badge "Déjà téléchargé" si dans le cache (texte vert discret)
    - Bouton "Ajouter" (icône + ou texte) → lance le download comme si l'utilisateur avait collé l'URL
  - **État loading** : spinner + "Recherche en cours..." pendant que yt-dlp cherche
  - **État vide** : "Aucun résultat pour '{query}'" si 0 résultats
  - **État erreur** : message d'erreur si yt-dlp échoue (pas de réseau, etc.)
  - **Nombre de résultats** : 10 par défaut (paramètre `max_results`)
  - Le flow après clic sur "Ajouter" est identique au flow URL actuel : le son apparaît dans la liste des fichiers du modal avec progress bar, momentum editor, etc.

- [ ] **1.8** Gestion du debounce et UX
  - Pas de recherche automatique au typing (yt-dlp est lent, ~2-5 secondes par recherche)
  - Recherche déclenchée uniquement par Enter ou clic sur le bouton Rechercher
  - Désactiver le bouton pendant une recherche en cours
  - Conserver les résultats affichés même après avoir lancé un téléchargement (l'utilisateur peut en ajouter plusieurs d'affilée)

## Comportement détaillé

```
Utilisateur tape "naruto sad ost" + Enter
    → Frontend appelle searchYoutube("naruto sad ost", 10)
    → Backend lance: yt-dlp "ytsearch10:naruto sad ost" --flat-playlist --dump-json
    → Parse les 10 résultats JSON
    → Vérifie le cache pour chaque video_id
    → Retourne Vec<YoutubeSearchResult>
    → Frontend affiche la liste
    → Utilisateur clique "Ajouter" sur "Naruto Shippuden OST - Sadness and Sorrow"
    → Lance addSoundFromYoutube(url, downloadId) comme avant
    → Son ajouté à la liste des fichiers du modal
    → Résultats restent affichés, utilisateur peut en ajouter d'autres
```

## Fichiers impactés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/commands.rs` | Ajouter commande `search_youtube` |
| `src-tauri/src/types.rs` | Ajouter `YoutubeSearchResult` |
| `src-tauri/src/main.rs` | Enregistrer la commande |
| `src/utils/tauriCommands.ts` | Ajouter wrapper |
| `src/types/index.ts` | Ajouter type frontend |
| `src/components/Sounds/AddSoundModal.tsx` | Champ recherche + zone résultats |
