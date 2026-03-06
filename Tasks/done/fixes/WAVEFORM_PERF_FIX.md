# Waveform & Momentum — Performance Fix

> **Categorie:** Bug Fix / Optimisation
> **Priorite:** Haute
> **Statut:** ✅ Completed
> **Date ajoutee:** 2026-02-02
> **Date completee:** 2026-02-02

## Description

Problemes de performance lies au calcul des waveforms et du momentum suggere. Le pool CPU partage (4 threads) est sature par les operations concurrentes (discovery predownloads, chargement profil, ajout de sons), causant des latences visibles dans l'UI.

## Motivation

Quand l'utilisateur ajoute un son (AddSoundModal), le waveform met du temps a s'afficher et le momentum suggere arrive en retard. Le probleme est amplifie quand la discovery tourne en arriere-plan (3 predownloads concurrents soumettent chacun 2 taches au pool de 4 threads = saturation). Le chargement d'un gros profil (50+ sons) monopolise aussi le pool pendant 1-3 secondes.

---

## Causes identifiees (par priorite)

### P0 - CRITIQUE

#### 1. Saturation du pool CPU par la discovery
**Fichier:** `src-tauri/src/commands.rs:1198-1212`

Chaque `predownload_suggestion` soumet 2 taches en parallele via `tokio::join!` (duree + waveform). Avec 3 predownloads simultanes (limite frontend), ca fait 6 taches pour un pool de 4 threads. Les waveforms utilisateur (AddSoundModal, SoundDetails) sont en queue derriere.

```rust
// commands.rs:1198-1212 — Les 2 taches se disputent le pool
let (duration_result, waveform_result) = tokio::join!(
    tokio::task::spawn_blocking(move || {
        pool_dur.install(|| BufferManager::get_audio_duration(&duration_path).unwrap_or(0.0))
    }),
    async {
        if !need_waveform {
            return Ok(None);
        }
        tokio::task::spawn_blocking(move || {
            pool_wf.install(|| analysis::compute_waveform_sampled(&wf_path, 150).map(Some))
        })
        .await
        .map_err(|e| format!("Waveform task failed: {}", e))?
    }
);
```

**Impact:** Les waveforms utilisateur (AddSoundModal) attendent 100-500ms derriere les taches discovery.
**Fix:** Semaphore pour limiter discovery a 1 waveform concurrent + sequentialiser duree puis waveform.

---

#### 2. `par_iter()` non borne au chargement de profil
**Fichier:** `src-tauri/src/commands.rs:300-307` (`preload_profile_sounds`)

```rust
// commands.rs:300 — Soumet potentiellement 50+ taches d'un coup
pool.install(|| {
    needs_work.par_iter().for_each(|entry| {
        if gen_counter.load(Ordering::SeqCst) != gen {
            return;
        }
        // ... BufferManager::get_audio_duration() par son
    });
});
```

**Fichier:** `src-tauri/src/commands.rs:496-507` (`get_waveforms_batch`)

```rust
// commands.rs:496 — Meme probleme pour les waveforms en batch
pool.install(|| {
    to_compute.par_iter().for_each(|entry| {
        if gen.load(Ordering::SeqCst) != current_gen {
            return;
        }
        if let Ok(data) = analysis::compute_waveform_sampled(&entry.path, entry.num_points) {
            new_results.lock().unwrap().insert(entry.path.clone(), data);
        }
    });
});
```

**Impact:** Un profil de 50 sons soumet 50 taches `par_iter` qui monopolisent le pool 1-3 secondes. Tout autre travail (waveform utilisateur, discovery) est bloque.
**Fix:** Borner le parallelisme a des chunks de 3-4 via `par_chunks()` ou chunking manuel.

---

### P1 - HIGH

#### 3. Waveform discovery calcule systematiquement (meme si jamais affiche)
**Fichier:** `src-tauri/src/commands.rs:1207` (dans `predownload_suggestion`)

```rust
pool_wf.install(|| analysis::compute_waveform_sampled(&wf_path, 150).map(Some))
```

**Fichier:** `src/hooks/useDiscoveryPredownload.ts:59-112`

Le frontend predownload une fenetre de 6 suggestions [current-2, current+3] + 2 du pool. Chaque predownload declenche un `compute_waveform_sampled` de 150 points cote backend, meme si l'utilisateur ne verra jamais cette suggestion dans le carousel.

**Impact:** ~50% du travail CPU discovery est gaspille sur des waveforms jamais affiches.
**Fix:** Rendre le waveform lazy — ne le calculer que quand la suggestion est visible dans le carousel.

---

#### 4. Appels IPC individuels au lieu du batch existant
**Fichier:** `src/components/Sounds/SoundDetails.tsx:95-114`

```typescript
// SoundDetails.tsx:109-112 — N appels individuels
commands.getWaveform(path, 200).then((data) => {
  setLocalWaveforms((prev) => new Map(prev).set(path, data));
  useWaveformStore.getState().setOne(path, data);
}).catch(() => {});
```

