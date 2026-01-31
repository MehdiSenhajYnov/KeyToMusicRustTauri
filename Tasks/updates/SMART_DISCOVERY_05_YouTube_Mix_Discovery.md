# Section 5 — Découverte musicale (YouTube Mix croisés)

> **Statut:** ⏳ PLANIFIE
> **Dépendance:** Sections 1 + 2 + 3 + 4
> **Parent:** [SMART_DISCOVERY.md](./SMART_DISCOVERY.md)

> **Objectif:** L'app analyse les sons YouTube existants de l'utilisateur, génère des YouTube Mix pour chacun, croise les recommandations pour identifier les morceaux les plus pertinents, et les propose dans un panneau sidebar avec one-click add. Zéro API externe, zéro parsing de noms — uniquement yt-dlp et l'algorithme de recommandation de YouTube.

## Pourquoi YouTube Mix au lieu de Last.fm

- **Last.fm a une mauvaise couverture des OST anime/manga** — exactement le public cible de l'app
- **Le parsing de noms de fichiers est fragile** — "track_03.mp3", titres en japonais, etc.
- **YouTube Mix est universel** — fonctionne avec n'importe quel genre (orchestral, anime OST, epic music, ambient...)
- **Pas de dépendance externe** — pas d'API key, pas de `reqwest`, uniquement yt-dlp déjà dans le projet
- **Le scoring par occurrences croisées est plus fiable** — un morceau recommandé par 3 seeds différents est quasi certain d'être pertinent

## Architecture globale

```
Sons YouTube de l'utilisateur (seeds)
    │
    │  Pour chaque seed qui a un video_id :
    ▼
YouTube Mix: https://youtube.com/watch?v={VIDEO_ID}&list=RD{VIDEO_ID}
    │
    │  yt-dlp --flat-playlist --dump-json
    ▼
~25-50 recommandations par seed
    │
    │  Comptage des occurrences croisées par video_id
    ▼
Scoring : video_id apparaît dans N Mix différents → score = N
    │
    │  Filtrer : exclure les sons déjà dans le profil
    │  Filtrer : exclure les durées < 30s ou > 15min (pas de la musique)
    │  Trier par score décroissant
    ▼
Top 30-40 suggestions
    │
    │  Auto-assign intelligent (frontend) :
    │  Analyse du profil → mode multi-sound ou single-sound
    │  Suggestion de touche + track pré-remplie (modifiable)
    │
    │  Téléchargement on-demand (au preview ou à l'ajout)
    │  Waveform + auto-momentum (sections 3-4)
    ▼
Card de suggestion dans la sidebar (one-click add)
```

## Exemple concret

```
Profil avec 10 sons YouTube :
  Seed A: "Two Steps From Hell - Heart of Courage" (id: XYZ1)
  Seed B: "Jo Blankenburg - Vendetta" (id: XYZ2)
  Seed C: "MHA - You Say Run" (id: XYZ3)
  ... (7 autres)

Étape 1 : Générer les Mix
  Mix de A → [D, E, F, G, H, I, J, K, ...]          (25 vidéos)
  Mix de B → [D, F, L, M, N, O, P, Q, ...]          (25 vidéos)
  Mix de C → [D, F, H, R, S, T, U, V, ...]          (25 vidéos)
  ... (7 autres Mix)

Étape 2 : Compter les occurrences
  D → apparaît dans Mix A + Mix B + Mix C = score 3  ← très pertinent
  F → apparaît dans Mix A + Mix B + Mix C = score 3  ← très pertinent
  H → apparaît dans Mix A + Mix C = score 2
  E, G, I, J, K → score 1 (un seul Mix)
  ...

Étape 3 : Filtrer et trier
  Exclure A, B, C (déjà dans le profil)
  Exclure les vidéos < 30s ou > 15min
  Trier : D (score 3), F (score 3), H (score 2), E (score 1), ...

Étape 4 : Présenter les top 30
  → L'utilisateur voit les suggestions triées par pertinence
  → Les suggestions score 2+ sont quasi certaines d'être pertinentes
  → Les suggestions score 1 sont plus exploratoires
```

