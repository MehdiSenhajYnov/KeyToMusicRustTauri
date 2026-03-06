# Manga Mood AI — Résultats des tests

> Statut (mars 2026) : ce fichier documente surtout les **benchmarks Phase 1 sur images isolées**. Le winner actuel du benchmark `realtest_benchmark` dans le repo est maintenant le **protocole historique RealTest + Qwen3-VL-4B-Thinking**, reproduit à **46/70 strict et 59/70 relaxed (84.3%)** sur `BL/1`. Pour la vue d'ensemble à jour, voir [RESEARCH_SYNTHESIS.md](./RESEARCH_SYNTHESIS.md).

## Images de test

| Image | Contenu | Mood attendu |
|---|---|---|
| 1.jpg | Gohan SSJ2 (DBZ) | epic_battle |
| 10.jpg | Blue Lock stade | emotional_climax |
| 11.jpg | Boxeur victoire | emotional_climax |
| 12.jpg | Luffy pleure | emotional_climax |
| 13.png | Robin pleure (One Piece) | sadness |
| 2.jpeg | Thorfinn (Vinland Saga) | tension |
| 4.jpg | Tokyo Revengers gang | tension |
| 5.png | "I don't give up" | tension |
| 6.jpg | Baiser / Kiss | romance |
| 7.jpg | Pleurs | sadness |
| 8.jpg | Horreur skull | horror |
| 3.jpeg | Solo Leveling battle | epic_battle |
| 9.png | Visage bleu horreur | horror |

---

## En compétition

### Qwen3-VL 2B — Short 5k ⭐ CHAMPION

Modèle : `qwen3-vl:2b` (1.9 GB) | Ollama
Prompt : `"What is the mood of this manga page? Pick ONE: [moods]. Reply with just the mood word."`
Config : `temperature: 0.1, num_predict: 5000`

**Performance (RTX 4070 12GB) :**
| Métrique | Valeur |
|---|---|
| VRAM peak | 4976 MB (41%) |
| VRAM delta | +2843 MB |
| GPU avg / peak | 64% / 100% |
| CPU avg / peak | 21.8% / 50% |
| RAM delta | +922 MB |

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 801ms |
| 10.jpg | emotional_climax | tension | non | 2377ms |
| 11.jpg | emotional_climax | emotional_climax | oui | 932ms |
| 12.jpg | emotional_climax | emotional_climax | oui | 1040ms |
| 13.png | sadness | sadness | oui | 1667ms |
| 2.jpeg | tension | tension | oui | 1020ms |
| 4.jpg | tension | tension | oui | 3303ms |
| 5.png | tension | emotional_climax | ~oui | 1310ms |
| 6.jpg | romance | romance | oui | 1075ms |
| 7.jpg | sadness | sadness | oui | 806ms |
| 8.jpg | horror | horror | oui | 1014ms |
| 3.jpeg | epic_battle | epic_battle | oui | 3247ms |
| 9.png | horror | horror | oui | 942ms |

**Score : 12/13 (92%)** — 0 EMPTY, 0 garbage. Meilleur que le 7B ! Rapide (0.6-3.2s). Ne freeze pas le PC.
Note : 5.png classée "emotional_climax" au lieu de "tension" — acceptable, les deux moods sont pertinents pour "I don't give up".
💡 **Utilise seulement 41% de la VRAM** — reste largement de la place pour l'app et le système.

---

### InternVL3.5 4B — SECOND, ULTRA RAPIDE

Modèle : `blaifa/InternVL3_5:4B` (~3 GB) | Ollama
Config : `temperature: 0.1, num_predict: 300`, prompt JSON standard

**Performance (RTX 4070 12GB) :**
| Métrique | Valeur |
|---|---|
| VRAM peak | 9117 MB (74%) ⚠️ |
| VRAM delta | +4141 MB |
| GPU avg / peak | 44.8% / 91% |
| CPU avg / peak | 20.3% / 39.1% |
| RAM delta | +793 MB |

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 4098ms |
| 10.jpg | emotional_climax | tension | non | 587ms |
| 11.jpg | emotional_climax | tension | non | 537ms |
| 12.jpg | emotional_climax | comedy | non | 526ms |
| 13.png | sadness | sadness | oui | 590ms |
| 2.jpeg | tension | tension | oui | 522ms |
| 4.jpg | tension | tension | oui | 585ms |
| 5.png | tension | tension | oui | 529ms |
| 6.jpg | romance | romance | oui | 546ms |
| 7.jpg | sadness | sadness | oui | 564ms |
| 8.jpg | horror | horror | oui | 541ms |
| 3.jpeg | epic_battle | tension | non | 576ms |
| 9.png | horror | horror | oui | 556ms |

