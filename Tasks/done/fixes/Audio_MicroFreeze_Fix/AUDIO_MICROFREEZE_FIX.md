# Audio Micro-Freeze Fix

**Status:** Completed
**Completed:** 2026-02-01

## Probleme

Mini-freezes/glitches intermittents dans la lecture audio. Analyse complete du systeme audio revele plusieurs causes potentielles.

---

## Causes identifiees (par priorite)

### P0 - CRITIQUE

#### 1. Decodage synchrone dans le callback audio
**Fichier:** `src-tauri/src/audio/symphonia_source.rs:174-189`

Quand le buffer de samples est epuise, `decode_next_packet()` est appele directement dans `Iterator::next()` — sur le thread audio temps reel. Zero pre-buffering : un seul paquet decode a l'avance. Si le decodage prend trop longtemps (disque lent, CPU occupe, fichier complexe), le thread audio est affame → micro-freeze.

```rust
fn next(&mut self) -> Option<f32> {
    if self.sample_pos >= self.sample_buf.len() {
        self.decode_next_packet();  // BLOQUE LE THREAD AUDIO
    }
    // ...
}
```

**Fix:** Implementer un buffer ring/queue avec 3-5 paquets d'avance, decodes dans un thread separe.

---

#### 2. I/O fichier sur le thread audio
**Fichier:** `src-tauri/src/audio/engine.rs:280-327`

`SymphoniaSource::new()` execute sur le thread audio :
- `File::open()` — ouverture fichier
- Probe du format (symphonia)
- Seek si momentum > 0
- Decodage du premier paquet

Avec momentum, peut prendre 50-200ms → freeze perceptible.

**Fix:** Pre-creer la source dans un thread separe avant de l'envoyer au thread audio.

---

### P1 - HIGH

#### 3. Polling peripherique audio toutes les 3s
**Fichier:** `src-tauri/src/audio/engine.rs:454-469`

Quand `audioDevice = None`, le thread audio appelle `cpal::default_output_device()` toutes les 3 secondes. Sur Windows, l'enumeration peut prendre 10-50ms, interrompant le thread audio.

**Fix:** Cacher le resultat de `get_default_device_name()`, ne rafraichir que sur changement notifie.

---

#### 4. Pool CPU de 2 threads partagee
**Fichier:** `src-tauri/src/state.rs:32-38`

Le pool rayon n'a que 2 threads pour tout le travail CPU (waveforms, durees, predownload discovery). Pendant le predownload discovery, 10+ taches bloquantes se disputent 2 threads.

**Fix:** Augmenter le pool a 4 threads.

---

#### 5. Contention mutex sur le canal d'evenements
**Fichier:** `src-tauri/src/main.rs:154-225`

Le thread de forwarding d'evenements tient un Mutex lock pendant la serialisation JSON et l'emission IPC. Peut retarder l'envoi d'evenements depuis le thread audio.

**Fix:** Drainer les evenements sans tenir le mutex pendant l'emission.

---

### P2 - MEDIUM

#### 6. Cache waveform ecrit sur disque a chaque insertion
**Fichier:** `src-tauri/src/audio/analysis.rs:506, 564-567`

`save_to_disk()` appele a chaque insertion avec serialisation JSON + ecriture fichier synchrone.

**Fix:** Debouncer les ecritures disque (toutes les 5s max).

---

#### 7. `file_mtime()` appele a chaque lookup du cache
**Fichier:** `src-tauri/src/audio/analysis.rs:471-479`

Chaque lecture du cache fait un appel systeme `stat()` pour verifier si le fichier a change.

**Fix:** Cacher le mtime, invalider sur modification ou avec TTL.

---

#### 8. Boucle de decodage non bornee
**Fichier:** `src-tauri/src/audio/symphonia_source.rs:118-168`

`decode_next_packet()` boucle indefiniment pour trouver un paquet valide (skip des paquets d'autres pistes, paquets corrompus). Pour certains fichiers, peut boucler 10-50+ fois.

**Fix:** Ajouter une limite d'iterations ou filtrer en amont.

---

#### 9. Courbe de crossfade avec creux de volume
**Fichier:** `src-tauri/src/audio/crossfade.rs:25-54`

La courbe 35%-65% cree un gap ou le volume total tombe a ~30%. Pas un freeze mais un artefact audible.

**Fix:** Utiliser une courbe equal-power (sin/cos) ou au minimum garder la somme a 100%.

---

#### 10. Pas de ramping de volume
**Fichier:** `src-tauri/src/audio/track.rs:120-138`

`sink.set_volume()` est instantane — pas de rampe de 5-10ms. Peut causer des clicks/pops.

**Fix:** Implementer un ramping lineaire sur 5-10ms pour chaque changement de volume.

---

## Plan d'implementation

### Phase 1 : Buffer asynchrone pour SymphoniaSource (P0)
- [x] Creer un thread de decodage separe avec channel bounded (4 paquets)
- [x] `Iterator::next()` lit depuis le channel sans bloquer (try_recv, silence si vide)
- [x] Pre-decoder le premier buffer a l'initialisation
- [x] Gerer proprement l'arret (stop_flag AtomicBool, Drop impl) et le seek
- [x] Limiter les iterations de decodage a 50 (MAX_DECODE_ITERATIONS)

### Phase 2 : Pre-creation des sources hors thread audio (P0)
- [x] Ajouter `PlaySoundPrepared` a AudioCommand (accepte une source pre-creee)
- [x] Creer `SymphoniaSource` dans `commands::play_sound` avant d'envoyer au thread audio
- [x] Gerer le cas momentum (seek fait avant envoi, start_position transmis pour le tracking)
- [x] Ajouter `play_prepared()` a AudioTrack

### Phase 3 : Corrections secondaires (P1-P2)
- [x] Augmenter le pool CPU a 4 threads
- [x] Augmenter l'intervalle de polling device a 5s
- [x] Courbe crossfade equal-power (cos/sin)
- [x] Protection division par zero dans crossfade
- [x] Ramping de volume (target_volume + tick_volume_ramp)

### Deja fait (avant cette tache)
- [x] Debounce `save_to_disk()` du cache waveform (dirty flag + flush 5s)
- [x] Cache mtime (valide seulement au chargement profil, pas par acces)
- [x] Drainer les evenements sans mutex pendant l'emission (try_recv drain loop)

---

## Fichiers concernes

| Fichier | Modifications |
|---------|--------------|
| `src-tauri/src/audio/symphonia_source.rs` | Buffer async, limite de boucle, silence sur buffer vide |
| `src-tauri/src/audio/engine.rs` | PlaySoundPrepared, volume ramp tick, device poll 5s |
| `src-tauri/src/audio/track.rs` | play_prepared(), volume ramping (target/current/tick) |
| `src-tauri/src/audio/crossfade.rs` | Courbe equal-power, guard div-by-zero |
| `src-tauri/src/state.rs` | Pool CPU 4 threads |
| `src-tauri/src/commands.rs` | Pre-creation source dans play_sound |
