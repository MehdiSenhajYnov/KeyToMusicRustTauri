# Plan: Pass V8 — Contexte asymetrique 4+2

> **Mise a jour (mars 2026) :** ce plan est historique. Il compare des variantes de contexte textuel autour de V6. La reference benchmark actuelle n'est plus V6/V8, mais :
> - **V12 multi-image + Qwen3.5-VL 4B** sur le 31 pages Blue Lock
> - **V12 historique + Qwen3-VL-4B-Thinking** sur `realtest_benchmark` / `BL/1`

## Contexte

| Pass | Fenetre | Strict | Relaxed |
|------|---------|--------|---------|
| V6 | 4 passe, 0 futur | **22/31 (71%)** | 26/31 (84%) |
| V7 | 2 passe, 2 futur | 21/31 (68%) | **27/31 (87%)** |

V6 = stable en milieu d'arc, V7 = meilleur aux transitions mais desoriente le VLM en milieu d'arc.

V8 combine les deux : **4 passe + 2 futur** (6 descriptions total).
Le budget tokens est OK — le serveur tourne a `-c 32768` et 6 descriptions × ~80 tokens = ~480 tokens.

**Attention :** C'est un modele 4B. Si 4+2 regresse vs V6, on arrete et on reste sur V6 (past-only). Le modele n'est peut-etre pas capable de gerer le contexte futur sans confusion.

---

## Modifications

### 1. inference.rs — rien a changer

`classify_with_context()` accepte deja `previous_descriptions` + `next_descriptions` (modifie pour V7). Aucun changement necessaire.

### 2. director.rs — ajouter Pass V8

Inserer un **Pass V8** juste apres le Pass V7 existant, et avant le Pass 3 (director).

Structure identique au V7, seules les constantes changent :

```rust
// ━━━ Pass V8: Asymmetric context (4 before + 2 after) ━━━━━━━━━━━━

let v8_name = format!("{} (pass V8: asymmetric 4+2)", model_cfg.name);
println!("  {CYAN}{BOLD}━━━ {v8_name} ━━━{RESET}");

const V8_CTX_BEFORE: usize = 4;
const V8_CTX_AFTER: usize = 2;

let v8_cache_path = cache_dir.join(format!("pass_v8_bluelock_{}.json", cache_name));
// ... meme pattern de cache que V6/V7 ...

for (i, img) in images.iter().enumerate() {
    // Past context: up to 4 previous descriptions
    let before_start = if i >= V8_CTX_BEFORE { i - V8_CTX_BEFORE } else { 0 };
    let before_descs: Vec<(u32, &str)> = descriptions[before_start..i]
        .iter().map(|(n, d)| (*n, d.as_str())).collect();

    // Future context: up to 2 next descriptions
    let after_end = std::cmp::min(i + 1 + V8_CTX_AFTER, descriptions.len());
    let after_descs: Vec<(u32, &str)> = descriptions[i+1..after_end]
        .iter().map(|(n, d)| (*n, d.as_str())).collect();

    let result = server.classify_with_context(&img.b64, &before_descs, &after_descs).await;
    // ... meme scoring/printing que V6/V7 ...
}
```

**Copier-coller la structure du V7** et changer uniquement :
- Le nom du pass (`V8: asymmetric 4+2`)
- Les constantes (`V8_CTX_BEFORE = 4`, `V8_CTX_AFTER = 2`)
- Le nom du cache (`pass_v8_bluelock_{model}.json`)

### 3. Pass 3 (director) — rien a changer

Il prend deja `all_results.last()` automatiquement. Si V8 est le dernier pass, le director s'applique dessus.

---

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/mood/director.rs` | Ajouter Pass V8 entre V7 et Pass 3 (~80 lignes, copie de V7 avec constantes differentes) |

**1 seul fichier. inference.rs ne change pas.**

---

## Cache

Fichier : `manga-mood-ai/results/pass_v8_bluelock_{model}.json`
Format : `{ "filename": "mood:intensity" }` (identique aux autres passes)

---

## Commande

```bash
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

Les passes V6 et V7 se chargent depuis le cache. Seul V8 fait de nouvelles inferences (~31 × 8-10s = ~5 min).

## Decision apres le test

- **V8 > V6 (strict)** → V8 devient la methode de production
- **V8 = V6 ou V8 < V6** → On reste sur V6 (past-only). Le modele 4B ne gere pas le futur.
