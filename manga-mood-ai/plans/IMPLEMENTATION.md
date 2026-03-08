# Manga Mood AI — Spec d'implementation pour KeyToMusic

> Statut documentaire:
>
> - ce fichier melange etat technique courant, benchmark et historique de recherche
> - pour le flux produit actif extension/backend/runtime, la source de verite est [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md)
> - pour la synthese benchmark, utiliser [manga-mood-ai/research/RESEARCH_SYNTHESIS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/RESEARCH_SYNTHESIS.md)

## Modele choisi

**Production app/extension (2026-03-08) : Qwen3-VL-4B-Thinking** (GGUF Q4_K_M, ~3.3 GB avec mmproj) — meme backbone que le winner benchmark, via un **noyau partage** `winner.rs` et un scheduler prod temps reel. **Plus de fallback single-page simpliste**.
**Ancien baseline RealTest : Qwen3-VL-4B-Thinking historique** — protocole V12 3 pages, **47/70 strict et 60/70 relaxed (85.7%)** sur `BL/1`.
**Winner benchmark local actuel : `t4b_wide5_selective_hold`** — `Qwen3-VL-4B-Thinking` + backbone `wide-5` + reprompts selectifs aux bords de runs + **delayed action bridge hold** zero-cost, **58/70 strict, 68/70 relaxed, 24/70 intensity** sur `BL/1`, **~11.9s/page**, `12` pages reanalysees.
**Winner precedent / mini-amelioration importante : `t4b_wide5_selective`** — meme backbone sans hold structurel, **56/70 strict, 68/70 relaxed, 24/70 intensity** sur `BL/1`, **~11.8s/page** sur rerun comparable du 2026-03-07 (ancien artefact observe autour de `13.0s/page`).
**Meilleur variant recherche non deployable : ensemble derive** — **47/70 strict, 62/70 relaxed, 41/70 intensity** sur `BL/1` en fusionnant `t4b_hist_live + t4b_focus_direct + q3vl_2b + q35_9b` avec Viterbi. **Utile pour la recherche, mais trop lent pour la prod**.
**Meilleur challenger live observe : Qwen3.5-9B** — **36/70 strict, 57/70 relaxed, 26/70 intensity** sur `BL/1`, ~2.8s/window, ~9.5 GB VRAM.
**Meilleur modele auxiliaire observe : Qwen3-VL-2B-Instruct** — **41/70 intensity** sur `BL/1`, ~1.2s/window, utile comme tete d'intensite ou expert `tension`.
**Reference recherche 31 pages Blue Lock : Qwen3.5-VL 4B** (GGUF Q4_K_M, ~2.5 GB) — meilleur resultat sur le benchmark sequentiel 31 pages, avec V12 multi-image a **23/31 strict, 28/31 relaxed**.

| Modele | Images isolees | Sequence (31 pages) | RealTest `BL/1` | Vitesse | VRAM |
|--------|-----------------|---------------------|-----------------|---------|------|
| Qwen3-VL 2B | 18/18 (100%) | Non teste | Non teste comme primaire en RealTest | ~1.1s | 38% |
| Qwen3.5-VL 4B | Non teste | 23/31 strict (74%), 28/31 relaxed (90%) avec V12 | **23/70 strict, 32/70 relaxed** dans la suite comparative Mars 2026 | ~1.9s/window | ~58% |
| Qwen3.5-9B | Non teste | Non teste | **36/70 strict, 57/70 relaxed, 26/70 intensity** | ~2.8s/window | ~79% |
| Qwen3-VL-2B-Instruct | Non teste | Non teste | **32/70 strict, 55/70 relaxed, 41/70 intensity** | ~1.2s/window | ~55% |
| Qwen3-VL-4B-Thinking | Non teste | Variante dediee thinking, pas winner sur le 31 pages | **58/70 strict, 68/70 relaxed** avec `wide5_selective_hold` ; winner precedent: `56/70` avec `wide5_selective` ; ancien historique: `47/70` | ~11.9s/page (`wide5_selective_hold`) ; ~11.8s/page (`wide5_selective`) | ~69% |
| Ensemble derive (4 signaux) | Non teste | Non teste | **47/70 strict, 62/70 relaxed, 41/70 intensity** | ~19-23s/page effectifs selon les dependances | VRAM pic = max du plus gros modele actif |

