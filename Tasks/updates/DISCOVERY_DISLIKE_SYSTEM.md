# Système de Dislike Permanent pour Discovery

> **Statut:** En attente d'implémentation
> **Type:** Update — Remplacer le dismiss temporaire par un dislike permanent avec UI de gestion
> **Objectif:** Permettre aux utilisateurs de blacklister définitivement des sons dans Discovery, avec possibilité de gérer cette liste dans les Settings

---

## Vue d'ensemble

Actuellement, le bouton "X" (dismiss) dans Discovery retire un son de la session courante, mais il peut réapparaître lors d'une nouvelle génération. Ce système n'est pas suffisamment permanent et le bouton fait doublon avec refresh/flèches.

L'objectif est de transformer ce système en **dislike permanent** :
- Le bouton devient un "thumbs down" (pouce vers le bas)
- Les sons dislikés ne reviennent **jamais** pour ce profil
- Une section dans Settings permet de voir la liste des sons dislikés et de les retirer si nécessaire

## Architecture actuelle

### Backend (`dismissed_ids`)

**Fichier:** `src-tauri/src/discovery/cache.rs:10-25`

```rust
pub struct DiscoveryCacheData {
    pub dismissed_ids: Vec<String>,  // ← Actuellement session-only
    // ...
}
```

**Filtrage:** `src-tauri/src/commands.rs:1038-1046`
- Les `dismissed_ids` sont chargés du cache précédent et ajoutés à `existing_set`
- `build_suggestions()` filtre les suggestions contre cet ensemble

**Limitation:** Les `dismissed_ids` vivent uniquement dans le cache Discovery (`data/discovery/{profile_id}.json`). Quand le cache est recréé ou qu'une nouvelle discovery est lancée, les dismissed sont préservés **mais** ne sont pas stockés ailleurs.

### Frontend

**UI:** `src/components/Discovery/DiscoveryPanel.tsx:536-546`
- Bouton X (icône croix) en haut à droite
- Appelle `handleDismiss()` (ligne 295-301)

**Handler:** `removeSuggestion()` + `commands.dismissDiscovery()`

---

## Implementation

### 1. Backend — Stockage permanent des dislikes

**Objectif:** Stocker les dislikes dans le profil lui-même, pas juste dans le cache Discovery.

#### 1.1 Ajouter `disliked_videos` au profil

**Fichier:** `src-tauri/src/types.rs`

Trouver la struct `Profile` et ajouter :
```rust
#[serde(default)]
pub disliked_videos: Vec<String>,  // video_ids blacklistés définitivement
```

**Note:** Le `#[serde(default)]` assure la rétrocompatibilité avec les anciens profils.

#### 1.2 Créer la commande `dislike_discovery`

**Fichier:** `src-tauri/src/commands.rs`

Ajouter après `dismiss_discovery` (ligne ~1186) :

```rust
#[tauri::command]
pub fn dislike_discovery(
    state: State<'_, AppState>,
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    // Load profile
    let mut profile = storage::profile::load_profile(&profile_id)?;

    // Add to disliked_videos (avoid duplicates)
    if !profile.disliked_videos.contains(&video_id) {
        profile.disliked_videos.push(video_id.clone());
    }

    // Save profile
    storage::profile::save_profile(&profile)?;

    // Also dismiss from current discovery cache
    discovery::cache::DiscoveryCache::dismiss(&profile_id, &video_id)?;

    // Clean up cached audio in background (best-effort)
    let cache = state.youtube_cache.clone();
    let vid = video_id.clone();
    std::thread::spawn(move || {
        if let Ok(mut cache) = cache.lock() {
            cache.ensure_loaded();
            cache.remove_entry_by_video_id(&vid);
        }
    });

    Ok(())
}
```

**Registrer la commande:** `src-tauri/src/main.rs:~276`
Ajouter `dislike_discovery,` dans la liste des commandes.

#### 1.3 Créer la commande `undislike_discovery`

**Fichier:** `src-tauri/src/commands.rs`

Ajouter après `dislike_discovery` :

