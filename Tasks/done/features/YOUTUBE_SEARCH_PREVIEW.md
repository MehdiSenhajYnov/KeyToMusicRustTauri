# YouTube Search Preview (Streaming)

> **Categorie:** Feature
> **Priorite:** Haute
> **Statut:** Completed
> **Date ajoutee:** 2026-02-02
> **Date completee:** 2026-02-02
> **Type:** Update — Preview audio en streaming pour les resultats de recherche YouTube

---

## Description

Ajouter un systeme de preview audio en streaming pour les resultats de recherche YouTube dans le modal AddSound. L'utilisateur peut ecouter un resultat directement depuis YouTube (sans telecharger) pour verifier que c'est le bon son avant de l'ajouter. Une barre de controle inline s'affiche sous le resultat avec play/pause, seek bar, et timer.

## Motivation

Actuellement, l'utilisateur doit telecharger un son pour l'ecouter. S'il se trompe, il a perdu du temps et de la bande passante. Le streaming permet de valider rapidement un resultat avant de s'engager dans le telechargement.

---

## Vue d'ensemble technique

### Approche : Streaming via URL directe yt-dlp

yt-dlp peut extraire l'URL de stream audio directe d'une video YouTube via `--dump-json` (sans `--flat-playlist`). Le JSON retourne contient un tableau `formats[]` avec des URLs directes (expirent apres ~6h). On extrait la meilleure URL audio et on la joue via un systeme dedie (pas le moteur audio principal).

**Avantage:** Zero telechargement, lecture quasi-instantanee.
**Contrainte:** Les URLs expirent (~6h), il faut les re-fetcher si necessaire. Necessite une connexion active pendant la lecture.

---

## Implementation

### Section 1 — Backend : Extraction d'URL de stream

**Fichiers concernes:**
- `src-tauri/src/youtube/search.rs` — Nouvelle fonction `get_stream_url`
- `src-tauri/src/commands.rs` — Nouvelle commande Tauri `get_youtube_stream_url`

**Details:**

Creer une fonction qui, pour un `video_id` donne, lance yt-dlp pour extraire l'URL audio directe :

```bash
yt-dlp "https://www.youtube.com/watch?v={video_id}" --dump-json --no-download -f bestaudio
```

Le JSON retourne contient un champ `url` au top-level (quand `-f` est specifie) qui est l'URL directe du stream audio.

**Fonction a creer dans `search.rs`:**
```rust
pub async fn get_stream_url(video_id: &str) -> Result<StreamUrlResult, String>
```

**Struct retour:**
```rust
pub struct StreamUrlResult {
    pub url: String,           // URL directe du stream audio
    pub duration: f64,         // Duree en secondes
    pub format: String,        // "m4a", "webm", etc.
    pub expires_at: Option<u64>, // Timestamp d'expiration (parse depuis l'URL si possible)
}
```

**Commande Tauri dans `commands.rs`:**
```rust
#[tauri::command]
pub async fn get_youtube_stream_url(
    state: State<'_, AppState>,
    video_id: String,
) -> Result<StreamUrlResult, String>
```