- Thinking model : utilise des `<think>` tags internes pour raisonner avant de repondre
- **8 moods dimensionnels** : epic, tension, sadness, comedy, romance, horror, peaceful, mystery — chacun avec intensite 1-3
- Ancien systeme (10 moods avec emotional_climax, chase_action) abandonne — trop confusable
- **Pipeline V12 (reference benchmark historique) :** 3 pages consecutives + vote majoritaire
- **Pipeline V6 :** historique important, mais depasse comme reference benchmark
- **Suite comparative RealTest (mars 2026) :** baseline cache + variantes live comparees automatiquement par `realtest_benchmark`
- **Architecture actuelle propre :**
  - `winner.rs` = noyau partage du winner (prompts, parsing, aggregation, repairs, action hold)
  - `director_realtest_suite.rs` = harness benchmark / labo d'experiences
  - `chapter_pipeline.rs` = orchestration prod temps reel
  - `server.rs` = API HTTP locale
  - `background.js` / `content.js` = extension, detection page visible + orchestration UI/debug
- **Plan runtime hors CUDA :** `manga-mood-ai/plans/PLAN_RUNTIME_SIZING_NGL_SUPPORT.md`
- **Runtime hors CUDA implemente (2026-03-07) :**
  - demarrage `llama-server` avec `context / np / ngl` dynamiques et fallback decroissant
  - bench par defaut ramene de `32768x4` a `primary_12288x1`
  - `research_32768x4` conserve seulement pour les runs historiques
  - CPU retire du chemin auto supporte sous Linux, garde uniquement via override explicite
- **Attention :** sur `Qwen3-VL-4B-Thinking`, toutes les variantes activees avec grammar / PNG / Viterbi / OCR / semantic ont collapse en sortie quasi constante `sadness`; ces runs ne prouvent donc pas encore que ces axes sont mauvais, ils signalent d'abord un probleme de protocole/harness.
- **Observation cle :** les meilleurs gains pratiques viennent pour l'instant d'une **fusion conservative de plusieurs specialistes**, pas d'un unique prompt miracle.
- **Nouveau gate de validation :** une variante n'est consideree comme gagnante que si elle reste dans un budget de cout proche du baseline historique et apporte **au moins +2 strict** ou **+4 relaxed** sur `BL/1`.
- **Eliminations smoke deja actees :** les variantes `thinking4b` en prompt direct courant-page (`focus_short`, `focus_grammar_short`, `wide5_narrative`, `wide5_narrative_grammar`) sont trop instables ou trop mauvaises des les 3 premieres pages; elles ne doivent plus etre priorisees tant que leur parsing/protocole n'est pas repense.
- **Mini-amelioration documentee mais insuffisante :** `t4b_wide5_sequence` et `t4b_wide5_sequence_viterbi` montent a **61/70 relaxed** avec un cout encore proche du baseline (~9.3-9.4s/window), mais restent a **43/70 strict**. Cette famille a servi de backbone au winner final.
- **Winner precedent documente :** `t4b_wide5_selective` reprend ce backbone `wide-5`, puis ne reprompte que `12` pages aux bords de runs `comedy / tension / epic`. Resultat: **56/70 strict**, **68/70 relaxed**, **24/70 intensity**, **~13.0s/page**. Le gain vient de corrections locales structurelles:
  - ouverture de petits runs `epic` qui sont encore en fait du `tension`
  - faux runs `comedy` dont seuls les bords sont reellement `mystery` ou `tension`
  - longs runs `tension` qui cachent un debut `mystery` et une fin `epic`
  - fins de runs `epic` qui rebasculent en `tension`
- **Winner final actuel :** `t4b_wide5_selective_hold` conserve exactement ces `12` reprompts, puis ajoute un **hold sequentiel zero-cost** sur les runs `epic -> tension court -> epic`. Regle:
  - si un run `tension` de longueur `4-6` est encadre par un run `epic` court avant lui (`<=3`) et un run `epic` confirme apres lui (`>=4`)
  - alors on retarde la coupure d'OST et on garde `epic` sur les **2 premieres pages** de ce run `tension`
  - sur `BL/1`, cela ne change que **2 pages**: `13` et `14` passent de `tension` a `epic`
  - resultat final: **58/70 strict**, **68/70 relaxed**, **24/70 intensity**, **~11.9s/page**

