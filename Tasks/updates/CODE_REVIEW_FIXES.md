# Code Review Fixes

Issues identified during full codebase analysis. Verified against actual code.

---

## Bugs actifs

1. ~~**`useTextInputFocus` ecrase l'etat manual** — `useTextInputFocus.ts:43-46` — reactive la detection de touches meme si l'utilisateur l'avait desactivee manuellement~~ **FIXED** — checks `keyDetectionEnabled` before disabling
2. ~~**Stale closure** — `AddSoundModal.tsx:91-100` — `handleStopPreview` manque dans les deps du useEffect, utilise un `selectedTrackId` perime~~ **FIXED** — moved `handleStopPreview` before useEffect, added to deps, used ref for unmount cleanup
3. ~~**`ProfileSummary` type mismatch** — `tauriCommands.ts:4-11` attend `sound_count` et `track_count` que le backend n'envoie pas~~ **FIXED** — removed dead fields

## Crashes potentiels

4. ~~**`dirs::data_dir().unwrap()`** — `storage/config.rs:9,15,21` — panic au demarrage si le repertoire OS est indetermine~~ **FIXED** — replaced with `.expect()` with clear message
5. **48+ `lock().unwrap()`** — partout — crash irreparable si un thread panic pendant qu'il tient un mutex — **DEFERRED** (needs separate audit)
6. **`panic!()` explicite** si l'audio engine fail au demarrage — `main.rs:65` — intentionnel (app can't function without audio)
7. ~~**`.expect()` sur `rdev::listen`** — `detector.rs:211` — crash du thread key detection sur Linux~~ **FIXED** — replaced with `if let Err` + warning log

## Performance

8. ~~**Pas de debounce sur les sliders volume** — `Header.tsx:28`, `TrackView.tsx:148`, `SoundDetails.tsx:192` — flood d'appels IPC~~ **FIXED** — 100ms debounce on backend IPC calls
9. **`KeyGrid` re-render tous les 100ms** — subscribe a `playingTracks` pour le statut playing — **DEFERRED** (needs perf profiling)
10. **Non-selective store subscriptions** — **DEFERRED** (needs perf profiling)
11. ~~**Logs `info!` sur chaque keypress** — `windows_listener.rs:194,206`~~ **FIXED** — changed to `debug!`

## Fuites memoire

12. ~~**Timers non cleares a l'unmount** — `SoundDetails.tsx:29-30`, `TrackView.tsx:16`~~ **FIXED** — added cleanup useEffects

## Robustesse

13. ~~**Updates optimistes sans rollback** — tous les setters de `settingsStore.ts`~~ **FIXED** — all setters now rollback on error + show toast
14. ~~**Ecriture fichier non-atomique** — `config.rs`, `profile.rs`~~ **FIXED** — write-to-tmp-then-rename pattern
15. **Validation UUID sur profile IDs** — **DEFERRED** (low risk in practice)
16. **Verification integrite binaires telecharges** — **DEFERRED** (complex external integration)

## Dead code a nettoyer

17. ~~**`errors.rs` entier** + dependance `thiserror`~~ **FIXED** — file deleted, dep removed
18. ~~**Dependances Cargo inutilisees** : `walkdir`, `rand`, `sanitize-filename`~~ **FIXED** — removed from Cargo.toml
19. ~~**`BufferManager`** — 6 methodes mortes~~ **FIXED** — removed dead methods, simplified struct
20. ~~**Cooldown backend** — `cooldown_ms`, `check_cooldown()`, `update_trigger_time()`, `last_trigger_time`~~ **FIXED** — removed
21. ~~**`create_track()`, `remove_track()`, `drain_events()`**~~ **FIXED** — removed methods (kept AudioCommand variants used by audio thread)
22. ~~**`profile_exists()`, `code_to_key()`, `is_modifier()`, `is_enabled()`**~~ **FIXED** — removed
23. ~~**`static mut RAW_INPUT_HWND`**~~ **FIXED** — removed

## Duplication de code

24. ~~**`getSoundFilePath`** — copie dans 4 fichiers~~ **FIXED** — extracted to `src/utils/soundHelpers.ts`
25. ~~**Detection text input** — dupliquee dans 3 hooks~~ **FIXED** — extracted to `src/utils/inputHelpers.ts`
26. ~~**Device switch resume** — ~60 lignes copiees 2x dans `engine.rs`~~ **FIXED** — extracted `capture_and_stop_tracks()` and `resume_tracks()` helpers
27. **Key capture logic** — dupliquee entre `KeyCaptureSlot.tsx` et `SoundDetails.tsx` — **DEFERRED** (large refactor, risk of regression)
28. **Momentum editor UI** — duplique entre `AddSoundModal.tsx` et `SoundDetails.tsx` — **DEFERRED** (false positive per exploration)
29. ~~**Array `shortcuts`** — identique dans `SettingsModal.tsx` et `KeyGrid.tsx`~~ **FIXED** — extracted `buildShortcutsList()` in keyMapping.ts

## Structure / Architecture

30. **`commands.rs` monolithique** (~812 lignes) — **DEFERRED** (large structural refactor)
31. **Legacy import** embarque dans `commands.rs` — **DEFERRED** (large structural refactor)
32. ~~**`MomentumModifierType`** defini 2 fois~~ **FIXED** — unified via re-export from types
33. **Langues melangees** — UI et commentaires en francais et anglais — **DEFERRED** (cosmetic)

## Tests

34. **5 tests unitaires** dans tout le projet — **DEFERRED** (separate effort)

---

## Retires apres verification (faux positifs)

~~**Shallow clone undo/redo**~~ — `historyStore.ts` utilise le spread operator (clone superficiel) mais le code zustand ne mute jamais les objets en place — pas de risque reel.

~~**Bug precedence preventDefault**~~ — `useKeyDetection.ts` ne bloque que les touches avec un binding reel. Shift/Ctrl/Alt seuls n'ont pas de binding, donc ne sont pas bloques.

~~**Division par zero crossfade**~~ — `crossfade.rs` — l'UI enforce 100-2000ms, la valeur 0 ne peut pas arriver. Et en float, division par zero donne `Inf`, pas `NaN`.

~~**`sound_volumes` / `track_sounds` memory leak**~~ — `engine.rs` — bornes par max 20 tracks, `insert()` remplace les entrees existantes. Pas de fuite.