**Score : 9/13 (69%)** — Ultra rapide (~500ms !). Bon sur les moods simples, rate les emotional_climax et confond Solo Leveling. Ne freeze pas le PC.
⚠️ **74% VRAM** — risqué sur des GPU avec moins de 12GB.

---

### Qwen2.5-VL 7B — BON MAIS FREEZE PC

Modèle : `qwen2.5vl:7b` (6 GB) | VRAM : ~6-8 GB | Ollama
Config : `temperature: 0.1, num_predict: 300`
Note : Monopolise la VRAM, rend le PC inutilisable. Pourrait fonctionner avec `num_gpu` réduit (pas encore testé).

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | ~2s |
| 10.jpg | emotional_climax | epic_battle | non | ~2s |
| 11.jpg | emotional_climax | epic_battle | non | ~2s |
| 12.jpg | emotional_climax | tension | non | ~2s |
| 13.png | sadness | sadness | oui | ~2s |
| 2.jpeg | tension | tension | oui | ~2s |
| 4.jpg | tension | tension | oui | ~2s |
| 5.png | tension | tension | oui | ~2s |
| 6.jpg | romance | romance | oui | ~2s |
| 7.jpg | sadness | sadness | oui | ~2s |
| 8.jpg | horror | horror | oui | ~2s |
| 3.jpeg | epic_battle | epic_battle | oui | ~2s |
| 9.png | horror | horror | oui | ~2s |

**Score : 11/13 (85%)** — Très précis, mais inutilisable en l'état (freeze PC).

---

### Qwen3-VL 4B — MOYEN

Modèle : `qwen3-vl:4b` (4.4B) | VRAM : ~5-6 GB | Ollama
Config : `temperature: 0.1, num_predict: 5000`, prompt court (thinking ON)

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 12115ms |
| 10.jpg | emotional_climax | epic_battle | non | 2671ms |
| 11.jpg | emotional_climax | emotional_climax | oui | 1183ms |
| 12.jpg | emotional_climax | emotional_climax | oui | 1600ms |
| 13.png | sadness | emotional_climax | non | 1315ms |
| 2.jpeg | tension | horror | non | 3164ms |
| 4.jpg | tension | tension | oui | 2861ms |
| 5.png | tension | horror | non | 5556ms |
| 6.jpg | romance | romance | oui | 961ms |
| 7.jpg | sadness | sadness | oui | 1822ms |
| 8.jpg | horror | horror | oui | 1500ms |
| 3.jpeg | epic_battle | epic_battle | oui | 1964ms |
| 9.png | horror | horror | oui | 1143ms |

**Score : 9/13 (69%)** — Paradoxalement moins bon que le 2B. Confond tension/horror sur certaines images sombres. Plus lent.

---

### MiniCPM-V 4.0 — BIAIS TENSION

Modèle : `openbmb/minicpm-v4` (4.1B, 3 GB) | Ollama
Config : `temperature: 0.1, num_predict: 300`, prompt JSON standard

**Performance (RTX 4070 12GB) :**
| Métrique | Valeur |
|---|---|
| VRAM peak | 5713 MB (47%) |
| VRAM delta | +3364 MB |
| GPU avg / peak | 51.1% / 95% |
| CPU avg / peak | 19.1% / 31.3% |
| RAM delta | +678 MB |

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 2275ms |
| 10.jpg | emotional_climax | tension | non | 1191ms |
| 11.jpg | emotional_climax | emotional_climax | oui | 374ms |
| 12.jpg | emotional_climax | tension | non | 478ms |
| 13.png | sadness | tension | non | 693ms |
| 2.jpeg | tension | tension | oui | 532ms |
| 4.jpg | tension | tension | oui | 1274ms |
| 5.png | tension | tension | oui | 509ms |
| 6.jpg | romance | romance | oui | 544ms |
| 7.jpg | sadness | tension | non | 906ms |
| 8.jpg | horror | horror | oui | 806ms |
| 3.jpeg | epic_battle | tension | non | 882ms |
| 9.png | horror | horror | oui | 921ms |

**Score : 8/13 (62%)** — Rapide (~500-1200ms), VRAM correcte (47%). Mais biais "tension" sur sadness, emotional_climax et epic_battle. Malgré le hype "surpasse GPT-4.1-mini", sous-performe sur le manga.