---

## Runtime : llama.cpp server (pas Ollama)

### Pourquoi pas Ollama
- Ollama = install externe que l'utilisateur doit faire lui-meme
- Overhead Go/HTTP (+10-30% latence)
- Moins de controle sur les parametres (KV cache, flash attention, etc.)

### Architecture
- **llama-server** : binaire standalone (~50 MB), auto-telecharge dans `data/bin/` au premier lancement (meme pattern que yt-dlp et ffmpeg)
- **Modele GGUF winner actuel** : `Qwen3-VL-4B-Thinking` + `mmproj`, auto-telecharges dans `data/models/`
- Communication via **HTTP localhost** (API compatible OpenAI)
- Lance en subprocess par le backend Rust, tue a la fermeture de l'app
- Plan futur backends GPU / CUDA / cross-platform : voir `manga-mood-ai/plans/PLAN_GPU_BACKENDS_RUNTIME.md`

### Lifecycle du serveur
1. User active la feature "Manga Mood" dans les settings
2. KeyToMusic telecharge llama-server + modele GGUF si pas deja present
3. Lance `llama-server` en subprocess sur un port local random
4. Le serveur reste actif tant que la feature est activee
5. A la fermeture de l'app ou desactivation de la feature : kill le process

---

## Optimisations et enseignements historiques

> Cette section conserve des notes utiles issues des anciennes iterations 2B / 10-moods / V6.
> Elle ne decrit pas a elle seule le runtime produit actuel.

### 1. Resize image a 672px

Avant d'envoyer l'image au modele, la redimensionner pour que le cote le plus long = 672px.

```
Impact : +15% vitesse, +8% accuracy (13/13 vs 12/13)
Pourquoi : moins de tokens visuels = le modele se concentre sur le mood
```

Implementation :
- Cote Rust, avant l'appel HTTP : resize avec `image` crate
- Garder le ratio, LANCZOS pour la qualite
- Convertir en JPEG quality 90 (plus petit que PNG)
- Pas besoin de sauver sur disque : envoyer en base64 directement

### 2. Fenetre de contexte

- **Ancienne prod 2B (historique) :** `-c 2048`
- **Pipeline V6 (historique) :** `-c 32768`
- **Runtime produit actuel :** sizing pilote par `runtime_intent`, avec profils dynamiques et winner core partage

Implementation :
- Les valeurs fixes ci-dessus sont historiques.
- Le runtime actuel est documente dans `llama_manager.rs`, `inference.rs` et `PLAN_RUNTIME_SIZING_NGL_SUPPORT.md`.

### 3. Prompt court historique

```
What is the mood of this manga page? Pick ONE: epic_battle, tension, sadness,
comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action.
Reply with just the mood word.
```

Ce prompt appartient a l'ancienne phase 10 moods. Le produit actuel n'utilise plus cette taxonomie.

- Pas de `/no_think` : laisser le modele penser
- Pas de JSON demande : juste le mot
- `temperature: 0.1` pour la reproductibilite
- `num_predict: 5000` (le thinking peut utiliser beaucoup de tokens, mais num_ctx 2048 le contraint naturellement)

### 4. Flash Attention

```
Impact attendu : -10-30% VRAM, un poil plus rapide
```

Implementation :
- Flag llama-server : `--flash-attn`
- A tester : verifier que le modele GGUF le supporte

### 5. KV Cache q8_0

Quantizer le cache d'attention de FP16 a 8-bit.

```
Impact attendu : ~50% de VRAM en moins sur le cache (~200-400 MB)
```

Implementation :
- Flag llama-server : `--cache-type-k q8_0 --cache-type-v q8_0`

### 6. Process priority BELOW_NORMAL

Baisser la priorite du process llama-server pour ne pas freeze le PC.

