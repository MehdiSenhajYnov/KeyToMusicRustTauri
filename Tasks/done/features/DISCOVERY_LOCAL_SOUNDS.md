# Discovery — Support des Sons Locaux comme Seeds

> **Catégorie:** Feature
> **Priorité:** Moyenne
> **Statut:** ✅ Completed
> **Date ajoutée:** 2026-02-02

## Description

Étendre le système Discovery pour que les sons importés localement puissent servir de seeds, au même titre que les sons téléchargés depuis YouTube. Actuellement, seuls les sons avec `SoundSource::YouTube` sont éligibles car le pipeline repose sur les video IDs pour fetcher des YouTube Mix (`&list=RD{id}`).

## Motivation

Un utilisateur qui importe ses sons localement (OST, ambiances, SFX) ne bénéficie pas du tout du système Discovery. Même un profil avec 50 sons locaux affiche "pas assez de seeds". L'idée est de **résoudre automatiquement** un video ID YouTube pour chaque son local via ses métadonnées/nom de fichier, puis de réutiliser le pipeline Mix existant sans modification.

## Approche

### Principe clé : pas besoin de validation

Le cross-seed aggregation filtre naturellement le bruit. Une seed fausse (mauvais match YouTube) génère un Mix de vidéos sans rapport, mais ces vidéos n'apparaissent que dans ce Mix-là (occurrence = 1). Les bonnes suggestions reviennent dans plusieurs Mix et montent en score (4-5+). Le top 30 trié par occurrence évince les orphelines automatiquement.

**Conséquence :** on peut prendre le premier résultat YouTube sans scoring ni validation de durée. La simplicité prime.

---

## Implémentation

### Section 1 — Lecture des métadonnées audio

**Fichiers concernés :**
- `src-tauri/src/audio/analysis.rs` — ajouter une fonction de lecture de tags

**Détails :**

Symphonia est déjà utilisé pour prober les fichiers (`analysis.rs:40-46`, `symphonia_source.rs:56-58`, `buffer.rs:55-57`) mais `MetadataOptions::default()` est passé sans jamais extraire les tags ID3/Vorbis/iTunes. Seuls les `codec_params` (sample_rate, channels, n_frames) sont lus.

Créer une fonction `read_audio_metadata_tags(path: &str) -> Option<AudioTags>` qui :
1. Ouvre le fichier avec Symphonia (`get_probe().format(...)`)
2. Accède à `probed.metadata.get()` puis itère `current()` → `tags()` pour extraire `TrackTitle`, `Artist`, `Album`
3. Retourne un struct simple :

```rust
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
}
```

Les tags sont dans les premiers KB du fichier (headers ID3/MP4 atoms/Vorbis comments). Coût < 1ms par fichier.

**Note :** Symphonia expose les métadonnées via `probed.format.metadata()` et `probed.metadata`. Les tags sont des `Tag` avec `std_key` (`Some(StandardTagKey::TrackTitle)`, `Some(StandardTagKey::Artist)`). Pas besoin de feature Cargo supplémentaire — les codecs activés (`mp3`, `flac`, `ogg`, `wav`, `aac`, `isomp4`) incluent déjà le parsing des métadonnées.

---

### Section 2 — Nettoyage du nom de fichier (fallback)

**Fichiers concernés :**
- `src-tauri/src/discovery/engine.rs` — ajouter une fonction utilitaire

**Détails :**

Quand les tags sont absents ou vides, construire une query à partir du nom de fichier nettoyé :

```
"03_Tokyo_Ghoul_-_Unravel_(OST).mp3" → "Tokyo Ghoul Unravel OST"
```

Fonction `clean_filename_for_search(filename: &str) -> String` :
1. Retirer l'extension (`.mp3`, `.m4a`, `.wav`, etc.)
2. Remplacer `_`, `-`, `.` par des espaces
3. Supprimer les patterns parasites : `\b(copy|final|v\d+|edit|\d{2,3}kbps|track\s?\d+)\b`
4. Supprimer les crochets/parenthèses et leur contenu **sauf** les mots-clés utiles (OST, Soundtrack, Original)
5. Supprimer les numéros isolés en début de chaîne : `^\d+\s+`
6. Collapse des espaces multiples