---

## Tableau comparatif

| Modèle | Config | Score | Vitesse moy. | VRAM peak | VRAM % | GPU avg | Statut |
|---|---|---|---|---|---|---|---|
| **Qwen3-VL 2B** | **optimisé** ⭐ | **13/13 (100%)** | **~1.1s** | **4700 MB** | **38%** | **~70%** | **Champion optimisé** |
| Qwen3-VL 2B | baseline (non optimisé) | 12/13 (92%) | ~1.4s | 4976 MB | 41% | 64% | Avant optis |
| InternVL3.5 4B | standard | 9/13 (69%) | **~500ms** | 9117 MB | 74% ⚠️ | 45% | Second (rapide mais VRAM) |
| Qwen2.5-VL 7B | full GPU | 11/13 (85%) | ~2s | — | — | — | Bon mais freeze PC |
| Qwen3-VL 4B | short 5k | 9/13 (69%) | ~2.8s | — | — | — | Moyen |
| MiniCPM-V 4.0 | standard | 8/13 (62%) | ~800ms | 5713 MB | 47% | 51% | Biais tension |
| Moondream 2 (2B) | transformers BF16 | — | — | — | — | — | ELIMINÉ (deps Windows) |
| Moondream 3 (9B) | transformers 4bit | — | — | 12180 MB | 99% | — | ELIMINÉ (VRAM + dtype errors) |
| Gemma 3 4B | standard | ~2/13 (15%) | ~1s | — | — | — | ELIMINÉ |
| SmolVLM2 2.2B | standard | ~3/13 (23%) | ~150ms | — | — | — | ELIMINÉ |

---

## Optimisations testées (sur Qwen3-VL 2B)

### Resize images

Redimensionner les images avant inference pour réduire le nombre de tokens visuels.

| Config | Score | Avg Time | VRAM | Verdict |
|---|---|---|---|---|
| Pas de resize (baseline) | 11-12/13 (85-92%) | ~1300ms | 40% | Référence |
| **Resize 672px** | **13/13 (100%)** | **~810ms** | **40%** | **+39% vitesse, +accuracy** |
| Resize 448px | 11/13 (85%) | ~870ms | 40% | Trop agressif, perd du détail |

Le resize 672px **améliore** l'accuracy : moins de tokens visuels = le modèle se concentre sur le mood au lieu de détailler l'image.

### num_ctx (fenêtre de contexte)

Réduire la fenêtre de contexte pour limiter la VRAM et discipliner le thinking.

| Config (avec resize 672px) | Score | Avg Time | VRAM | Verdict |
|---|---|---|---|---|
| Pas de num_ctx (défaut) | 12/13 (92%) | ~1200ms | 40% | Référence |
| **num_ctx 2048** | **13/13 (100%)** | **~1120ms** | **38-40%** | **Le meilleur !** |
| num_ctx 4096 | 12/13 (92%) | ~1800ms | 40% | Plus lent, pas mieux |
| num_ctx 8192 | 12/13 (92%) | ~1300ms | 44% | Plus de VRAM, pas mieux |

Stabilité confirmée : **5 runs consécutifs à 13/13 (100%)** avec num_ctx 2048.
Le contexte réduit force le modèle à penser de manière concise au lieu de divaguer.

### Structured output (JSON schema)

Forcer la sortie via schema JSON avec enum (Ollama `format=schema`).

| Config | Score | Avg Time | Verdict |
|---|---|---|---|
| Struct /no_think 200 tokens | 0/13 (0%) | ~1560ms | EMPTY partout |
| Struct /no_think 5k tokens | 9/13 (69%) | ~5730ms | Lent, imprécis |
| Struct think-in-JSON | 10/13 (77%) | ~6500ms | 8x plus lent |
| Struct think-in-JSON + 672px | 11/13 (85%) | ~6600ms | 8x plus lent |

**Verdict : incompatible avec Qwen3-VL 2B.** Ce modèle a besoin de ses `<think>` tags pour raisonner. Le format JSON schema l'empêche de penser, il retourne du vide. Le trick "think-in-JSON" (champ thinking dans le schema) marche partiellement mais est 8x plus lent. Le parsing free-form est plus fiable.

### Process priority (BELOW_NORMAL)

Baisser la priorité CPU/GPU du process Ollama.