```
Impact : zero impact sur la vitesse en usage normal, evite les freezes
quand le PC fait autre chose
```

Implementation :
- Cote Rust : lancer le subprocess avec priorite BELOW_NORMAL
  - Windows : `CREATE_BELOW_NORMAL_PRIORITY_CLASS` dans `CreateProcessW`
  - macOS/Linux : `nice +10` ou `setpriority()`
- Optionnel dans les settings : "Performance mode" qui remet en NORMAL

### 7. Adaptive num_gpu (essentiel pour PC faibles)

Detecter la VRAM disponible et ajuster le nombre de layers GPU.

```
Impact : permet de faire tourner le modele sur des PC avec moins de VRAM
en offloadant une partie sur CPU (plus lent mais marche)
```

Implementation :
- Detecter VRAM totale au lancement (via NVML ou `vulkaninfo`)
- Si VRAM < 6 GB : `-ngl 20` (partiel GPU)
- Si VRAM < 4 GB : `-ngl 0` (full CPU, ~5-10x plus lent)
- Si VRAM >= 6 GB : `-ngl 99` (full GPU, defaut)

---

## Flags llama-server complets

> Note : cette section melange de l'historique et des profils cibles. La reference runtime actuelle cote prod est le **winner runtime intent** aligne sur le backbone benchmark gagnant, pas l'ancien mode 2B simple.

### Historique — production simple 2B
```bash
llama-server \
  -m data/models/qwen3-vl-2b-q4_k_m.gguf \
  -c 2048 \
  --flash-attn \
  --cache-type-k q8_0 \
  --cache-type-v q8_0 \
  --port {PORT} \
  --host 127.0.0.1 \
  -ngl 99
```

### Historique — pipeline V6 (4B, descriptions contextuelles)
```bash
llama-server \
  -m data/models/qwen3.5-vl-4b-q4_k_m.gguf \
  -c 32768 \
  --flash-attn auto \
  --cache-type-k q8_0 \
  --cache-type-v q8_0 \
  --port {PORT} \
  --host 127.0.0.1 \
  -ngl 99 \
  --image-min-tokens 1024
```

---

## Parsing de la reponse

> Note: le parsing 10 moods ci-dessous est historique. Le chemin produit actuel passe par le winner core partage, avec 8 moods + intensite, fenetres de contexte et aggregation.

Le modele retourne du texte libre avec potentiellement des `<think>` tags :

```
<think>The image shows a character crying with dark tones...</think>
sadness
```

Parsing :
1. Supprimer les `<think>...</think>` tags (regex)
2. Chercher un des 10 moods dans le texte restant
3. Si aucun mood trouve : fallback "unknown"

Pas de structured output / JSON schema : incompatible avec le thinking de ce modele.

---

## Flux utilisateur

> Les deux flux ci-dessous sont historiques. Le flux produit actuel repose sur le noyau winner partage.

### Historique — mode simple 2B
```
Page manga detectee (screenshot/fichier)
    |
    v
Resize a 672px (Rust, image crate)
    |
    v
POST /v1/chat/completions (single-label GUIDED_V3 prompt)
    |
    v
Parse response -> mood (string)
    |
    v
Mapper mood -> playlist/son dans KeyToMusic
```

### Historique — pipeline V6 contextuel
```
Page manga N detectee
    |
    v
Resize a 672px (Rust, image crate)
    |
    v
Step 1: describe_page(image_N)
  → POST /v1/chat/completions (describe prompt, no classification)
  → description factuelle (~500-800 tokens)
    |
    v
Step 2: classify_with_context(image_N, descriptions[N-4..N-1])
  → POST /v1/chat/completions (image + 4 descriptions passees + classify prompt)
  → mood + intensite (ex: "tension 3")
    |
    v
Stocker description dans DescriptionBuffer
    |
    v
Mapper mood -> playlist/son dans KeyToMusic
```

### Flux produit actuel
```text
Page visible detectee par l'extension
    |
    v
lookup(chapter, page) dans le cache local
    |
    +--> hit
    |     |
    |     v
    |   trigger OST
    |
    +--> miss
          |
          v
analyze-window sur la page visible avec voisinage local
          |
          v
chapter_pipeline.rs appelle le noyau winner partage
  - wide-5 backbone
  - aggregation winner
  - selective repairs locaux si possibles
  - action bridge hold
          |
          v
cache + trigger OST si la page visible est toujours la bonne
```

