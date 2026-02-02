# Nettoyage des champs Track inutilises

> **Statut:** Completed (2026-02-02)
> **Type:** Fix — Suppression de dette technique
> **Objectif:** Supprimer les 3 champs morts sur le type `Track` (`currentlyPlaying`, `playbackPosition`, `isPlaying`) qui ne sont jamais mis a jour ni lus, tout en gardant la retrocompatibilite avec les profils existants

---

## Probleme

Le type `Track` (profil serialise) contient 3 champs qui ne servent a rien :

```rust
// src-tauri/src/types.rs:71-78
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub volume: f32,
    pub currently_playing: Option<SoundId>,  // ← MORT
    pub playback_position: f64,              // ← MORT
    pub is_playing: bool,                    // ← MORT
}
```

```typescript
// src/types/index.ts:40-47
export interface Track {
  id: TrackId;
  name: string;
  volume: number;
  currentlyPlaying: SoundId | null;  // ← MORT
  playbackPosition: number;          // ← MORT
  isPlaying: boolean;                // ← MORT
}
```

### Pourquoi ils sont morts

L'etat de lecture reel est gere par deux systemes separes qui n'ecrivent **jamais** dans ces champs :

| Donnee | Source reelle | Fichier |
|--------|--------------|---------|
| Son en cours | `AudioTrack.currently_playing` (runtime) | `src-tauri/src/audio/track.rs:17` |
| Position | `audioStore._positions` (mutable map) | `src/stores/audioStore.ts:11` |
| En lecture | `audioStore.playingTracks` (Map) | `src/stores/audioStore.ts:7` |

Les 3 champs du `Track` serialise sont :
- **Ecrits une seule fois** avec des valeurs par defaut (`null`, `0`, `false`) a la creation
- **Jamais mis a jour** — aucun code n'ecrit dedans apres la creation
- **Jamais lus** — aucun composant, hook ou commande ne lit ces champs

### Impact

- **Dette technique** : nouveaux contributeurs confondent `Track.isPlaying` (mort) avec `AudioTrack.is_playing()` (vivant)
- **Espace disque** : 3 champs inutiles serialises dans chaque track de chaque profil JSON et export `.ktm`
- **Bruit** : chaque creation de track exige de passer ces 3 valeurs bidon

---

## Audit complet des usages

### Ecritures (defaults uniquement, jamais mis a jour)

| Fichier | Ligne | Contexte |
|---------|-------|----------|
| `src/components/Tracks/TrackView.tsx` | 65-67 | `addTrack({ ..., currentlyPlaying: null, playbackPosition: 0, isPlaying: false })` |
| `src/components/Sounds/AddSoundModal.tsx` | 675-677 | Meme pattern lors de la creation auto de track |
| `src-tauri/src/commands.rs` | 879-881 | Legacy import, creation de track par defaut |

### Lectures : aucune

Grep sur `*.tsx` et `*.ts` :
- `WaveformDisplay.tsx:21` — `playbackPosition` est un **prop du composant** (position de lecture du curseur), pas le champ Track
- `KeyGrid.tsx:75` — `isPlaying` est une **variable locale** calculee depuis `playingSoundIds`, pas le champ Track
- `SearchResultPreview.tsx:13` — `isPlaying` est un **state local** du composant audio preview
- `SoundDetails.tsx:473` — `playbackPosition={playbackPos}` passe la position depuis `useTrackPosition`, pas depuis Track

**Aucune lecture reelle des champs `Track.currentlyPlaying`, `Track.playbackPosition`, ou `Track.isPlaying`.**

---

## Plan d'implementation

### Phase 1 : Backend Rust

- [x] **1.1** Modifier `src-tauri/src/types.rs:71-78` — Supprimer les 3 champs et ajouter `serde(default)` pour retrocompat :
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct Track {
      pub id: TrackId,
      pub name: String,
      #[serde(default = "default_volume")]
      pub volume: f32,
      // SUPPRIME: currently_playing, playback_position, is_playing
      // Note: serde(deny_unknown_fields) n'est PAS utilise, donc les anciens
      // champs dans les JSON existants seront simplement ignores a la deserialisation.
  }

  fn default_volume() -> f32 { 1.0 }
  ```
  **Note retrocompat** : serde ignore par defaut les champs inconnus lors de la deserialisation. Les profils existants contenant `currentlyPlaying`/`playbackPosition`/`isPlaying` se chargeront sans erreur — les champs seront simplement ignores.

- [x] **1.2** Modifier `src-tauri/src/commands.rs:875-882` — Supprimer les 3 champs de la creation de track dans `import_legacy_save` :
  ```rust
  let track = crate::types::Track {
      id: track_id.clone(),
      name: "OST".to_string(),
      volume: 1.0,
      // Plus besoin de currently_playing, playback_position, is_playing
  };
  ```

- [x] **1.3** Verifier la compilation : `cargo clippy --manifest-path src-tauri/Cargo.toml`

### Phase 2 : Frontend TypeScript

- [x] **2.1** Modifier `src/types/index.ts:40-47` — Supprimer les 3 champs :
  ```typescript
  export interface Track {
    id: TrackId;
    name: string;
    volume: number;
  }
  ```

- [x] **2.2** Modifier `src/components/Tracks/TrackView.tsx:63-67` — Retirer les 3 champs de `addTrack()` :
  ```typescript
  addTrack({
    id: trackId,
    name: newTrackName.trim(),
    volume: 1.0,
  });
  ```

- [x] **2.3** Modifier `src/components/Sounds/AddSoundModal.tsx:673-677` — Meme nettoyage :
  ```typescript
  addTrack({
    id: trackId,
    name: newTrackName.trim(),
    volume: 1.0,
  });
  ```

### Phase 3 : Verification

- [ ] **3.1** `npm run tauri dev` — Verifier que l'app demarre
- [ ] **3.2** Charger un profil existant (avec les anciens champs dans le JSON) — verifier qu'il charge sans erreur
- [ ] **3.3** Creer un nouveau track — verifier que le JSON sauvegarde ne contient plus les 3 champs
- [ ] **3.4** Import legacy — verifier que l'import fonctionne toujours
- [ ] **3.5** Export/Import `.ktm` — verifier le round-trip

---

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/types.rs` | Supprimer `currently_playing`, `playback_position`, `is_playing` du struct `Track` |
| `src-tauri/src/commands.rs` | Retirer les 3 champs de la creation de track (legacy import, ligne 879-881) |
| `src/types/index.ts` | Supprimer `currentlyPlaying`, `playbackPosition`, `isPlaying` de l'interface `Track` |
| `src/components/Tracks/TrackView.tsx` | Retirer les 3 champs du call `addTrack()` (ligne 65-67) |
| `src/components/Sounds/AddSoundModal.tsx` | Retirer les 3 champs du call `addTrack()` (ligne 675-677) |

---

## Risques

- **Retrocompatibilite** : Zero risque. serde ignore les champs inconnus par defaut (`deny_unknown_fields` n'est pas utilise). Les profils existants se chargeront normalement, les champs morts seront juste ignores.
- **Export .ktm** : Les anciens `.ktm` avec ces champs s'importeront sans probleme (meme raison).
- **AudioTrack** : Le struct `audio::track::AudioTrack` a ses propres `currently_playing` et `is_playing()` — ils ne sont PAS affectes par ce nettoyage. Ce sont des champs runtime du moteur audio, pas des champs serialises.