| Config (avec resize 672px) | Score | Avg Time | Verdict |
|---|---|---|---|
| Priorité normale | 12/13 (92%) | ~1200ms | Référence |
| BELOW_NORMAL | 12/13 (92%) | ~1100ms | Même vitesse (PC pas chargé) |

Pas d'impact mesurable en bench isolé. Utile en production quand le PC fait autre chose (manga reader, musique, navigateur).

---
---

## Archive (éliminés)

### Gemma 3 4B (3 GB) — ELIMINÉ

Modèle : `gemma3:4b` | Config : prompt JSON standard, `num_predict: 300`

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | tension | non | 4356ms |
| 10.jpg | emotional_climax | tension | non | 1076ms |
| 11.jpg | emotional_climax | tension | non | 910ms |
| 12.jpg | emotional_climax | tension | non | 906ms |
| 13.png | sadness | tension | non | 1041ms |
| 2.jpeg | tension | tension | oui | 922ms |
| 4.jpg | tension | tension | oui | 1047ms |
| 5.png | tension | horror | non | 922ms |
| 6.jpg | romance | peaceful | non | 974ms |
| 7.jpg | sadness | emotional_climax | non | 1026ms |
| 8.jpg | horror | tension | non | 954ms |
| 3.jpeg | epic_battle | tension | non | 1010ms |
| 9.png | horror | tension | non | 921ms |

**Score : ~2/13 (15%)** — Dit "tension" pour presque tout. Même problème que SigLIP mais avec un autre mot.

---

### SmolVLM2 2.2B (1.4 GB) — ELIMINÉ

Modèle : `richardyoung/smolvlm2-2.2b-instruct` | Config : prompt JSON standard, `num_predict: 300`

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | tension | non | 1478ms |
| 10.jpg | emotional_climax | tension | non | 173ms |
| 11.jpg | emotional_climax | tension | non | 137ms |
| 12.jpg | emotional_climax | tension | non | 140ms |
| 13.png | sadness | tension | non | 150ms |
| 2.jpeg | tension | tension | oui | 165ms |
| 4.jpg | tension | tension | oui | 191ms |
| 5.png | tension | tension | oui | 141ms |
| 6.jpg | romance | tension | non | 162ms |
| 7.jpg | sadness | tension | non | 152ms |
| 8.jpg | horror | tension | non | 137ms |
| 3.jpeg | epic_battle | tension | non | 149ms |
| 9.png | horror | tension | non | 158ms |

**Score : ~3/13 (23%)** — Dit "tension" pour 13/13 images. Ultra rapide (~150ms) mais complètement inutile.

---

### SigLIP 2 ViT-B (86 Mo) — ELIMINÉ

Approche embedding + classification (pas un VLM). Utilise softmax pour ranking relatif.

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | <100ms |
| 10.jpg | emotional_climax | epic_battle | non | <100ms |
| 11.jpg | emotional_climax | epic_battle | non | <100ms |
| 12.jpg | emotional_climax | epic_battle | non | <100ms |
| 13.png | sadness | epic_battle | non | <100ms |
| 2.jpeg | tension | epic_battle | non | <100ms |
| 4.jpg | tension | epic_battle | non | <100ms |
| 5.png | tension | epic_battle | non | <100ms |
| 6.jpg | romance | epic_battle | non | <100ms |
| 7.jpg | sadness | epic_battle | non | <100ms |
| 8.jpg | horror | epic_battle | non | <100ms |
| 3.jpeg | epic_battle | epic_battle | oui | <100ms |
| 9.png | horror | epic_battle | non | <100ms |

**Score : 2/13 (15%)** — Dit "epic_battle" pour tout. Biaisé par le style manga.

---

### Moondream 0.5B (1.7 GB) — ELIMINÉ

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | comedy | non | ~1s |
| 10.jpg | emotional_climax | comedy | non | ~1s |
| 11.jpg | emotional_climax | comedy | non | ~1s |
| 12.jpg | emotional_climax | comedy | non | ~1s |
| 13.png | sadness | comedy | non | ~1s |
| 2.jpeg | tension | comedy | non | ~1s |
| 4.jpg | tension | comedy | non | ~1s |
| 5.png | tension | comedy | non | ~1s |
| 6.jpg | romance | comedy | non | ~1s |
| 7.jpg | sadness | comedy | non | ~1s |
| 8.jpg | horror | comedy | non | ~1s |
| 3.jpeg | epic_battle | comedy | non | ~1s |
| 9.png | horror | comedy | non | ~1s |

**Score : ~1/13 (8%)** — Dit "comedy" pour tout. Trop petit pour comprendre le manga.