## Backend (Rust)

- [ ] **5.1** Créer le module `src-tauri/src/discovery/mod.rs`
  - Module principal qui orchestre le pipeline de découverte

- [ ] **5.2** Créer `src-tauri/src/discovery/mix_fetcher.rs`
  - Fonction `fetch_youtube_mix(video_id: &str, yt_dlp_path: &str) -> Result<Vec<YoutubeSearchResult>, String>`
    - Construit l'URL Mix : `https://www.youtube.com/watch?v={video_id}&list=RD{video_id}`
    - Lance yt-dlp avec `--flat-playlist --dump-json`
    - Parse chaque ligne JSON
    - Retourne la liste des vidéos recommandées (réutilise `YoutubeSearchResult` de la section 1)
  - Gestion des erreurs : certains Mix peuvent être vides (vidéo privée, supprimée, etc.) → retourner Vec vide, pas d'erreur

- [ ] **5.3** Créer `src-tauri/src/discovery/engine.rs` — le moteur de découverte
  - Struct `DiscoveryEngine` :
    ```rust
    pub struct DiscoveryEngine {
        yt_dlp_path: String,
    }

    impl DiscoveryEngine {
        pub async fn generate_suggestions(
            &self,
            seeds: Vec<SeedInfo>,
            existing_video_ids: HashSet<String>,
            cancel_flag: &AtomicBool,
            progress_callback: impl Fn(DiscoveryProgress),
        ) -> Result<Vec<DiscoverySuggestion>, String>
    }

    pub struct SeedInfo {
        pub video_id: String,
        pub sound_name: String,  // Pour l'affichage de progression
    }
    ```
  - **Pipeline :**
    1. Pour chaque seed (séquentiellement, yt-dlp est lent) :
       - Émettre la progression
       - Vérifier `cancel_flag` (annulation si changement de profil)
       - Appeler `mix_fetcher::fetch_youtube_mix(video_id)`
       - Stocker les résultats dans une `HashMap<String, MixOccurrence>`
    2. Compter les occurrences croisées :
       ```rust
       struct MixOccurrence {
           video: YoutubeSearchResult,
           source_seeds: Vec<String>,  // video_ids des seeds qui ont recommandé cette vidéo
           occurrence_count: usize,
       }
       ```
    3. Filtrer :
       - Exclure les `video_id` déjà dans `existing_video_ids`
       - Exclure les durées < 30 secondes (SFX, intros) ou > 900 secondes / 15 min (compilations, mix)
    4. Trier par `occurrence_count` décroissant, puis par durée (préférer les morceaux de 2-6 min)
    5. Garder les top N résultats (N = 30 par défaut)

- [ ] **5.4** Créer `src-tauri/src/discovery/cache.rs`
  - Fichier : `data/discovery/{profile_id}.json`
  - Structure :
    ```rust
    pub struct DiscoveryCache {
        pub profile_id: String,
        pub seed_hash: String,           // Hash des video_ids des seeds (pour détecter les changements)
        pub generated_at: String,        // ISO 8601
        pub suggestions: Vec<CachedDiscoverySuggestion>,
    }

    pub struct CachedDiscoverySuggestion {
        pub video_id: String,
        pub title: String,
        pub channel: String,
        pub duration: f64,
        pub url: String,
        pub occurrence_count: usize,     // Dans combien de Mix cette vidéo est apparue
        pub source_seed_ids: Vec<String>,  // video_ids des seeds sources
        pub source_seed_names: Vec<String>, // Noms des seeds pour l'affichage
        pub dismissed: bool,
        pub added: bool,                 // true si déjà ajouté au profil
    }
    ```
  - Fonctions : `load(profile_id)`, `save(cache)`, `delete(profile_id)`
  - **Seed hash** : calculé à partir des `video_id` des sons YouTube du profil. Si les sons changent → hash change → re-génération