---

## Telechargements auto (premier lancement)

| Fichier | Taille | Source | Destination |
|---|---|---|---|
| llama-server | ~50 MB | GitHub releases llama.cpp | `data/bin/llama-server` |
| Qwen3-VL-4B-Thinking GGUF | ~2.5 GB | HuggingFace | `data/models/` |
| mmproj Qwen3-VL-4B-Thinking | ~0.8 GB | HuggingFace | `data/models/` |

Meme pattern que `yt_dlp_manager.rs` et `ffmpeg_manager.rs` :
- Verifier si present
- Telecharger avec progress bar
- Verifier checksum
- Extraire si archive

---

## Limites connues

- **10.jpg (Blue Lock stade)** : souvent classee "tension" au lieu de "emotional_climax" — les deux sont valides
- **3.jpeg (Solo Leveling)** : alterne entre "tension" et "epic_battle" — les deux sont valides
- **Images ambigues** : les moods sont subjectifs, le modele choisit une interpretation raisonnable
- **Pas de "comedy" dans les tests** : le dataset n'inclut pas d'images comedy, non teste
- **VRAM minimum** : ~4.7 GB GPU ou CPU-only (beaucoup plus lent)
- **Cold start** : ~3-5s au premier lancement du serveur + chargement modele
- **emotional_climax over-prediction** : sur une sequence de 31 pages, le modele confond l'intensite emotionnelle avec le climax narratif. 6 FAIL sur 31 pages, tous sur sadness/tension interpretees comme emotional_climax.
- **Scored prompt inutilisable** : demander au modele 10 scores (0.0-1.0) donne 26% de precision vs 65% en single-label. Les petits modeles sont des classificateurs, pas des regresseurs.

---

## Architecture implementee (etat actuel)

### Winner Core (`winner.rs`)

Source de verite partagee entre benchmark et prod.

- `WINNER_MODEL_KEY`
- `winner_runtime_intent()`
- prompts winner :
  - `build_wide_sequence_prompt()`
  - `build_focus_prompt()`
  - `build_narrative_focus_prompt()`
- preprocessing / request body / parse :
  - `encode_image()`
  - `build_request_body()`
  - `parse_prediction()`
- logique de decision winner :
  - `build_vote_stats()`
  - `aggregate_prediction()`
  - `aggregate_majority()`
  - `plan_selective_repairs()`
  - `apply_action_bridge_holds()`

### Chapter Mood Pipeline (`chapter_pipeline.rs`)

Orchestrateur prod temps reel, base sur le noyau winner.

- bufferise les pages d'un chapitre
- sait lancer les actions winner (`execute_action()`)
- sait analyser localement une page visible avec son voisinage via `analyze_visible_window()`
- finalise les pages deja calculables
- applique les selective repairs et le hold structurel avec la logique partagee winner

### Benchmark Runner (`director_realtest_suite.rs`)

Harness offline / recherche.

- declare les experiences via `build_suite()`
- ajoute facilement :
  - nouveau modele
  - nouveau prompt
  - nouveau sampling
  - nouveau preprocessing
  - nouvelle decision strategy
- utilise maintenant le meme noyau winner pour :
  - prompts
  - parsing
  - aggregation majoritaire winner
  - selective repairs
  - action bridge hold

### MoodDirector (`director.rs`)

Smoothing algorithmique post-VLM. Evite le "zapping" d'OST.

- **Sliding window** (5 pages) avec scores moyennes
- **Dwell counter** : min 2 pages dans le meme mood avant de changer l'OST
- **Entry/exit thresholds** : 0.55 pour entrer dans un mood, 0.25 pour en sortir
- **Transition matrix** : plausibilite narrative des transitions (ex: sadness → peaceful = ok, comedy → horror = rare)
- **Strong override** : un score >0.85 ignore le dwell minimum
- **Chapter reset** : premiere page d'un nouveau chapitre = commit immediat

### NarrativeContext (`inference.rs`)