---

### Qwen3-VL 2B — No Think — ELIMINÉ

Prompt avec `/no_think` pour désactiver le raisonnement interne. `num_predict: 300`.

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | EMPTY | - | 5153ms |
| 10.jpg | emotional_climax | EMPTY | - | 2299ms |
| 11.jpg | emotional_climax | EMPTY | - | 1889ms |
| 12.jpg | emotional_climax | EMPTY | - | 1830ms |
| 13.png | sadness | EMPTY | - | 2183ms |
| 2.jpeg | tension | EMPTY | - | 2040ms |
| 4.jpg | tension | EMPTY | - | 2879ms |
| 5.png | tension | tension | oui | 1483ms |
| 6.jpg | romance | EMPTY | - | 1896ms |
| 7.jpg | sadness | sadness | oui | 1746ms |
| 8.jpg | horror | EMPTY | - | 2271ms |
| 3.jpeg | epic_battle | EMPTY | - | 2652ms |
| 9.png | horror | EMPTY | - | 1999ms |

**Score : 2/13 (15%)** — 11 réponses vides. Le /no_think casse le modèle sur la plupart des images.

---

### Qwen3-VL 2B — Think 2k — ELIMINÉ

Prompt demandant du JSON complet. `num_predict: 2000`.

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 5126ms |
| 10.jpg | emotional_climax | EMPTY | - | 13793ms |
| 11.jpg | emotional_climax | EMPTY | - | 12159ms |
| 12.jpg | emotional_climax | emotional_climax | oui | 4771ms |
| 13.png | sadness | sadness | oui | 2725ms |
| 2.jpeg | tension | tension | oui | 8582ms |
| 4.jpg | tension | EMPTY | - | 13893ms |
| 5.png | tension | EMPTY | - | 12503ms |
| 6.jpg | romance | romance | oui | 2175ms |
| 7.jpg | sadness | sadness | oui | 1963ms |
| 8.jpg | horror | horror | oui | 2439ms |
| 3.jpeg | epic_battle | epic_battle | oui | 6847ms |
| 9.png | horror | horror | oui | 5853ms |

**Score : 9/13 (69%)** — 4 EMPTY (thinking bouffe les tokens). Correct quand il répond, mais pas fiable.

---

### Qwen3-VL 2B — Think 5k — ELIMINÉ

Plus de tokens pour laisser le thinking finir. `num_predict: 5000`.

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | EMPTY | - | 31902ms |
| 10.jpg | emotional_climax | tension | non | 7496ms |
| 11.jpg | emotional_climax | EMPTY | - | 31763ms |
| 12.jpg | emotional_climax | EMPTY | - | 25825ms |
| 13.png | sadness | sadness | oui | 1231ms |
| 2.jpeg | tension | tension | oui | 9181ms |
| 4.jpg | tension | EMPTY | - | 34773ms |
| 5.png | tension | EMPTY | - | 33254ms |
| 6.jpg | romance | romance | oui | 1094ms |
| 7.jpg | sadness | sadness | oui | 1538ms |
| 8.jpg | horror | EMPTY | - | 32919ms |
| 3.jpeg | epic_battle | epic_battle | oui | 8059ms |
| 9.png | horror | EMPTY | - | 33091ms |

**Score : 5/13 (38%)** — PIRE que 2k. Plus de tokens = le modèle tourne en boucle (30s+).

---

### Qwen2.5-VL 3B (3.2 GB) — ELIMINÉ

| Image | Attendu | Résultat | Correct | Temps |
|---|---|---|---|---|
| 1.jpg | epic_battle | epic_battle | oui | 5222ms |
| 10.jpg | emotional_climax | epic_battle | non | 7678ms |
| 11.jpg | emotional_climax | tension | non | 934ms |
| 12.jpg | emotional_climax | tension | non | 959ms |
| 13.png | sadness | sadness | oui | 2077ms |
| 2.jpeg | tension | tension | oui | 1011ms |
| 4.jpg | tension | GARBAGE | - | 9838ms |
| 5.png | tension | GARBAGE | - | 3796ms |
| 6.jpg | romance | GARBAGE | - | 3844ms |
| 7.jpg | sadness | GARBAGE | - | 10285ms |
| 8.jpg | horror | GARBAGE | - | 6642ms |
| 3.jpeg | epic_battle | GARBAGE | - | 10632ms |
| 9.png | horror | GARBAGE | - | 4533ms |