- [ ] **5.5** Extraire les seeds d'un profil
  - Fonction `extract_seeds(profile: &Profile) -> Vec<SeedInfo>`
    - Parcourir tous les sons du profil
    - Pour les sons YouTube (`source == YouTube` et `cached_path` contient un video_id) :
      - Extraire le video_id depuis le `cached_path` (format: `{video_id}.m4a`) ou depuis l'URL YouTube stockée
      - Ajouter comme seed
    - Les sons locaux sont ignorés (pas de video_id = pas de Mix possible)
    - Retourner la liste des seeds

- [ ] **5.6** Créer les commandes Tauri
  - `start_discovery(profile_id: String)` → lance en background, retourne immédiatement
  - `get_discovery_suggestions(profile_id: String) -> Result<Vec<DiscoverySuggestionForFrontend>, String>` → retourne le cache
  - `dismiss_discovery(profile_id: String, video_id: String)` → marque comme ignoré
  - `add_discovery_sound(profile_id: String, video_id: String, url: String, track_id: String, key_code: String) -> Result<Sound, String>` → télécharge, crée le binding sur la touche/track indiquée, retourne le son
  - Type frontend-facing :
    ```rust
    pub struct DiscoverySuggestionForFrontend {
        pub video_id: String,
        pub title: String,
        pub channel: String,
        pub duration: f64,
        pub url: String,
        pub occurrence_count: usize,        // Score de pertinence (nombre de Mix croisés)
        pub source_seed_ids: Vec<String>,   // video_ids des seeds sources (pour l'auto-assign frontend)
        pub source_seed_names: Vec<String>, // "Recommandé depuis: Heart of Courage, Vendetta, You Say Run"
    }
    ```

- [ ] **5.7** Enregistrer les commandes dans `main.rs`

## Backend — Gestion du background thread

- [ ] **5.8** Lancement au démarrage de l'app
  - Dans `main.rs`, après le chargement du profil initial :
    1. Extraire les seeds du profil
    2. Si aucun seed YouTube → ne rien lancer (pas de découverte sans seeds)
    3. Si cache existe ET `seed_hash` correspond → charger le cache, émettre `discovery_ready`
    4. Si pas de cache OU hash différent → lancer `start_discovery` en background thread
  - Le background thread :
    1. Émet `discovery_started`
    2. Pour chaque seed séquentiellement (yt-dlp est lent, ~3-5s par Mix) :
       - Émet `discovery_progress` avec le seed en cours
       - Fetch le Mix
       - Vérifier `cancel_flag` entre chaque seed
    3. Phase de scoring et filtrage
    4. Émet `discovery_complete` avec les suggestions triées
    5. Sauvegarde le cache

- [ ] **5.9** Events Tauri pour le frontend
  ```rust
  // Début de la génération
  "discovery_started" → {}

  // Progression
  "discovery_progress" → { current: usize, total: usize, current_seed_name: String }

  // Génération terminée (toutes les suggestions d'un coup, pas incrémental)
  "discovery_complete" → { suggestions: Vec<DiscoverySuggestionForFrontend> }

  // Erreur
  "discovery_error" → { message: String }
  ```
  - **Pourquoi pas incrémental cette fois :** Les suggestions n'ont de sens qu'après le croisement de TOUS les Mix. Un morceau qui apparaît dans 1 Mix n'est pas encore scoré. On attend la fin du pipeline.

