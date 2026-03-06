# Plan: Pass V6 — Classification VLM avec contexte de descriptions

> **Mise a jour (mars 2026) :** ce plan est historique. Il documente la piste V6 "descriptions contextuelles", qui a ensuite ete depassee comme reference benchmark par :
> - **V12 multi-image + Qwen3.5-VL 4B** sur le benchmark 31 pages Blue Lock
> - **V12 historique + Qwen3-VL-4B-Thinking** sur le benchmark RealTest `BL/1` par defaut dans le repo
> Voir `manga-mood-ai/research/RESEARCH_SYNTHESIS.md` pour l'etat de reference.

## Concept

Le VLM voit l'image courante **ET** les descriptions textuelles des pages precedentes.
C'est different de tout ce qu'on a teste :

| Approche testee | Image au classify? | Contexte injecte | Resultat |
|---|---|---|---|
| Pass 1 (baseline) | Oui | Aucun | 45% strict |
| Pass 3 ancien (describe→classify) | **Non** (text-only) | Descriptions batch | 19% strict |
| Context injection (ancien) | Oui | **Mood labels** (biaises) | 40% strict |
| V5 (describe+correct) | **Non** (text-only correcteur) | Descriptions + labels | +1 correction |
| **V6 (ce plan)** | **Oui** | **Descriptions factuelles** | A tester |

**Pourquoi ca devrait marcher :** Le VLM garde l'image (sa force) + recoit du contexte narratif factuel (pas des labels potentiellement faux). Pas de feedback loop car les descriptions ne contiennent pas de jugement de mood.

---

## Architecture du Pass V6

```
Pour chaque page N (dans l'ordre) :
  1. VLM describe(image_N) → description_N          [deja cache depuis V5]
  2. VLM classify(image_N, [desc_{N-4}...desc_{N-1}]) → mood_N   [NOUVEAU]
```

- Etape 1 reutilise le cache existant (`pass_v5_descriptions_{model}.json`)
- Etape 2 est la seule nouvelle inference : le VLM voit l'image + 4 descriptions precedentes

**Fenetre glissante de 4 pages** : suffisant pour le contexte narratif, ne surcharge pas le contexte 2048 tokens (4 descriptions × ~80 tokens = ~320 tokens de contexte).

---

## Implementation dans le test benchmark

### Fichier a modifier

`src-tauri/src/mood/director.rs` — ajouter un Pass V6 dans le test `bluelock_sequence`, juste apres le Pass 2 (V5) existant et avant le Pass 3 (director).

### Nouvelle methode dans inference.rs

Ajouter dans `impl LlamaServer` (fichier `src-tauri/src/mood/inference.rs`) :

```rust
/// V6: Classify mood with image + contextual descriptions of previous pages.
/// Unlike text-only classify, the VLM sees the current image.
/// Unlike mood-label injection, the context is factual descriptions (no bias).
pub async fn classify_with_context(
    &self,
    image_base64: &str,
    previous_descriptions: &[(u32, &str)],  // [(page_num, description)]
) -> Result<(BaseMood, MoodIntensity), String>
```

### Prompt V6

```
{context_block}

Look at this manga page. Based on what you see AND the narrative context above,
what is the mood of THIS page for soundtrack purposes?

Pick ONE mood: epic, tension, sadness, comedy, romance, horror, peaceful, mystery
Pick intensity: 1 (calm), 2 (moderate), 3 (peak)

Reply: MOOD INTENSITY
Example: sadness 2
```

Ou `{context_block}` est construit dynamiquement :

```
Previous pages in this chapter:
- Page 22: "A character stands alone in the rain, head bowed. Dark heavy shading..."
- Page 23: "Close-up on tears streaming down. Another character reaches out..."
- Page 24: "Two characters embracing. Soft shading, emotional atmosphere..."
- Page 25: "Silent panel, character sitting alone. Minimal background..."
```

Si c'est la premiere page (pas de contexte), le bloc est simplement omis et on tombe sur le prompt standard.

### HTTP call

Meme pattern que `analyze_mood()` existant (lignes 262-336 de inference.rs) :
- POST `/v1/chat/completions`
- Image en base64 dans le content array
- Le context_block est du texte avant l'image dans le message user
- `max_tokens: 5000` (pour le thinking), `temperature: 0.1`

### Parsing

Reutiliser `parse_mood_intensity_response()` existant (lignes 1531-1582 de inference.rs).
Il cherche deja le pattern `\b(mood)\s+([123])\b` avec fallback keyword-only.

---

## Integration dans le benchmark test

### Placement dans bluelock_sequence

Inserer **entre le Pass 2 (V5) et le Pass 3 (director)**, vers la ligne 1193 de director.rs.

### Pseudo-code du Pass V6

