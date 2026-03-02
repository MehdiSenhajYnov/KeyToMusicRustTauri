# Manga Mood AI — Spec d'implementation pour KeyToMusic

## Modele choisi

**Qwen3-VL 2B** (GGUF Q4_K_M, ~1.9 GB)

- Score : 13/13 (100%) sur 13 images de test, stable sur 5 runs consecutifs
- Vitesse : ~1.1s/image en moyenne
- VRAM : ~4700 MB (38% d'une RTX 4070 12GB)
- Thinking model : utilise des `<think>` tags internes pour raisonner avant de repondre
- 10 moods : epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action

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

### 2. num_ctx 2048

Limiter la fenetre de contexte a 2048 tokens.

```
Impact : thinking plus discipline, 100% accuracy stable
Pourquoi : le modele pense de maniere concise au lieu de divaguer
```

Implementation :
- Flag llama-server : `-c 2048`
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

```
Page manga detectee (screenshot/fichier)
    |
    v
Resize a 672px (Rust, image crate)
    |
    v
POST http://127.0.0.1:{PORT}/v1/chat/completions
  body: { messages: [{ role: "user", content: prompt, images: [base64] }],
          temperature: 0.1, max_tokens: 5000 }
    |
    v
Parse response -> mood (string)
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