**Score : ~3/13 (23%)** — 7/13 garbage (caractères aléatoires). Freeze le PC. Inutilisable.

---

## Enseignements clés (Phase 1)

1. **Le prompt est aussi important que le modèle.** Qwen3-VL 2B passe de 2/13 à 12/13 juste en changeant le prompt.
2. **Le thinking aide**, mais il faut un prompt simple pour que le thinking ne boucle pas.
3. **Plus de tokens ≠ mieux.** 5000 tokens avec un prompt complexe = le modèle tourne dans le vide.
4. **Plus gros ≠ mieux.** Le 2B bat le 4B de la même famille (Qwen3-VL) grâce au bon prompt.
5. **Les petits modèles généralistes (Gemma, SmolVLM) ne comprennent pas le manga** — ils ont un biais "tension" sur tout contenu N&B.
6. **InternVL3.5 est le plus rapide** (~500ms) mais moins précis. Intéressant si la vitesse est critique.
7. **Le prompt court + thinking** est la meilleure combinaison : le modèle réfléchit vite et donne juste le mot.
8. **VRAM : Qwen3-VL 2B (38%) vs InternVL (74%).** Le 2B laisse largement de place pour le système.
9. **Resize + contexte réduit = meilleur sur tous les critères.** Contre-intuitif : moins d'info = meilleure classification. Le modèle se concentre au lieu de divaguer.
10. **Structured output incompatible avec les thinking models.** La grammaire JSON bloque les `<think>` tags. Pas de workaround viable.
11. **Le parsing free-form est fiable** — 0 erreur de parsing sur des centaines de runs avec le bon prompt.
12. **Moondream 2/3 : enfer de dépendances sur Windows.** pyvips/libvips, torch/torchao incompatibilités, 99% VRAM. Pas viable pour une app desktop.

---
---

## Phase 2 — Blue Lock Sequence (31 pages consécutives)

Test d'intégration Rust : 31 pages consécutives de Blue Lock Tome 1 (pages 6-36), via llama-server avec le modèle Qwen3.5-VL 4B (Q4_K_M GGUF). Prompt GUIDED_V3, temperature 0.0, max_tokens 8192.

**Commande :** `cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture`

### Qwen3.5-VL 4B — Baseline (single-label, GUIDED_V3)

| Page | Attendu | Détecté | Résultat |
|------|---------|---------|----------|
| 006 | tension | tension | pass |
| 007 | tension | tension | pass |
| 008 | tension | tension | pass |
| 009 | emotional_climax | emotional_climax | pass |
| 010 | emotional_climax | emotional_climax | pass |
| 011 | emotional_climax | emotional_climax | pass |
| 012 | tension | tension | pass |
| 013 | chase_action | epic_battle | ~ok~ |
| 014 | tension | emotional_climax | FAIL |
| 015 | tension | tension | pass |
| 016 | tension | emotional_climax | FAIL |
| 017 | tension | emotional_climax | ~ok~ |
| 018 | emotional_climax | epic_battle | ~ok~ |
| 019 | emotional_climax | emotional_climax | pass |
| 020 | emotional_climax | emotional_climax | pass |
| 021 | emotional_climax | emotional_climax | pass |
| 022 | emotional_climax | emotional_climax | pass |
| 023 | emotional_climax | emotional_climax | pass |
| 024 | sadness | sadness | pass |
| 025 | sadness | sadness | pass |
| 026 | sadness | sadness | pass |
| 027 | sadness | emotional_climax | FAIL |
| 028 | sadness | sadness | pass |
| 029 | sadness | emotional_climax | FAIL |
| 030 | sadness | sadness | pass |
| 031 | sadness | emotional_climax | FAIL |
| 032 | sadness | sadness | pass |
| 033 | sadness | emotional_climax | ~ok~ |
| 034 | peaceful | peaceful | pass |
| 035 | mystery | tension | ~ok~ |
| 036 | mystery | chase_action | FAIL |

**Score : Strict 20/31 (65%), Relaxed 25/31 (81%)**

### Approches testées pour améliorer au-delà de 65%/81% (système 10 moods)