```rust
#[tauri::command]
pub fn undislike_discovery(
    profile_id: String,
    video_id: String,
) -> Result<(), String> {
    let mut profile = storage::profile::load_profile(&profile_id)?;

    // Remove from disliked_videos
    profile.disliked_videos.retain(|id| id != &video_id);

    storage::profile::save_profile(&profile)?;

    Ok(())
}
```

**Registrer:** `src-tauri/src/main.rs:~277`

#### 1.4 Créer la commande `list_disliked_videos`

**Fichier:** `src-tauri/src/commands.rs`

Retourner les infos des vidéos dislikées (titre, durée, etc.) :

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DislikedVideoInfo {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub duration: f64,
    pub url: String,
}

#[tauri::command]
pub async fn list_disliked_videos(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Vec<DislikedVideoInfo>, String> {
    let profile = storage::profile::load_profile(&profile_id)?;
    let mut results = Vec::new();

    for video_id in &profile.disliked_videos {
        let url = format!("https://www.youtube.com/watch?v={}", video_id);

        // Try to fetch video info via yt-dlp
        match youtube::fetch_video_info(&url).await {
            Ok(info) => {
                results.push(DislikedVideoInfo {
                    video_id: video_id.clone(),
                    title: info.title,
                    channel: info.channel.unwrap_or_default(),
                    duration: info.duration,
                    url,
                });
            }
            Err(_) => {
                // If fetch fails, add minimal info
                results.push(DislikedVideoInfo {
                    video_id: video_id.clone(),
                    title: format!("Video {}", video_id),
                    channel: "Unknown".to_string(),
                    duration: 0.0,
                    url,
                });
            }
        }
    }

    Ok(results)
}
```

**Note:** Il faudra peut-être créer une helper function `youtube::fetch_video_info()` qui utilise yt-dlp pour récupérer les métadonnées d'une vidéo. Regarder `youtube/downloader.rs` pour s'inspirer.

**Registrer:** `src-tauri/src/main.rs:~278`

#### 1.5 Filtrer les suggestions par `disliked_videos`

**Fichier:** `src-tauri/src/commands.rs:1038-1046`

Modifier la construction de `existing_set` :

```rust
// Load profile to get disliked videos
let profile = storage::profile::load_profile(&profile_id)?;

// Combine profile sounds + previous dismissed + disliked_videos
let mut existing_ids: HashSet<String> = profile
    .sounds
    .iter()
    .filter_map(|s| {
        let url = &s.file_path;
        if url.contains("youtube.com") || url.contains("youtu.be") {
            youtube::extract_video_id(url)
        } else {
            None
        }
    })
    .collect();

// Add disliked videos to exclusion set
for video_id in &profile.disliked_videos {
    existing_ids.insert(video_id.clone());
}

// Add previously dismissed (session-only)
let previous_dismissed = discovery::cache::DiscoveryCache::load(&profile_id)
    .map(|d| d.dismissed_ids)
    .unwrap_or_default();
for vid in previous_dismissed.iter() {
    existing_ids.insert(vid.clone());
}
```

---

### 2. Frontend — Changer le bouton dismiss → dislike

**Objectif:** Remplacer l'icône X par un thumbs-down, changer le handler, ajouter confirmation optionnelle.

#### 2.1 Modifier l'icône et le handler

**Fichier:** `src/components/Discovery/DiscoveryPanel.tsx:536-546`

Remplacer le bouton X par :

```tsx
{current && (
  <button
    onClick={() => handleDislike(current.videoId)}
    className="w-5 h-5 flex items-center justify-center rounded text-text-muted hover:text-accent-error hover:bg-accent-error/10 transition-colors"
    title="Dislike (permanent)"
  >
    <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2}
            d="M10 14H5.236a2 2 0 01-1.789-2.894l3.5-7A2 2 0 018.736 3h4.018a2 2 0 01.485.06l3.76.94m-7 10v5a2 2 0 002 2h.096c.5 0 .905-.405.905-.904 0-.715.211-1.413.608-2.008L17 13V4m-7 10h2m5-10h2a2 2 0 012 2v6a2 2 0 01-2 2h-2.5" />
    </svg>
  </button>
)}
```

**Note:** Icône thumbs-down de Heroicons.

#### 2.2 Créer le handler `handleDislike`

**Fichier:** `src/components/Discovery/DiscoveryPanel.tsx`

Remplacer `handleDismiss` (ligne 295-301) par :

```tsx
const handleDislike = (videoId: string) => {
  if (!profile) return;
  stopPreview();
  removeSuggestion(videoId);
  commands.dislikeDiscovery(profile.id, videoId).catch(() => {});
  persistCursor();
};
```

**Note:** Quasiment identique mais appelle `dislikeDiscovery` au lieu de `dismissDiscovery`.

#### 2.3 Ajouter les wrappers TypeScript

**Fichier:** `src/utils/tauriCommands.ts`

Remplacer `dismissDiscovery` (ligne 318-320) par :

```typescript
export async function dislikeDiscovery(profileId: string, videoId: string): Promise<void> {
  return invoke("dislike_discovery", { profileId, videoId });
}

