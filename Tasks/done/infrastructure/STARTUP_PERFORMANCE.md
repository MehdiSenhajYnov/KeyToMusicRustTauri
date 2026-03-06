# Optimisation des Performances au Demarrage

> **Categorie:** Amelioration / Optimisation
> **Priorite:** Haute
> **Statut:** ✅ Terminé
> **Date ajoutee:** 2026-02-02

## Description

L'application affiche un ecran noir pendant ~300-750ms au demarrage avant que l'UI soit interactive. Ce delai vient d'initialisations synchrones cote Rust (enumeration audio cpal, chargement des caches) et d'une cascade d'appels IPC sequentiels cote React. L'objectif est de reduire le temps percu a quasi-zero via un skeleton instantane et de reduire le temps reel a ~50-100ms.

## Motivation

L'utilisateur voit une fenetre noire vide pendant une duree perceptible a chaque lancement. Pour une app soundboard utilisee en lecture manga (ouverture/fermeture frequentes), la rapidite du demarrage est critique pour le confort d'usage.

## Diagnostic

### Backend Rust — Operations bloquantes avant affichage fenetre

| Etape | Duree estimee | Fichier | Ligne |
|-------|---------------|---------|-------|
| Logging + repertoires | ~15-40ms | `src-tauri/src/main.rs` | L45-49 |
| Config load (sync) | ~20-50ms | `src-tauri/src/storage/config.rs` | `load_config()` L41-54 |
| **Audio engine init (cpal)** | **100-500ms** | `src-tauri/src/audio/engine.rs` | `AudioEngineHandle::new()` L111-128, `create_output_stream()` L244-261 |
| YouTube cache load (sync) | ~10-50ms | `src-tauri/src/youtube/cache.rs` | `load_index()` L44-65 |
| Waveform cache load (sync) | ~30-100ms | `src-tauri/src/audio/analysis.rs` | `new_with_disk()` L655-665, `load_from_disk()` L732-762 |
| **Total bloquant** | **~195-760ms** | | |

Le pire coupable : `create_output_stream()` dans `engine.rs:244-261` qui appelle `cpal::default_host()` puis `host.output_devices()` pour enumerer les peripheriques audio. Sur Windows avec plusieurs devices USB, ca bloque 200-500ms.

### Frontend React — Cascade d'appels sequentiels

```
t=0     App mount (App.tsx:23), 6 hooks initialises (L30-35)
t=0     useEffect #1 (L38-41): loadConfig() + loadProfiles()  <- AWAIT
t=50ms  config.currentProfileId disponible
t=50ms  useEffect #2 (L44-48): loadProfile(id)                <- AWAIT cascade
t=150ms Profile charge -> UI interactive
        fire-and-forget: verifyProfileSounds, computeProfileDurations, preloadWaveforms
```

3 appels Tauri sequentiels dans `App.tsx:38-58` :
- `getConfig()` via `settingsStore.ts:48`
- `listProfiles()` via `profileStore.ts:99`
- `loadProfile(id)` via `profileStore.ts:126`

### Fenetre et HTML

- `tauri.conf.json:16-28` : `visible` non defini (defaut `true`), pas de splash screen
- `index.html:10` : `<div id="root"></div>` vide, aucun contenu statique
- `src/index.css:73-85` : body `background-color: #0f0f0f`, `#root` flex column plein ecran
- Aucun code splitting (`vite.config.ts`), tous les composants charges d'un bloc

---

## Plan d'implementation

### Phase 1 : Perception instantanee (fenetre)

- [x] **1.1** Ajouter un skeleton CSS-only dans `index.html`
  - [x] Ajouter du HTML statique dans `<div id="root">` representant le layout (header, sidebar, zone principale)
  - [x] Utiliser des blocs gris avec animation `@keyframes pulse` (shimmer)
  - [x] Couleurs matching le theme sombre : fond `#0f0f0f`, surfaces `#1a1a2e` / `#252525`
  - [x] Le skeleton est automatiquement remplace quand React hydrate `#root` via `createRoot()`
  - [x] Pas de JS necessaire — pur HTML/CSS inline dans `index.html`

- [x] **1.2** Cacher la fenetre puis la montrer quand React est pret
  - [x] Ajouter `"visible": false` dans `tauri.conf.json` (section `windows`, L16-28)
  - [x] Ajouter `"core:window:allow-show"` dans `capabilities/default.json` (L6-12)
  - [x] Dans `src/main.tsx` (L6-10) : appeler `getCurrentWindow().show()` apres `ReactDOM.createRoot().render()` via double `requestAnimationFrame` post-render
  - [x] Le skeleton de 1.1 est le premier contenu visible quand la fenetre apparait