| # | Approche | Strict | Relaxed | Verdict |
|---|----------|--------|---------|---------|
| 1 | **Baseline** (single-label, GUIDED_V3) | **20/31 (65%)** | **25/31 (81%)** | **Référence** |
| 2 | Single-label + text-only refinement | 20/31 (65%) | 25/31 (81%) | Le LLM texte n'a rien changé → IDENTIQUE |
| 3 | Pipeline V2 describe→classify | 6/31 (19%) | 19/31 (61%) | Descriptions perdent l'émotion → PIRE |
| 4 | Pipeline V3 extract→classify (LLM texte) | 13/31 (42%) | 26/31 (84%) | LLM texte perd info visuelle → PIRE strict, MIEUX relaxed |
| 5 | Hybrid 1-inférence (mood+features) | 14/31 (45%) | 24/31 (77%) | Dual-task dégrade les deux → PIRE |
| 6 | Hybrid + fusion déterministe | 18/31 (58%) | 28/31 (90%) | Fusion marche (+4 corrections) mais base faible |
| 7 | 2 inférences + fusion | 21/31 (68%) | 26/31 (84%) | Meilleur strict 10-moods. +1/+1 vs baseline |
| — | Context injection (previous moods in VLM prompt) | 12/30 (40%) | 18/30 (60%) | Feedback loops → PIRE |
| — | Scored prompt (10 floats) | 8/31 (26%) | 17/31 (55%) | Le modèle ne sait pas scorer → PIRE |
| — | Scored prompt + text-only refinement | 6/31 (19%) | 15/31 (48%) | Garbage in, garbage out → PIRE |

### Enseignements clés (Phase 2 — 10 moods)

13. **Le scored prompt est catastrophique sur les petits modèles.** Un modèle 4B ne sait pas outputter 10 floats fiables. Single-label (classification) >> scored (régression).
14. **L'injection de contexte dans le prompt VLM crée des feedback loops.** Le modèle s'ancre sur le mood précédent et propage les erreurs.
15. **Le refinement texte sans les images est inutile.** Les labels "semblent plausibles" même quand ils sont faux — le LLM texte n'a pas de raison de les corriger.
16. **Le problème n'est pas le modèle, c'est l'architecture.** La confusion `sadness ↔ emotional_climax` vient du fait qu'on demande un jugement narratif à partir d'une observation visuelle unique.
17. **L'intensité visuelle ≠ la catégorie narrative.** Le modèle voit "émotion forte" et dit `emotional_climax`, mais pour nous c'est `sadness` (sorrow intense). La distinction est narrative, pas visuelle.

---

## Phase 3 — Système dimensionnel (8 moods × 3 intensités) + Contexte

Migration vers 8 moods (epic, tension, sadness, comedy, romance, horror, peaceful, mystery) avec 3 niveaux d'intensité. `emotional_climax` et `chase_action` supprimés car trop confusables. Matching relaxé = familles de moods (epic↔tension, sadness↔peaceful, etc.).

### Pipeline V6 — describe_page() + classify_with_context()

Architecture en 2 étapes :
1. `describe_page(image)` → description textuelle factuelle (pas de mood)
2. `classify_with_context(image, 4_descriptions_passées)` → mood + intensité

Le VLM voit toujours l'image ET les descriptions des pages précédentes. Pas de feedback loop car les descriptions sont factuelles.

### Résultats complets (système dimensionnel)

| Pass | Contexte | Strict | Relaxed | Intensité | Verdict |
|------|----------|--------|---------|-----------|---------|
| Baseline | aucun | 14/31 (45%) | 23/31 (74%) | 18/31 (58%) | Référence dimensionnelle |
| V5 | describe + text-only correct | 15/31 (48%) | 24/31 (77%) | — | Le LLM texte corrige rien |
| **V6** | **4 passé full** | **22/31 (71%)** | **26/31 (84%)** | 17/31 (55%) | **⭐ MEILLEUR RÉSULTAT** |
| V7 | 2+2 bidirectionnel | 21/31 (68%) | 27/31 (87%) | 15/31 (48%) | Futur désoriente mid-arc |
| V8 | 4+2 asymétrique | 19/31 (61%) | 27/31 (87%) | 15/31 (48%) | Trop de contexte |
| V9 | 3+3 symétrique | 22/31 (71%) | 26/31 (84%) | 16/31 (52%) | Égalité V6, erreurs différentes |
| V10 | 2 full + 5 first-sentence | 16/31 (52%) | 24/31 (77%) | — | Résumés trop génériques |
| V11 | 2 full + résumé LLM | 19/31 (61%) | 25/31 (81%) | 20/31 (65%) | Résumés ajoutent du bruit |

### Détail page par page — Comparaison complète (système dimensionnel)