Si après nettoyage la string est vide ou < 3 caractères → skip ce son (pas de seed).

---

### Section 3 — Résolution YouTube (query cascade)

**Fichiers concernés :**
- `src-tauri/src/discovery/engine.rs` — nouvelle fonction `resolve_local_seeds()`
- `src-tauri/src/youtube/search.rs` — utiliser `search_youtube()` existant (ligne 23)

**Détails :**

Pour chaque son local, construire la query par cascade de qualité :

1. **Tags titre + artiste** → `"{title} {artist}"` (meilleure qualité)
2. **Tag titre seul** → `"{title}"`
3. **Pas de tags** → `clean_filename_for_search(filename)`

Puis appeler `search_youtube(query, 1, yt_dlp_bin, youtube_cache)` (N=1, premier résultat suffit).

Le résultat `YoutubeSearchResult` (`search.rs:10-20`) contient le `video_id` nécessaire pour créer un `SeedInfo`.

**Concurrence :** Résoudre les sons locaux en parallèle (comme les Mix fetches, `buffer_unordered`). Max 5 concurrent pour ne pas spammer yt-dlp.

---

### Section 4 — Cache des video IDs résolus

**Fichiers concernés :**
- `src-tauri/src/types.rs` — ajouter champ optionnel à `Sound`
- `src/types/index.ts` — miroir TypeScript
- `src-tauri/src/storage/profile.rs` — persistance automatique

**Détails :**

Ajouter un champ optionnel au struct `Sound` (`types.rs:44-53`) :

```rust
pub struct Sound {
    pub id: SoundId,
    pub name: String,
    pub source: SoundSource,
    pub momentum: f64,
    pub volume: f32,
    pub duration: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_video_id: Option<String>,  // NEW
}
```

Côté TypeScript (`src/types/index.ts:19-26`) :

```typescript
export interface Sound {
  id: SoundId;
  name: string;
  source: SoundSource;
  momentum: number;
  volume: number;
  duration: number;
  resolvedVideoId?: string;  // NEW
}
```

Après résolution YouTube, stocker le `video_id` dans le son → sauvegardé avec le profil. Les prochains Discovery runs ne re-cherchent pas.

**Invalidation :** Si le fichier source change (nom différent), on pourrait vouloir re-résoudre. Mais en pratique le fichier local ne change pas, donc pas de logique d'invalidation nécessaire.

---

### Section 5 — Intégration dans le pipeline Discovery

**Fichiers concernés :**
- `src-tauri/src/commands.rs` — modifier `start_discovery()` (lignes 913-920)

**Détails :**

Actuellement, l'extraction des seeds dans `start_discovery()` (`commands.rs:913-920`) :

```rust
for sound in &profile.sounds {
    if let SoundSource::YouTube { url, .. } = &sound.source {
        if let Some(video_id) = youtube::downloader::extract_video_id(url) {
            seeds.push(discovery::engine::SeedInfo {
                video_id: video_id.clone(),
                sound_name: sound.name.clone(),
            });
            existing_ids.push(video_id);
        }
    }
}
```

Modifier pour inclure les sons locaux :

```rust
for sound in &profile.sounds {
    match &sound.source {
        SoundSource::YouTube { url, .. } => {
            // Existant — inchangé
            if let Some(video_id) = youtube::downloader::extract_video_id(url) {
                seeds.push(SeedInfo { video_id: video_id.clone(), sound_name: sound.name.clone() });
                existing_ids.push(video_id);
            }
        }
        SoundSource::Local { path } => {
            // NOUVEAU — utiliser resolved_video_id si dispo
            if let Some(ref video_id) = sound.resolved_video_id {
                seeds.push(SeedInfo { video_id: video_id.clone(), sound_name: sound.name.clone() });
            } else {
                // Collecter pour résolution batch
                unresolved_locals.push((sound.id.clone(), path.clone(), sound.name.clone()));
            }
        }
    }
}
```