Struct pour enrichir le prompt VLM avec du contexte (previous moods, current OST, look-ahead). **Teste et abandonne** — cree des feedback loops qui degradent la precision de 65% a 40%.

### LlamaServer (`inference.rs`)

Wrapper autour du subprocess llama-server :
- `start()` : lance le serveur sur un port libre, attend qu'il soit ready
- `analyze_mood()` : helper Tauri local / legacy, **pas le chemin HTTP de production de l'extension**
- `describe_page()` : helper/reliquat de recherche des pipelines historiques
- `classify_with_context()` : helper/reliquat de recherche des pipelines historiques
- `summarize_descriptions()` : resume LLM de descriptions (teste en V11, abandonne — regresse)
- `impl Drop` : kill automatique du process
- **Important :** le runtime prod est maintenant aligne sur le winner benchmark via `winner_runtime_intent()`

### HTTP API Server (`server.rs`)

Axum server (port configurable, defaut 8765) pour recevoir les images d'outils externes :
- `POST /api/analyze-window` : **endpoint prod principal** pour la page visible + contexte local
- `POST /api/chapter/page` : ingestion incremental d'une page de chapitre
- `POST /api/chapter/focus` : hint de priorite autour de la page visible
- `POST /api/live/cancel` : annulation cooperative de la requete visible supersedee
- `POST /api/lookup` : lecture du cache mood pour la page visible
- `POST /api/trigger` : re-emission d'un mood deja connu
- `GET /api/cache/status` : etat debug du cache et du pipeline chapitre
- `GET /api/status` : etat du serveur
- `GET /api/moods` : liste des 8 moods dimensionnels

### Extension navigateur (`WebExtension/manga-mood/`)

L'extension n'est plus un endroit ou l'on decide le mood.

- `content.js`
  - detecte la page visible
  - precharge in-page les images du reader reel
  - vise une zone analysee `X-10 .. X+20`
  - charge une fenetre elargie `X-14 .. X+24` pour donner assez de contexte au pipeline
  - demande d'abord `lookup`, puis `analyze_window` si necessaire
- `background.js`
  - orchestre la file visible-page
  - garde la priorite sur la page actuellement a l'ecran
  - nourrit aussi le pipeline chapitre via `chapter/page` et `chapter/focus`
  - annule les requetes visibles supersedees
  - expose un debug popup simplifie
- `popup.js/html/css`
  - montre :
    - visible page
    - already analyzed
    - current work
    - next pages
    - recent events

### Regle d'architecture

La prod n'appelle pas directement le harness benchmark.

- benchmark et prod partagent le **meme noyau winner**
- benchmark = execution offline exhaustive
- prod = execution temps reel priorisee autour de la page visible
- promotion d'un winner :
  1. prototype dans `director_realtest_suite.rs`
  2. extraire / promouvoir les primitives reutilisables dans `winner.rs`
  3. brancher `chapter_pipeline.rs` sur ce noyau
  4. laisser l'extension inchangée sauf si le besoin de contexte change

---

## Resultat final

L'exploration historique du pipeline V6 est terminee. **V6 a ete le meilleur resultat de sa phase historique**, mais n'est plus la reference actuelle.

Depuis, la reference benchmark a evolue :
- **31 pages Blue Lock :** V12 multi-image + Qwen3.5-VL 4B → 23/31 strict, 28/31 relaxed
- **RealTest `BL/1` winner actuel :** `t4b_wide5_selective_hold` + Qwen3-VL-4B-Thinking → **58/70 strict, 68/70 relaxed, 24/70 intensity**
- **Production cible actuelle :** meme backbone winner, aligne via le noyau partage `winner.rs`

Toutes les variantes V7-V11 regressent par rapport a V6 dans la famille "descriptions contextuelles". Voir FINDINGS.md section 7 et RESULTS.md Phase 3 pour les details complets.

**Prochaines etapes possibles :**
- verification E2E navigateur reel + OST
- nouveaux candidats benchmark via `build_suite()`
- promotion propre d'un futur winner via `winner.rs`
- Test sur d'autres sequences manga (autres genres)
- Test avec un modele plus gros (7B+) si GPU le permet
