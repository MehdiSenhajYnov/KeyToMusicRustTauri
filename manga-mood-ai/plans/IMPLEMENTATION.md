# Manga Mood AI — Spec d'implementation pour KeyToMusic

## Modele choisi

**Production actuelle : Qwen3-VL 2B** (GGUF Q4_K_M, ~1.9 GB) — deploye dans l'app (classification simple sans contexte).
**Reference RealTest par defaut : Qwen3-VL-4B-Thinking** — protocole historique V12 reproduit sur Linux, **46/70 strict et 59/70 relaxed (84.3%)** sur `BL/1`.
**Reference recherche 31 pages Blue Lock : Qwen3.5-VL 4B** (GGUF Q4_K_M, ~2.5 GB) — meilleur resultat sur le benchmark sequentiel 31 pages, avec V12 multi-image a **23/31 strict, 28/31 relaxed**.

| Modele | Images isolees | Sequence (31 pages) | RealTest `BL/1` | Vitesse | VRAM |
|--------|-----------------|---------------------|-----------------|---------|------|
| Qwen3-VL 2B | 18/18 (100%) | Non teste | Non teste en RealTest par defaut | ~1.1s | 38% |
| Qwen3.5-VL 4B | Non teste | 23/31 strict (74%), 28/31 relaxed (90%) avec V12 | ~30/70 strict, ~36/70 relaxed sur le variant moderne BL/1 | ~1.8s/window (moderne) | ~50% |
| Qwen3-VL-4B-Thinking | Non teste | Variante dediee thinking, pas winner sur le 31 pages | **46/70 strict, 59/70 relaxed (84.3%)** avec protocole historique V12 | ~9.5s/window (historique) | ~50% |

- Thinking model : utilise des `<think>` tags internes pour raisonner avant de repondre
- **8 moods dimensionnels** : epic, tension, sadness, comedy, romance, horror, peaceful, mystery — chacun avec intensite 1-3
- Ancien systeme (10 moods avec emotional_climax, chase_action) abandonne — trop confusable
- **Pipeline V12 (reference benchmark) :** 3 pages consecutives + vote majoritaire
- **Pipeline V6 :** historique important, mais depasse comme reference benchmark

---

## Runtime : llama.cpp server (pas Ollama)

### Pourquoi pas Ollama
- Ollama = install externe que l'utilisateur doit faire lui-meme
- Overhead Go/HTTP (+10-30% latence)
- Moins de controle sur les parametres (KV cache, flash attention, etc.)

### Architecture
- **llama-server** : binaire standalone (~50 MB), auto-telecharge dans `data/bin/` au premier lancement (meme pattern que yt-dlp et ffmpeg)
- **Modele GGUF** : auto-telecharge dans `data/models/` au premier lancement (~1.9 GB)
- Communication via **HTTP localhost** (API compatible OpenAI)
- Lance en subprocess par le backend Rust, tue a la fermeture de l'app

### Lifecycle du serveur
1. User active la feature "Manga Mood" dans les settings
2. KeyToMusic telecharge llama-server + modele GGUF si pas deja present
3. Lance `llama-server` en subprocess sur un port local random
4. Le serveur reste actif tant que la feature est activee
5. A la fermeture de l'app ou desactivation de la feature : kill le process

---

## Optimisations a appliquer

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

- **Production (2B, sans contexte) :** `-c 2048` — thinking discipline, 100% accuracy stable
- **Pipeline V6 (4B, avec contexte) :** `-c 32768` — necessaire pour les 4 descriptions (~2500-3000 tokens) + image + prompt

Implementation :
- Flag llama-server : `-c 2048` (2B) ou `-c 32768` (4B avec pipeline V6)
- Fixe au lancement, pas besoin de changer par requete

### 3. Prompt court

```
What is the mood of this manga page? Pick ONE: epic_battle, tension, sadness,
comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action.
Reply with just the mood word.
```

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

### Production (2B, classification simple)
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

### Pipeline V6 (4B, classification contextuelle)
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

### Mode simple (production actuelle, 2B)
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

### Mode contextuel (pipeline V6, 4B)
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

---

## Telechargements auto (premier lancement)

| Fichier | Taille | Source | Destination |
|---|---|---|---|
| llama-server | ~50 MB | GitHub releases llama.cpp | `data/bin/llama-server` |
| qwen3-vl-2b GGUF | ~1.9 GB | HuggingFace | `data/models/` |

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

## Architecture implementee (Rust)

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
- `analyze_mood()` : single-label classification (production, 10 moods legacy)
- `describe_page()` : description factuelle d'une page manga (etape 1 du pipeline V6)
- `classify_with_context()` : classification avec image + descriptions passees (etape 2 du pipeline V6)
- `summarize_descriptions()` : resume LLM de descriptions (teste en V11, abandonne — regresse)
- `impl Drop` : kill automatique du process
- **Dead code :** `analyze_mood_scored()`, `refine_moods_from_labels()`, `classify_batch()`, `extract_features_manga()`, `extract_hybrid()`, `normalize_emotion()`, `parse_hybrid_response()`

### HTTP API Server (`server.rs`)

Axum server (port configurable, defaut 8765) pour recevoir les images d'outils externes :
- `POST /api/analyze` : base64 image → mood + event Tauri (classification simple)
- `POST /api/analyze-v2` : base64 image → mood via 2 inferences (pipeline V6 contextuel)
- `GET /api/status` : etat du serveur
- `GET /api/moods` : liste des 8 moods dimensionnels

---

## Resultat final

L'exploration historique du pipeline V6 est terminee. **V6 a ete le meilleur resultat de cette phase : 22/31 strict (71%), 26/31 relaxed (84%).**

Depuis, la reference benchmark a evolue :
- **31 pages Blue Lock :** V12 multi-image + Qwen3.5-VL 4B → 23/31 strict, 28/31 relaxed
- **RealTest `BL/1` par defaut dans le repo :** protocole historique V12 + Qwen3-VL-4B-Thinking → 46/70 strict, 59/70 relaxed (84.3%)

Toutes les variantes V7-V11 regressent par rapport a V6 dans la famille "descriptions contextuelles". Voir FINDINGS.md section 7 et RESULTS.md Phase 3 pour les details complets.

**Prochaines etapes possibles :**
- Integration du pipeline V6 dans les endpoints de production (server.rs)
- Test sur d'autres sequences manga (autres genres)
- Test avec un modele plus gros (7B+) si GPU le permet