Ensuite, résoudre les `unresolved_locals` en batch (Section 3), stocker les `resolved_video_id` dans le profil (Section 4), et ajouter les nouvelles seeds au vecteur.

**Événement frontend :** Émettre un événement `discovery_resolving` avec le nombre de sons locaux en cours de résolution, pour afficher un état intermédiaire dans le `DiscoveryPanel` ("Résolution de 5 sons locaux...").

---

### Section 6 — Feedback UI (optionnel)

**Fichiers concernés :**
- `src/components/Discovery/DiscoveryPanel.tsx` — afficher l'état de résolution
- `src/stores/discoveryStore.ts` — ajouter état de résolution

**Détails :**

Pendant la phase de résolution des sons locaux (avant le fetch des Mix), afficher un message type "Identification de 5 sons locaux..." dans le panel Discovery. L'étape est rapide (1 recherche YouTube par son local) mais visible si beaucoup de sons non résolus.

Possibilité d'ajouter dans `SoundDetails.tsx` un indicateur montrant le video ID résolu (ou "non résolu") pour les sons locaux, permettant à l'utilisateur de voir quels sons contribuent au Discovery.

---

## Tâches

### Backend
- [x] Créer `read_audio_metadata_tags()` dans `audio/analysis.rs`
- [x] Créer `clean_filename_for_search()` dans `discovery/engine.rs`
- [x] Créer `resolve_local_seeds()` dans `discovery/engine.rs` (query cascade + search YouTube)
- [x] Ajouter `resolved_video_id: Option<String>` au struct `Sound` dans `types.rs`
- [x] Modifier `start_discovery()` dans `commands.rs` pour inclure les sons locaux
- [x] Sauvegarder les `resolved_video_id` dans le profil après résolution
- [x] Émettre événement `discovery_resolving` pour feedback frontend
- [x] Inclure les `resolved_video_id` dans le calcul du `seed_hash` (cache Discovery)

### Frontend
- [x] Ajouter `resolvedVideoId?: string` au type `Sound` dans `types/index.ts`
- [x] Gérer l'événement `discovery_resolving` dans `discoveryStore.ts`
- [x] Afficher l'état de résolution dans `DiscoveryPanel.tsx`
- [ ] (Optionnel) Indicateur de résolution dans `SoundDetails.tsx`

### Tests
- [ ] Tester `clean_filename_for_search()` avec des noms variés
- [ ] Tester la cascade tags → titre → filename
- [ ] Tester que les seeds fausses ne polluent pas le top 30 (agrégation)

---

## Fichiers à modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/types.rs` | Ajouter `resolved_video_id: Option<String>` à `Sound` (ligne ~52) |
| `src-tauri/src/audio/analysis.rs` | Nouvelle fonction `read_audio_metadata_tags()` |
| `src-tauri/src/discovery/engine.rs` | Fonctions `clean_filename_for_search()` + `resolve_local_seeds()` |
| `src-tauri/src/commands.rs` | Modifier `start_discovery()` (lignes 913-920) pour inclure les Local |
| `src-tauri/src/storage/profile.rs` | Sauvegarder les resolved_video_id après résolution |
| `src-tauri/src/discovery/cache.rs` | Inclure resolved IDs dans seed_hash |
| `src/types/index.ts` | Ajouter `resolvedVideoId?: string` à `Sound` |
| `src/stores/discoveryStore.ts` | Gérer événement `discovery_resolving` |
| `src/components/Discovery/DiscoveryPanel.tsx` | Afficher état de résolution |

## Notes

- Le goulot d'étranglement sera les appels `search_youtube` (réseau + yt-dlp), pas la lecture des tags
- Les sons locaux avec des noms de fichiers inutilisables (ex: `a.mp3`) seront simplement ignorés (query trop courte)
- Si un profil n'a que des sons YouTube, le comportement est 100% identique à l'actuel (aucune régression)
- Le `resolved_video_id` est persisté dans le profil → coût unique par son, jamais re-calculé
- Compatible avec l'import/export .ktm (le champ est sérialisé avec le profil)