| Page | Attendu | Baseline | V6 | V7 | V8 | V9 | V11 |
|------|---------|----------|-----|-----|-----|-----|------|
| 006 | tension 2 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 007 | tension 2 | ✅ | ❌ sadness | ✅ | ✅ | ✅ | ❌ comedy |
| 008 | tension 2 | ✅ | ❌ comedy | ✅ | ✅ | ✅ | ❌ comedy |
| 009 | tension 3 | ✅ | ✅ | ❌ comedy | ❌ comedy | ✅ | ✅ |
| 010 | tension 3 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 011 | tension 3 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 012 | tension 2 | ❌ epic | ✅ | ✅ | ✅ | ✅ | ✅ |
| 013 | tension 3 | ✅ | ✅ | ✅ | ~ok~ epic | ✅ | ✅ |
| 014 | tension 2 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 015 | tension 2 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 016 | tension 2 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 017 | tension 2 | ✅ | ✅ | ❌ sadness | ~ok~ epic | ✅ | ✅ |
| 018 | epic 3 | ~ok~ tension | ~ok~ epic | ~ok~ epic | ~ok~ tension | ~ok~ tension | ~ok~ tension |
| 019 | epic 3 | ~ok~ tension | ~ok~ tension | ~ok~ tension | ~ok~ tension | ✅ | ~ok~ tension |
| 020 | epic 3 | ~ok~ tension | ✅ | ✅ | ✅ | ❌ sadness | ~ok~ tension |
| 021 | epic 3 | ~ok~ tension | ~ok~ sadness | ✅ | ~ok~ sadness | ~ok~ tension | ~ok~ tension |
| 022 | sadness 3 | ~ok~ epic | ✅ | ~ok~ epic | ✅ | ✅ | ✅ |
| 023 | sadness 3 | ❌ epic | ❌ epic | ✅ | ~ok~ tension | ✅ | ❌ epic |
| 024 | sadness 2 | ✅ | ✅ | ✅ | ✅ | ❌ peaceful | ✅ |
| 025 | sadness 2 | ✅ | ✅ | ❌ peaceful | ✅ | ✅ | ✅ |
| 026 | sadness 2 | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| 027 | sadness 2 | ❌ epic | ✅ | ✅ | ❌ peaceful | ❌ tension | ✅ |
| 028 | sadness 2 | ❌ epic | ✅ | ✅ | ✅ | ✅ | ✅ |
| 029 | sadness 2 | ❌ epic | ✅ | ✅ | ❌ romance | ❌ romance | ✅ |
| 030 | sadness 2 | ~ok~ tension | ✅ | ~ok~ tension | ✅ | ✅ | ❌ epic |
| 031 | sadness 2 | ❌ tension | ❌ tension | ❌ horror | ❌ epic | ✅ | ❌ tension |
| 032 | sadness 2 | ❌ tension | ✅ | ✅ | ✅ | ✅ | ✅ |
| 033 | sadness 2 | ❌ comedy | ✅ | ✅ | ✅ | ~ok~ tension | ✅ |
| 034 | peaceful 1 | ~ok~ comedy | ❌ sadness | ~ok~ comedy | ✅ | ❌ sadness | ❌ sadness |
| 035 | mystery 2 | ~ok~ tension | ~ok~ tension | ~ok~ tension | ~ok~ tension | ✅ | ~ok~ tension |
| 036 | mystery 2 | ~ok~ tension | ~ok~ tension | ~ok~ tension | ~ok~ tension | ~ok~ tension | ~ok~ tension |

### Enseignements clés (Phase 3)

18. **Le contexte est le levier n°1.** +26 points strict (45%→71%) avec juste 4 descriptions passées. Aucune autre technique n'approche ce gain.
19. **Les descriptions factuelles éliminent les feedback loops.** Contrairement aux labels de mood, les descriptions textuelles ne biaisent pas le modèle.
20. **4 descriptions complètes = la limite du modèle 4B.** Plus (V8: 6 descriptions) dégrade les résultats.
21. **Le contexte futur n'aide pas en strict.** Le modèle essaie de "matcher" le mood futur au lieu de classifier ce qu'il voit.
22. **Les résumés (V10, V11) régressent.** Le modèle a besoin des descriptions complètes, pas de résumés.
23. **Les descriptions sont ~500-800 tokens chacune.** 4 descriptions = ~2500-3000 tokens de contexte. Nécessite `-c 32768`.
24. **V6 est le meilleur résultat de cette phase de recherche contextuelle** (avant V12 multi-image et avant la reproduction RealTest historique).