**Notes:**
- Reutiliser `yt_dlp_command()` de `downloader.rs:16` pour la construction de la commande
- Reutiliser `find_yt_dlp()` de `yt_dlp_manager.rs:27` pour le binaire
- Timeout de 10s (plus court que le download car c'est juste du metadata)
- Pas de retry — si ca echoue, l'utilisateur peut re-cliquer

---

### Section 2 — Frontend : Lecteur audio HTML5

**Fichiers concernes:**
- `src/components/Sounds/SearchResultPreview.tsx` — Nouveau composant (player inline)
- `src/components/Sounds/AddSoundModal.tsx` — Integration du player dans les resultats

**Details:**

**Choix technique : `<audio>` HTML5 natif** (pas le moteur rodio/symphonia du backend).

Raisons :
- Le moteur audio backend (`audio/engine.rs`) utilise `SymphoniaSource` qui ne lit que des fichiers locaux (`File::open` a `symphonia_source.rs:45`)
- Modifier le moteur audio pour supporter HTTP serait un changement lourd et risque pour la stabilite
- L'element `<audio>` HTML5 supporte nativement les streams HTTP, le seeking, et le buffering
- La preview n'a pas besoin des features du moteur (crossfade, multi-track, momentum)
- Le volume de preview peut etre controle independamment

**Composant `SearchResultPreview.tsx`:**

Props :
```typescript
interface SearchResultPreviewProps {
  streamUrl: string;
  duration: number;
  onClose: () => void;
}
```

Contenu :
- Element `<audio>` cache, ref via `useRef<HTMLAudioElement>`
- Bouton Play/Pause (toggle)
- Seek bar (`<input type="range">`) liee a `audio.currentTime`
- Timer : `currentTime / duration` au format `mm:ss / mm:ss`
- Mise a jour via `timeupdate` event du `<audio>` (~4x/sec natif)
- Gestion des etats : loading (buffering), playing, paused, ended, error

**Integration dans AddSoundModal:**

Actuellement les resultats de recherche sont rendus dans une liste scrollable (lignes 729-758 de `AddSoundModal.tsx`). Ajouter :
- Un bouton play (icone triangle) sur chaque resultat, a cote du bouton "Add"
- Au clic sur play : fetch l'URL de stream via `getYoutubeStreamUrl(videoId)`, puis afficher le composant `SearchResultPreview` inline sous le resultat
- Un seul preview actif a la fois (cliquer sur un autre arrete le precedent)
- Le preview se ferme si on clique sur un autre play ou sur le X

**Etat dans AddSoundModal:**
```typescript
const [previewState, setPreviewState] = useState<{
  videoId: string;
  streamUrl: string;
  duration: number;
  isLoading: boolean;
} | null>(null);
```

---

### Section 3 — UI/UX du player inline

**Fichiers concernes:**
- `src/components/Sounds/SearchResultPreview.tsx` — Style et layout

**Details:**

Le player apparait directement sous le resultat de recherche clique, dans la liste scrollable. Layout :

```
┌─────────────────────────────────────────────────┐
│ 🎵 Titre du son                    [Channel] 3:42 │  ← Resultat normal
│                                    [▶ Play] [Add] │
├─────────────────────────────────────────────────┤
│  [⏸] ═══════════●══════════════  1:23 / 3:42  [✕] │  ← Player inline (expanded)
└─────────────────────────────────────────────────┘
```

- **Fond:** `bg-surface-secondary` ou leger contraste par rapport au resultat parent
- **Hauteur:** ~36-40px, compact
- **Seek bar:** Style coherent avec le reste de l'app (accent-primary/indigo)
- **Timer:** `text-xs text-muted`
- **Bouton fermer (X):** Arrete la lecture et collapse le player
- **Etat loading:** Spinner a la place du bouton play pendant le fetch de l'URL de stream + buffering initial
- **Etat erreur:** Message inline "Preview indisponible" avec possibilite de retry

**Interaction avec le scroll:**
- Le resultat + player restent dans le flux normal de la liste scrollable
- Si le resultat avec preview est hors vue, la lecture continue (pas d'arret automatique au scroll)

---

### Section 4 — Gestion du cycle de vie

**Fichiers concernes:**
- `src/components/Sounds/AddSoundModal.tsx` — Cleanup et interactions

**Details:**

- **Fermeture du modal:** Stopper la lecture audio (`audio.pause()`) dans le cleanup du `useEffect` ou dans `handleClose`
- **Changement de mode (local/youtube):** Stopper la preview si active
- **Lancement d'un download:** Stopper la preview du meme resultat (on ne veut pas streamer et telecharger en parallele)
- **Unmount:** L'element `<audio>` est detruit naturellement, mais s'assurer de `pause()` + vider `src` pour liberer les ressources reseau
- **Un seul stream actif:** Mutual exclusion — cliquer play sur un autre resultat arrete le precedent
- **Preview existante pendant typing:** Si l'utilisateur tape une nouvelle recherche, la preview en cours continue (ne pas la couper). Elle disparait quand les resultats changent.

---

### Section 5 — Commande Tauri + Types frontend

**Fichiers concernes:**
- `src/utils/tauriCommands.ts` — Nouveau wrapper
- `src/types/index.ts` — Nouveau type

**Details:**

**Type TypeScript (`types/index.ts`):**
```typescript
interface StreamUrlResult {
  url: string;
  duration: number;
  format: string;
  expiresAt: number | null;
}
```

**Wrapper Tauri (`tauriCommands.ts`):**
```typescript
export async function getYoutubeStreamUrl(videoId: string): Promise<StreamUrlResult> {
  return invoke<StreamUrlResult>("get_youtube_stream_url", { videoId });
}
```

---

## Fichiers a creer

| Fichier | Description |
|---------|-------------|
| `src/components/Sounds/SearchResultPreview.tsx` | Composant player inline (audio HTML5 + seek bar + timer) |

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/youtube/search.rs` | Ajouter `get_stream_url()` et `StreamUrlResult` |
| `src-tauri/src/commands.rs` | Ajouter commande `get_youtube_stream_url` |
| `src-tauri/src/main.rs` | Enregistrer la nouvelle commande dans `invoke_handler` |
| `src/components/Sounds/AddSoundModal.tsx` | Bouton play par resultat, etat preview, integration du player inline |
| `src/utils/tauriCommands.ts` | Wrapper `getYoutubeStreamUrl()` |
| `src/types/index.ts` | Type `StreamUrlResult` |

## Taches

- [x] **Backend:** Creer `StreamUrlResult` struct dans `search.rs`
- [x] **Backend:** Implementer `get_stream_url()` avec invocation yt-dlp
- [x] **Backend:** Ajouter commande Tauri `get_youtube_stream_url` dans `commands.rs`
- [x] **Backend:** Enregistrer la commande dans `main.rs`
- [x] **Frontend:** Creer composant `SearchResultPreview.tsx` avec `<audio>` HTML5
- [x] **Frontend:** Ajouter type `StreamUrlResult` dans `types/index.ts`
- [x] **Frontend:** Ajouter wrapper dans `tauriCommands.ts`
- [x] **Frontend:** Integrer bouton play + player inline dans `AddSoundModal.tsx`
- [x] **Frontend:** Gestion du cycle de vie (cleanup, mutual exclusion, modal close)
- [x] **Test:** Verifier le streaming avec differents types de videos (musique, longue, courte)
- [x] **Test:** Verifier le seeking dans le stream
- [x] **Test:** Verifier le comportement en cas d'erreur reseau

## Notes

- Les URLs de stream YouTube expirent apres ~6h. Pour une session d'utilisation normale du modal, ce n'est pas un probleme. Si l'URL expire pendant la lecture, le `<audio>` emettra une erreur — afficher "Preview expiree, relancez" et re-fetch l'URL.
- Le format retourne par yt-dlp avec `-f bestaudio` est generalement du WebM/Opus ou M4A/AAC. Les deux sont supportes par `<audio>` HTML5 sur les navigateurs modernes (et donc Tauri WebView).
- Si yt-dlp n'est pas installe, le bouton play ne doit pas apparaitre (verifier via l'etat existant de `check_yt_dlp_installed`).
- Cette feature est independante du moteur audio backend — la preview utilise l'audio HTML5 du WebView, pas rodio/symphonia.
- Le volume de la preview HTML5 est independant du master volume de l'app. On peut ajouter un controle de volume plus tard si necessaire.