```rust
// ━━━ Pass V6: VLM classify with description context ━━━━━━━━━━━━━━━

let v6_name = format!("{} (pass V6: describe-context classify)", model_cfg.name);
println!("  {CYAN}{BOLD}━━━ {v6_name} ━━━{RESET}");

// Reuse descriptions from V5 cache (already loaded above)
// descriptions: Vec<(u32, String)> — already available from Pass 2

const CONTEXT_WINDOW: usize = 4; // Number of previous descriptions to include

let v6_cache_path = cache_dir.join(format!("pass_v6_bluelock_{}.json", cache_name));
let cached_v6: Option<HashMap<String, String>> = /* load from disk like Pass 1 */;

let mut v6_results: Vec<PageResult> = Vec::new();
let mut v6_cs = 0u32;
let mut v6_cr = 0u32;
let mut v6_int = 0u32;

if let Some(cached) = cached_v6 {
    // Load from cache (same pattern as Pass 1)
    // ...
} else {
    for (i, img) in images.iter().enumerate() {
        let page_num = GROUND_TRUTH[i].0;

        // Build context: up to CONTEXT_WINDOW previous descriptions
        let start = if i >= CONTEXT_WINDOW { i - CONTEXT_WINDOW } else { 0 };
        let prev_descs: Vec<(u32, &str)> = descriptions[start..i]
            .iter()
            .map(|(n, d)| (*n, d.as_str()))
            .collect();

        // VLM inference: image + description context
        let result = server.classify_with_context(&img.b64, &prev_descs).await;

        match result {
            Ok((mood, intensity)) => {
                let detected = mood.as_str().to_string();
                let det_int = intensity.to_u8();
                // Score against ground truth (same logic as Pass 1)
                // Print line, accumulate stats
            }
            Err(e) => { /* handle error */ }
        }
    }
    // Save to cache
}

// Print summary: strict/relaxed/intensity + delta from Pass 1
// Push to all_results
```

### Cache

Meme format que Pass 1 : `{ "filename": "mood:intensity" }` dans `pass_v6_bluelock_{model}.json`.
Permet de re-run le test sans re-inference (les 31 images prennent ~60s).

### Pass 3 (director) s'applique automatiquement

Le Pass 3 existant prend deja `all_results.last()` comme source. Si V6 est le dernier pass avant le director, il sera automatiquement smooth par le MoodDirector.

---

## Variantes a considerer

### Fenetre de contexte

Tester avec `CONTEXT_WINDOW = 2, 4, 6`. Plus de contexte = meilleur raisonnement narratif, mais plus de tokens = potentiellement plus lent et risque de depasser 2048 ctx.

Estimation tokens :
- Image : ~1024 tokens (avec `--image-min-tokens 1024`)
- 4 descriptions × 80 tokens = ~320 tokens
- Prompt + reponse : ~200 tokens
- Total : ~1544 tokens → OK pour ctx 2048

Avec 6 descriptions : ~1704 tokens → encore OK mais serré.

### Descriptions cumulatives vs fenetre

Option A : fenetre glissante (4 dernieres pages) — recommande, simple
Option B : toutes les descriptions precedentes resumes en 1-2 phrases — plus complexe, necessite un step de summarization

Commencer par Option A.

---

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/mood/inference.rs` | Ajouter `classify_with_context()` (~60 lignes) |
| `src-tauri/src/mood/director.rs` | Ajouter Pass V6 dans `bluelock_sequence` (~80 lignes) |

**Aucun autre fichier touche.** C'est uniquement du code de test + 1 nouvelle methode d'inference.

---

## Commande pour lancer

```bash
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

### Pre-requis

- llama-server + modele GGUF telecharges (deja fait)
- Cache des descriptions V5 present (`manga-mood-ai/results/pass_v5_descriptions_*.json`) — sinon le test les regenere automatiquement
- Pour forcer un re-run du V6 (ignorer le cache) : supprimer `manga-mood-ai/results/pass_v6_bluelock_*.json`

---

## Resultat attendu

Si le contexte de descriptions aide le VLM a distinguer "sadness intense" de "epic", on devrait voir :
- Pages 22-33 (arc sadness) : plus de `sadness` au lieu de `epic`/`tension`
- Strict > 48% (baseline V5 actuel)
- Le gain principal est sur les pages ou le VLM confond intensite visuelle avec categorie

Si ca ne marche pas, ca confirme que le probleme est dans le VLM 4B lui-meme (pas dans le manque de contexte).

---

## Apres le test

Si V6 > baseline :
1. Integrer `classify_with_context()` dans le vrai endpoint `POST /api/analyze` de `server.rs`
2. Le `DescriptionBuffer` existant dans server.rs accumule deja les descriptions — il suffit de les passer au nouveau classify
3. Le flux en production devient : extension envoie image → backend describe + classify_with_context → MoodDirector → playback

Si V6 = baseline :
1. Le contexte ne suffit pas, le VLM 4B a une limite intrinseque
2. Explorer : multi-image (3 images simultanées) ou modele plus gros (8B)