- [x] **1.3** Ajouter un fade-in CSS sur le contenu React
  - [x] Dans `src/index.css` : ajouter transition `opacity 0.25s ease-out`
  - [x] Appliquer sur le contenu React (pas le skeleton HTML) une transition `opacity 0.25s ease-out`
  - [x] Dans `src/main.tsx` : ajouter une classe `.loaded` sur `#root` apres le premier render
  - [x] Le skeleton HTML reste visible (opacity 1) et le contenu React fait un fade-in par-dessus

### Phase 2 : Defer audio engine (backend Rust)

- [x] **2.1** Deplacer l'initialisation de `AudioEngineHandle` apres la creation de la fenetre
  - [x] Dans `src-tauri/src/main.rs` : remplacer l'init synchrone par un init differe via `tokio::spawn`
  - [x] Modifier `AppState` (`state.rs`) : changer `audio_engine: AudioEngineHandle` en `audio_engine: Arc<tokio::sync::OnceCell<AudioEngineHandle>>`
  - [x] Dans le setup hook (`main.rs`) : lancer l'init audio dans un `tokio::spawn` qui remplit le `OnceCell`
  - [x] L'init appelle `AudioEngineHandle::new()` qui spawne le thread audio et `create_output_stream()`
  - [x] Deplacer `audio_engine.set_master_volume()` dans le meme spawn
  - [x] Event forwarding thread demarre dans le tokio::spawn apres init engine