export async function undislikeDiscovery(profileId: string, videoId: string): Promise<void> {
  return invoke("undislike_discovery", { profileId, videoId });
}

export interface DislikedVideoInfo {
  videoId: string;
  title: string;
  channel: string;
  duration: number;
  url: string;
}

export async function listDislikedVideos(profileId: string): Promise<DislikedVideoInfo[]> {
  return invoke("list_disliked_videos", { profileId });
}
```

---

### 3. Settings UI — Gestion des dislikes

**Objectif:** Ajouter un panneau "Discovery Dislikes" dans Settings où l'utilisateur peut voir et retirer des vidéos de la blacklist.

#### 3.1 Créer le composant `DislikedVideosPanel`

**Fichier à créer:** `src/components/Settings/DislikedVideosPanel.tsx`

```tsx
import { useState, useEffect } from "react";
import { useProfileStore } from "../../stores/profileStore";
import * as commands from "../../utils/tauriCommands";
import { useToastStore } from "../../stores/toastStore";
import { formatDuration } from "../../utils/fileHelpers";

export function DislikedVideosPanel() {
  const profile = useProfileStore((s) => s.currentProfile);
  const addToast = useToastStore((s) => s.addToast);
  const [disliked, setDisliked] = useState<commands.DislikedVideoInfo[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!profile) return;
    setLoading(true);
    commands.listDislikedVideos(profile.id)
      .then(setDisliked)
      .catch(() => addToast("Failed to load disliked videos", "error"))
      .finally(() => setLoading(false));
  }, [profile?.id, addToast]);

  const handleUndislike = async (videoId: string) => {
    if (!profile) return;
    try {
      await commands.undislikeDiscovery(profile.id, videoId);
      setDisliked((prev) => prev.filter((v) => v.videoId !== videoId));
      addToast("Video removed from dislikes", "success");
    } catch {
      addToast("Failed to remove dislike", "error");
    }
  };

  if (!profile) return null;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-lg font-semibold text-text-primary">Discovery Dislikes</h3>
        <span className="text-sm text-text-muted">{disliked.length} video(s)</span>
      </div>

      {loading && (
        <div className="text-text-muted text-sm">Loading...</div>
      )}

      {!loading && disliked.length === 0 && (
        <div className="text-text-muted text-sm">No disliked videos yet.</div>
      )}

      {!loading && disliked.length > 0 && (
        <div className="space-y-2 max-h-96 overflow-y-auto">
          {disliked.map((video) => (
            <div
              key={video.videoId}
              className="flex items-center justify-between gap-3 p-3 bg-bg-secondary rounded border border-border-primary"
            >
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-text-primary truncate">
                  {video.title}
                </div>
                <div className="text-xs text-text-muted">
                  {video.channel} • {formatDuration(video.duration)}
                </div>
              </div>
              <button
                onClick={() => handleUndislike(video.videoId)}
                className="px-3 py-1 text-xs bg-accent-primary hover:bg-accent-hover text-text-primary rounded transition-colors"
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
```

#### 3.2 Intégrer dans SettingsModal

**Fichier:** `src/components/Settings/SettingsModal.tsx`

Trouver la liste des sections/tabs et ajouter un nouvel onglet "Discovery Dislikes" :

```tsx
import { DislikedVideosPanel } from "./DislikedVideosPanel";

// Dans le rendu des sections/tabs :
{activeTab === "dislikes" && <DislikedVideosPanel />}
```

**Note:** L'emplacement exact dépend de la structure actuelle du SettingsModal. Regarder comment les autres panels (Audio, Keys, etc.) sont intégrés.

---

## Fichiers à créer

| Fichier | Description |
|---------|-------------|
| `src/components/Settings/DislikedVideosPanel.tsx` | Nouveau composant pour gérer la liste des vidéos dislikées |

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/types.rs` | Ajouter `disliked_videos: Vec<String>` à `Profile` |
| `src-tauri/src/commands.rs` | Ajouter 3 commandes : `dislike_discovery`, `undislike_discovery`, `list_disliked_videos` |
| `src-tauri/src/commands.rs:1038-1046` | Filtrer suggestions par `profile.disliked_videos` |
| `src-tauri/src/main.rs:~276` | Registrer les 3 nouvelles commandes |
| `src/components/Discovery/DiscoveryPanel.tsx:536-546` | Changer icône X → thumbs-down |
| `src/components/Discovery/DiscoveryPanel.tsx:295-301` | Renommer `handleDismiss` → `handleDislike`, appeler nouvelle commande |
| `src/utils/tauriCommands.ts:318-320` | Remplacer `dismissDiscovery` par `dislikeDiscovery`, `undislikeDiscovery`, `listDislikedVideos` |
| `src/components/Settings/SettingsModal.tsx` | Intégrer le nouveau panel `DislikedVideosPanel` |

## Notes techniques

### Rétrocompatibilité

L'ajout de `disliked_videos: Vec<String>` avec `#[serde(default)]` assure que les anciens profils sans ce champ seront chargés avec une liste vide.

### Distinction dismiss vs dislike

Après implémentation :
- **Dislike** (bouton thumbs-down) → permanent, stocké dans le profil
- **Dismiss** (ancien X) → peut être complètement supprimé ou gardé comme feature session-only si besoin
- Les deux peuvent coexister si on veut un "skip temporaire" vs "blacklist permanent"

### Fetch vidéo info

La commande `list_disliked_videos` nécessite de récupérer les métadonnées des vidéos via yt-dlp. Il faudra peut-être créer une helper function `youtube::fetch_video_info(url) -> VideoMetadata` dans `src-tauri/src/youtube/downloader.rs` qui extrait juste les infos sans télécharger l'audio.

### Performance

Si la liste de dislikes devient très grande (>100 vidéos), envisager :
- Pagination dans le panel Settings
- Cache côté frontend pour éviter de re-fetch à chaque ouverture
- Index HashSet côté backend pour filtrage O(1) au lieu de Vec scan

---

## Workflow d'implémentation suggéré

### Phase 1 : Backend
- [ ] Ajouter `disliked_videos` à `Profile` struct
- [ ] Créer `dislike_discovery` command
- [ ] Créer `undislike_discovery` command
- [ ] Créer `list_disliked_videos` command
- [ ] Créer helper `youtube::fetch_video_info()` si nécessaire
- [ ] Modifier filtrage discovery pour exclure `disliked_videos`
- [ ] Registrer les commandes dans `main.rs`
- [ ] Tester avec `cargo check` et `cargo test`

### Phase 2 : Frontend — Discovery Panel
- [ ] Ajouter wrappers TypeScript (`dislikeDiscovery`, etc.)
- [ ] Changer icône X → thumbs-down
- [ ] Renommer handler `handleDismiss` → `handleDislike`
- [ ] Tester l'ajout de dislikes dans Discovery

### Phase 3 : Frontend — Settings UI
- [ ] Créer `DislikedVideosPanel.tsx`
- [ ] Intégrer dans `SettingsModal.tsx`
- [ ] Tester l'affichage et suppression de dislikes
- [ ] Vérifier le comportement après undislike (vidéo réapparaît dans futures discoveries)

### Phase 4 : Tests & polish
- [ ] Tester la persistance (fermer/rouvrir app)
- [ ] Tester avec profil legacy (sans `disliked_videos`)
- [ ] Vérifier que les vidéos dislikées ne reviennent jamais
- [ ] Vérifier la suppression du cache audio lors du dislike
- [ ] Polish UI (animations, loading states, erreurs)

---

**Estimation effort:** ~4-6 heures (2h backend, 2h frontend, 1-2h tests/polish)