- [ ] **5.10** Gestion du changement de profil
  - Quand l'utilisateur change de profil :
    1. Annuler la génération en cours (`AtomicBool` comme pour l'export)
    2. Charger le cache du nouveau profil (si existe)
    3. Si pas de cache → relancer la découverte

## Optimisations de performance

- [ ] **5.11** Limiter le nombre de seeds
  - Si le profil a plus de 15 sons YouTube → prendre les 15 plus récents (ou les 15 plus écoutés si on tracke ça)
  - 15 seeds × ~4s par Mix = ~60 secondes de génération → acceptable en background
  - 30+ seeds → trop long, et les Mix supplémentaires ajoutent peu de valeur (rendements décroissants)

- [ ] **5.12** Parallélisation prudente
  - Lancer 2 fetches de Mix en parallèle max (pas plus, yt-dlp consomme du CPU)
  - Utilise `tokio::spawn` ou un thread pool restreint
  - Réduit le temps de ~60s à ~30s pour 15 seeds

## Auto-assign intelligent (frontend)

L'auto-assign calcule une suggestion de touche + track pour chaque suggestion de découverte. Toutes les valeurs sont pré-remplies mais modifiables directement sur la card. Le calcul est entièrement côté frontend (accès au profil, aux bindings, et au `layoutMap` de `keyMapping.ts`).

### Détection du mode utilisateur

- [ ] **5.12b** Créer `src/utils/profileAnalysis.ts`
  - Fonction `analyzeProfile(profile: Profile) -> ProfileMode`
    ```typescript
    type ProfileMode = 'multi-sound' | 'single-sound';

    function analyzeProfile(profile: Profile): ProfileMode {
      const bindings = profile.keyBindings;
      if (bindings.length === 0) return 'single-sound';
      const avgSounds = bindings.reduce((sum, b) => sum + b.soundIds.length, 0) / bindings.length;
      return avgSounds > 1.5 ? 'multi-sound' : 'single-sound';
    }
    ```

### Mode multi-sound (avg > 1.5 sons/touche)

L'utilisateur regroupe plusieurs sons par touche (ex: touche A = sons épiques, touche Z = sons tristes). La suggestion doit aller sur la touche la plus similaire.

- **Signal de similarité** : les `source_seed_ids` de la suggestion. Si la touche A contient un son dont le `video_id` est dans les `source_seed_ids`, c'est un match.
- **Algorithme** :
  ```
  Pour chaque binding du profil :
    match_count = nombre de sons du binding dont le video_id est dans suggestion.source_seed_ids
  Touche suggérée = le binding avec le plus grand match_count (> 0)
  Track suggéré = le track de ce binding
  En cas d'égalité : prendre le binding avec le plus de sons (l'utilisateur y accumule)
  Si aucun match (0 pour tous) : fall back vers le track le plus utilisé + le binding avec le plus de sons
  ```

### Mode single-sound (avg <= 1.5 sons/touche)

L'utilisateur assigne un son par touche. La suggestion doit proposer la prochaine touche libre.

- **Touche suggérée** : parcourir les touches dans l'ordre du layout clavier (`layoutMap` de `keyMapping.ts`), prendre la première qui n'a pas de binding
- **Track suggéré** : le track le plus utilisé dans le profil (celui avec le plus de bindings)
- Si toutes les touches sont prises : ne pas suggérer de touche (l'utilisateur devra choisir)

### Fonction principale

- [ ] **5.12c** Créer `computeAutoAssign` dans `profileAnalysis.ts`
  ```typescript
  interface AutoAssign {
    suggestedKey: string | null;    // ex: "KeyA", null si aucune suggestion possible
    suggestedKeyDisplay: string;    // ex: "A" (pour l'affichage)
    suggestedTrackId: string;       // ex: "track-1"
    suggestedTrackName: string;     // ex: "OST"
  }

  function computeAutoAssign(
    suggestion: DiscoverySuggestionForFrontend,
    profile: Profile,
    layoutMap: Map<string, string>,
  ): AutoAssign
  ```
  - Appelée par le `DiscoveryPanel` pour chaque suggestion lors du rendu
  - Recalculée si le profil change (bindings modifiés) → les suggestions s'adaptent en temps réel

## Frontend — Section Découverte dans la Sidebar

- [ ] **5.13** Créer le store `discoveryStore.ts`
  ```typescript
  interface DiscoveryState {
    suggestions: DiscoverySuggestionForFrontend[];
    isGenerating: boolean;
    progress: { current: number; total: number; currentSeedName: string } | null;
    error: string | null;

    // Actions
    setSuggestions: (suggestions: DiscoverySuggestionForFrontend[]) => void;
    removeSuggestion: (videoId: string) => void;
    setGenerating: (generating: boolean) => void;
    setProgress: (progress: ...) => void;
    clear: () => void;
  }
  ```

- [ ] **5.14** Créer le hook `useDiscovery.ts`
  - Écoute les events Tauri : `discovery_started`, `discovery_progress`, `discovery_complete`, `discovery_error`
  - Met à jour le `discoveryStore`

- [ ] **5.15** Créer le composant `DiscoveryPanel.tsx` dans `src/components/Discovery/`
  - **Position** : en bas de la Sidebar, sous "Now Playing"
  - **Header** : "Découverte" avec un petit compteur (nombre de suggestions)
  - **État loading** :
    - Spinner + "Exploration en cours..."
    - Texte secondaire : "Analyse de {current}/{total} sons..." avec le nom du seed en cours
  - **État vide** :
    - Si aucun son YouTube dans le profil : "Ajoutez des sons depuis YouTube pour activer la découverte"
    - Si génération terminée sans résultats : "Aucune suggestion trouvée"
  - **Carrousel vertical scrollable** :
    - Hauteur fixe (~200px), scroll interne
    - Cards triées par `occurrence_count` décroissant (les plus pertinents en haut)
  - **Bouton refresh** : icône en haut à droite pour forcer un re-scan
    - Efface le cache et relance la découverte

- [ ] **5.16** Design de la card de suggestion
  ```
  ┌──────────────────────────────────────┐
  │ Thomas Bergersen - Empire of Angels  │  ← titre (bold, truncate)
  │ ThePrimeCronus · 5:12               │  ← channel · durée
  │ ★★★ Recommandé 3 fois              │  ← score visuel
  │ Via: Heart of Courage, Vendetta...  │  ← seeds sources (texte muted, petit)
  │  [A]  [OST ▾]                        │  ← touche (cliquable) + track (dropdown)
  │                                      │
  │ ┌────┐ ┌─────────┐ ┌──┐            │
  │ │ ▶  │ │ Ajouter │ │ ✕│            │
  │ └────┘ └─────────┘ └──┘            │
  └──────────────────────────────────────┘
  ```
  - **Titre** : titre YouTube (bold, truncate avec tooltip)
  - **Sous-titre** : channel YouTube · durée formatée (MM:SS)
  - **Score** : `occurrence_count` affiché visuellement
    - 3+ : "★★★ Recommandé {N} fois" (texte accent, haute confiance)
    - 2 : "★★ Recommandé 2 fois" (texte normal)
    - 1 : "★ Suggestion" (texte muted, exploratoire)
  - **Sources** : "Via: {seed_name_1}, {seed_name_2}, ..." (texte petit, muted, truncate)
  - **Touche suggérée** : badge affichant la touche (ex: `[A]`), calculée par `computeAutoAssign`
    - **Au clic** : passe en mode capture de touche (même pattern que `KeyCaptureSlot` dans AddSoundModal)
    - Si `suggestedKey` est `null` : afficher "Choisir une touche" en placeholder
  - **Track suggéré** : dropdown inline affichant le nom du track
    - **Au clic** : ouvre la liste déroulante des tracks du profil pour changer
    - Valeur par défaut : calculée par `computeAutoAssign`
  - **Bouton Preview** : télécharge en temp (`data/temp/`), joue 15 secondes
  - **Bouton Ajouter** (one-click) :
    1. Télécharge le son via `addSoundFromYoutube`
    2. Crée le binding sur la touche/track affichés (via `add_discovery_sound` avec `key_code` et `track_id`)
    3. Le momentum est auto-détecté (waveform + auto-momentum des sections 3-4)
    4. Retire la suggestion du carrousel
    5. Toast : "Son ajouté sur {touche} ({track}) : {title}"
    6. L'utilisateur peut ajuster après via SoundDetails s'il le souhaite
    - **Si aucune touche sélectionnée** : le bouton Ajouter est disabled, tooltip "Choisissez une touche"
  - **Bouton Dismiss** : marque comme `dismissed: true` dans le cache

- [ ] **5.17** Track Preview dédié
  - Créer un track spécial côté audio engine, pas visible dans TrackView
  - ID réservé : `"__preview__"`
  - Volume : suit le master volume, volume track fixe à 1.0
  - Comportement :
    - Un seul preview à la fois (lancer un nouveau preview stop l'ancien)
    - Stop automatique après 15 secondes (ou fin du morceau si plus court)
    - Stop si l'utilisateur clique "Ajouter"
  - Le frontend n'affiche pas ce track dans NowPlaying (filtrer par ID !== "__preview__")
  - Commandes : réutiliser `play_sound` et `stop_sound` avec track_id = "__preview__"

- [ ] **5.18** Gestion des fichiers temporaires de preview
  - Dossier : `data/temp/`
  - Nommage : `preview_{video_id}.m4a`
  - Nettoyage :
    - Au démarrage de l'app : supprimer tout le contenu de `data/temp/`
    - Quand une suggestion est dismiss : supprimer le fichier temp associé s'il existe
    - Quand une suggestion est ajoutée : déplacer le fichier de `data/temp/` vers `data/cache/`

## Intégration dans App.tsx

- [ ] **5.19** Initialiser le hook `useDiscovery` dans `App.tsx`
  - Écouter les events de découverte
  - Lancer la découverte au chargement du profil initial
  - Relancer quand le profil change

- [ ] **5.20** Ajouter `DiscoveryPanel` dans `Sidebar.tsx`
  - En dessous de la section NowPlaying
  - Visible uniquement si le profil a au moins 1 son YouTube

## Limites et cas particuliers

- **Profil sans sons YouTube** : pas de seeds → pas de découverte → message "Ajoutez des sons YouTube pour activer"
- **Sons locaux uniquement** : ignorés (pas de video_id). Évolution future possible : rechercher le son sur YouTube par nom pour trouver un video_id, puis générer le Mix
- **Mix vide** (vidéo supprimée/privée) : le seed est skipé, pas d'erreur, la progression continue
- **Doublons dans le cache YouTube** : vérifier via `existing_video_ids` extraits du cache et du profil
- **Renouvellement naturel** : quand l'utilisateur ajoute un son depuis les suggestions → le seed hash change → prochaine génération inclura ce nouveau son comme seed → suggestions différentes

## Fichiers impactés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/discovery/mod.rs` | **Nouveau** — Module principal découverte |
| `src-tauri/src/discovery/mix_fetcher.rs` | **Nouveau** — Fetch des YouTube Mix via yt-dlp |
| `src-tauri/src/discovery/engine.rs` | **Nouveau** — Moteur de découverte (croisement, scoring) |
| `src-tauri/src/discovery/cache.rs` | **Nouveau** — Cache des suggestions par profil |
| `src/utils/profileAnalysis.ts` | **Nouveau** — Analyse du profil (mode multi/single) + auto-assign |
| `src-tauri/src/types.rs` | Ajouter types `SeedInfo`, `DiscoverySuggestion`, `DiscoveryCache`, etc. |
| `src-tauri/src/commands.rs` | Ajouter commandes `start_discovery`, `get_discovery_suggestions`, `dismiss_discovery`, `add_discovery_sound` |
| `src-tauri/src/main.rs` | Enregistrer commandes, lancer découverte au démarrage, nettoyage temp |
| `src/components/Discovery/DiscoveryPanel.tsx` | **Nouveau** — UI carrousel découverte sidebar |
| `src/stores/discoveryStore.ts` | **Nouveau** — Store Zustand pour la découverte |
| `src/hooks/useDiscovery.ts` | **Nouveau** — Hook pour écouter les events de découverte |
| `src/components/Layout/Sidebar.tsx` | Ajouter `DiscoveryPanel` |
| `src/components/Controls/NowPlaying.tsx` | Filtrer le track `"__preview__"` |
| `src/types/index.ts` | Ajouter types frontend |
| `src/utils/tauriCommands.ts` | Ajouter wrappers pour les commandes discovery |
| `src/App.tsx` | Initialiser `useDiscovery` |