- [x] **2.2** Adapter les commandes audio pour gerer l'engine pas encore pret
  - [x] Dans `commands.rs` : toutes les commandes audio utilisent `state.get_audio_engine()?` qui retourne une erreur si pas pret
  - [x] `set_master_volume` et `update_config` utilisent `if let Ok(engine)` (graceful, pas d'erreur si engine pas pret)
  - [x] L'init audio prend ~100-500ms, donc en pratique l'engine sera pret avant que l'user puisse declencher un son

### Phase 3 : Commande unifiee get_initial_state

- [x] **3.1** Creer la commande Rust `get_initial_state`
  - [x] Dans `commands.rs` : struct `InitialState` avec `#[serde(rename_all = "camelCase")]` retournant config, profiles, current_profile
  - [x] Implementation : lit `state.config`, appelle `storage::list_profiles()`, charge le profil courant si `current_profile_id` existe
  - [x] Commande enregistree dans le builder Tauri

- [x] **3.2** Ajouter le wrapper TypeScript
  - [x] Dans `src/utils/tauriCommands.ts` : `getInitialState()` qui `invoke("get_initial_state")`
  - [x] Type `InitialState` defini dans `src/types/index.ts`

- [x] **3.3** Remplacer la cascade d'effects dans `App.tsx`
  - [x] Remplace par un seul `useEffect` qui appelle `getInitialState()` une fois
  - [x] A la reception : setter `settingsStore.config`, `profileStore.profiles/currentProfile` en une fois
  - [x] Background tasks (verifyProfileSounds, durations, waveforms) lances via `bgTasksDone` ref
  - [x] useEffect de sync bindings conserve

### Phase 4 : Lazy loading des caches

- [x] **4.1** Lazy load du YouTube cache
  - [x] Dans `youtube/cache.rs` : champ `loaded: bool` ajoute, initialise a `false`
  - [x] Appel `youtube_cache.load_index()` supprime de `main.rs`
  - [x] Methode `ensure_loaded(&mut self)` ajoutee, appelee dans `get()`, `add_entry()`, `remove_entry_by_video_id()`
  - [x] Callers externes (`delete_profile`, `import_profile`, `dismiss_discovery`) appellent aussi `ensure_loaded()`

- [x] **4.2** Lazy load du waveform cache
  - [x] Dans `audio/analysis.rs` : `new_with_disk()` n'appelle plus `load_from_disk()`
  - [x] Champ `loaded: bool` ajoute au struct `WaveformCache`
  - [x] Methode `ensure_loaded(&mut self)` ajoutee, appelee dans `get()` et `insert()`
  - [x] `flush_if_dirty()` fonctionne sans changement (dirty flag set uniquement apres insert qui appelle ensure_loaded)

### Phase 5 : Paralleliser les loads Rust restants

- [x] **5.1** Paralleliser ce qui reste apres les taches precedentes
  - [x] `load_config()` et `cleanup_interrupted_export()` parallelises avec `std::thread::scope`
  - [x] `init_app_directories()` reste en premier (prerequis)
  - [x] `KeyDetector::new()` reste sequentiel (depend de config, <1ms)

### Phase 6 : Skeletons React pour les loading states

- [x] **6.1** Ajouter un flag `isLoading` aux stores
  - [x] `settingsStore.ts` : `isInitialized: false` ajoute, passe a `true` dans `setConfig()` et `loadConfig()`
  - [x] `profileStore.ts` : `isLoading: true` par defaut, `false` quand profil charge ou erreur

- [x] **6.2** Creer des composants skeleton
  - [x] `MainContent.tsx` : composant `MainContentSkeleton` avec blocs tracks + grille touches
  - [x] Skeleton affiche quand `isLoading` est true, avec `animate-pulse`
  - [x] Header se rend normalement (valeurs par defaut)

- [x] **6.3** Transition skeleton -> contenu reel
  - [x] Skeletons disparaissent quand `isLoading` passe a `false`
  - [x] Layout skeleton matche les dimensions du layout reel

### Phase 7 : Code splitting

- [x] **7.1** Lazy load des modals et composants lourds
  - [x] `AddSoundModal` : `React.lazy(() => import(...))` dans `MainContent.tsx`
  - [x] `SettingsModal` : `React.lazy(() => import(...))` dans `App.tsx`
  - [x] `DiscoveryPanel` : `React.lazy(() => import(...))` dans `Sidebar.tsx`
  - [x] `FileNotFoundModal` : `React.lazy(() => import(...))` dans `App.tsx`
  - [x] Chaque lazy import wrappé dans `<Suspense fallback={null}>`

- [ ] **7.2** Verifier le chunking
  - [ ] Lancer `npm run build` et verifier que la sortie montre des chunks separes
  - [ ] Le chunk principal doit etre significativement plus petit qu'avant

---

## Fichiers a modifier

| Fichier | Modifications |
|---------|--------------|
| `index.html` | Ajouter skeleton HTML/CSS statique dans `<div id="root">` |
| `src-tauri/tauri.conf.json` | Ajouter `"visible": false` dans la config fenetre |
| `src-tauri/capabilities/default.json` | Ajouter `"core:window:allow-show"` |
| `src/main.tsx` | Appeler `getCurrentWindow().show()` apres render, ajouter classe `.loaded` |
| `src/index.css` | Ajouter `@keyframes fadeIn`, styles pour `.loaded` |
| `src-tauri/src/main.rs` | Defer audio engine init, supprimer youtube_cache.load_index(), paralleliser |
| `src-tauri/src/state.rs` | Changer type `audio_engine` vers `Arc<OnceCell<AudioEngineHandle>>` |
| `src-tauri/src/audio/engine.rs` | Adapter pour init async |
| `src-tauri/src/audio/analysis.rs` | Ajouter lazy loading a `WaveformCache` |
| `src-tauri/src/youtube/cache.rs` | Ajouter lazy loading a `YouTubeCache` |
| `src-tauri/src/commands.rs` | Ajouter `get_initial_state`, adapter commandes audio pour `OnceCell` |
| `src-tauri/src/types.rs` | Ajouter struct `InitialState` |
| `src/utils/tauriCommands.ts` | Ajouter `getInitialState()` |
| `src/types/index.ts` | Ajouter type `InitialState` |
| `src/App.tsx` | Remplacer cascade effects par un seul appel `getInitialState()` |
| `src/stores/settingsStore.ts` | Ajouter `isInitialized` flag |
| `src/stores/profileStore.ts` | Ajouter `isLoading` flag |
| `src/components/Layout/MainContent.tsx` | Ajouter skeletons loading |
| `src/components/Layout/Sidebar.tsx` | Ajouter skeleton pour ProfileSelector |
| `vite.config.ts` | Aucun changement (Vite gere le code splitting automatiquement avec `React.lazy`) |

## Ordre d'execution recommande

Les phases sont numerotees par priorite (impact decroissant). A l'interieur d'une phase, les sous-taches sont sequentielles.

```
Phase 1 (Perception) ──> Phase 3 (get_initial_state) ──> Phase 6 (Skeletons React)
Phase 2 (Defer audio) ──> Phase 5 (Paralleliser Rust)
Phase 4 (Lazy caches) ─┘
Phase 7 (Code splitting) [independant]
```

## Notes

- Le gain total estime est de ~300-650ms sur le temps de demarrage reel, et un temps percu quasi-zero grace au skeleton.
- La Phase 2 (defer audio) est la plus delicate car elle change le type de `audio_engine` dans `AppState`, ce qui impacte toutes les commandes audio.
- Le `OnceCell` pattern est preferable au `Option<>` car il garantit une seule initialisation et un acces sans lock apres init.
- Tester sur Windows avec plusieurs peripheriques audio USB pour valider le gain de la Phase 2.