La commande `getWaveformsBatch` existe (`tauriCommands.ts:178-180`) mais n'est pas utilisee. SoundDetails fait N appels `getWaveform()` individuels.

**Fichier:** `src/components/Sounds/AddSoundModal.tsx:207-213`

```typescript
// AddSoundModal.tsx:207-213 — N appels getAudioDuration sans limite
for (const path of initialFiles) {
  commands.getAudioDuration(path)
    .then((duration) => {
      setFiles((prev) =>
        prev.map((f) => (f.path === path && f.duration === 0 ? { ...f, duration } : f))
      );
    }).catch(() => {});
}
```

20 fichiers = 20 appels IPC simultanes sans concurrency limit.

**Impact:** Overhead IPC + flood du backend + state update thrashing (chaque duree → `setFiles` → re-render).
**Fix:** Utiliser `getWaveformsBatch()` dans SoundDetails. Limiter la concurrence des `getAudioDuration` a ~5 simultanes dans AddSoundModal.

---

## Plan d'implementation

### Phase 1 : Semaphore discovery (P0)
- [x] Ajouter un `tokio::Semaphore` (permits=1) dans `AppState` pour les waveforms discovery
- [x] Dans `predownload_suggestion` (`commands.rs:1190-1225`), acquérir le semaphore avant le calcul waveform
- [x] Sequentialiser : d'abord duree, puis waveform (au lieu de `tokio::join!`)
- [x] Le semaphore ne s'applique PAS aux `get_waveform`/`get_waveforms_batch` (travail utilisateur = prioritaire)

### Phase 2 : Borner les par_iter (P0)
- [x] Dans `preload_profile_sounds` (`commands.rs:300`), remplacer `needs_work.par_iter().for_each()` par un chunking borne (chunks de 4 max)
- [x] Dans `get_waveforms_batch` (`commands.rs:496`), meme traitement : `to_compute.par_iter()` → chunks de 4
- [x] S'assurer que le check `profile_load_gen` est fait a chaque chunk (pas seulement au debut)

### Phase 3 : Waveform discovery lazy (P1)
- [x] Modifier `predownload_suggestion` (`commands.rs:1160-1235`) pour ne plus calculer de waveform — retourner `waveform: None`
- [x] Adapter `PredownloadResult` (`types.rs` ou `commands.rs`) : `waveform` devient `Option<WaveformData>`
- [x] Cote frontend dans `discoveryStore.ts:416-430` (`setPredownloadStatus`), gerer `waveform: null`
- [x] Dans `useDiscoveryPredownload.ts`, apres le predownload `"ready"`, declencher `getWaveform(cachedPath, 150)` uniquement pour les suggestions dans la fenetre visible [current-1, current+1]
- [x] Stocker le waveform dans `EnrichedSuggestion.waveform` (`discoveryStore.ts:28`) au retour
- [x] L'UI (DiscoveryPanel) affiche un loader waveform le temps du calcul

### Phase 4 : Batching frontend (P1)
- [x] Dans `SoundDetails.tsx:95-114`, collecter tous les paths non caches et appeler `getWaveformsBatch()` (`tauriCommands.ts:178`) en un seul appel IPC
- [x] Dans `AddSoundModal.tsx:207-227`, implementer un semaphore frontend (max 5 concurrent `getAudioDuration`) ou creer une commande batch `get_audio_durations_batch` cote backend
- [x] Meme pattern pour `AddSoundModal.tsx:507-515` (browse files)

---

## Fichiers concernes

| Fichier | Modifications |
|---------|--------------|
| `src-tauri/src/state.rs:13-29` | Ajouter `discovery_waveform_sem: Arc<tokio::Semaphore>` dans `AppState` |
| `src-tauri/src/commands.rs:1160-1235` | Semaphore + sequentialiser duree/waveform + retirer waveform (lazy) |
| `src-tauri/src/commands.rs:270-326` | Borner `par_iter` dans `preload_profile_sounds` |
| `src-tauri/src/commands.rs:453-522` | Borner `par_iter` dans `get_waveforms_batch` |
| `src/hooks/useDiscoveryPredownload.ts:59-112` | Declencher `getWaveform` lazy pour suggestions visibles |
| `src/stores/discoveryStore.ts:16-33, 416-430` | Gerer `waveform: null` dans `EnrichedSuggestion` et `setPredownloadStatus` |
| `src/components/Sounds/SoundDetails.tsx:95-114` | Remplacer N `getWaveform()` par 1 `getWaveformsBatch()` |
| `src/components/Sounds/AddSoundModal.tsx:207-227` | Limiter concurrence `getAudioDuration` (max 5) |

## Notes

- Le pool CPU reste a 4 threads (augmente de 2→4 dans le MicroFreeze Fix). L'objectif ici n'est pas d'augmenter les threads mais de mieux repartir le travail.
- Le `profile_load_gen` (annulation des operations obsoletes) est deja en place dans `preload_profile_sounds` et `get_waveforms_batch` — il suffit de s'assurer que le check est fait a chaque chunk.
- Les 4 phases sont TOUTES essentielles et doivent etre implementees. Aucune n'est optionnelle.
