# Plan: Pass V7 — Contexte bidirectionnel (passe + futur)

> **Mise a jour (mars 2026) :** ce plan est historique. Au moment de son ecriture, V6 etait le meilleur resultat observe. Depuis, la reference a ete depassee par :
> - **V12 multi-image + Qwen3.5-VL 4B** sur le benchmark 31 pages Blue Lock
> - **V12 historique + Qwen3-VL-4B-Thinking** sur le benchmark RealTest `BL/1` par defaut dans le repo
> Voir `manga-mood-ai/research/RESEARCH_SYNTHESIS.md` pour l'etat a jour.

## Contexte

V6 (contexte passe uniquement, fenetre de 4) a donne **22/31 (71%) strict, 26/31 (84%) relaxed** — meilleur resultat a ce stade de l'exploration. Mais 2 regressions sur les premieres pages (contexte trop court) et 1 erreur sur la transition sadness→peaceful (page 34) car le VLM ne voit pas que l'ambiance change apres.

L'extension navigateur charge souvent le chapitre entier ou plusieurs pages d'avance. On a le futur — autant l'utiliser.

## Ce qui change par rapport a V6

V6 actuel (ligne 1294 de director.rs) :
```rust
// Contexte = pages AVANT uniquement
let prev_descs: Vec<(u32, &str)> = descriptions[start..i]  // [i-4 .. i-1]
```

V7 :
```rust
// Contexte = pages AVANT + pages APRES
let before_descs: Vec<(u32, &str)> = descriptions[before_start..i]  // [i-2 .. i-1]
let after_descs: Vec<(u32, &str)> = descriptions[i+1..after_end]    // [i+1 .. i+2]
```

---

## Modifications

### 1. Nouvelle methode dans inference.rs

Modifier `classify_with_context()` (ligne 532 de `src-tauri/src/mood/inference.rs`) pour accepter le contexte futur, OU creer une nouvelle methode `classify_with_bidirectional_context()`.

**Option recommandee : modifier la signature existante** car c'est juste un parametre en plus.

```rust
pub async fn classify_with_context(
    &self,
    image_base64: &str,
    previous_descriptions: &[(u32, &str)],
    next_descriptions: &[(u32, &str)],      // NEW — empty = V6 behavior
) -> Result<MoodTag, String>
```

### 2. Nouveau prompt

Le bloc de contexte (lignes 540-550 de inference.rs) devient :

```rust
let context_block = {
    let mut block = String::new();
    if !previous_descriptions.is_empty() {
        block.push_str("Previous pages in this chapter:\n");
        for (page_num, desc) in previous_descriptions {
            writeln!(block, "- Page {}: \"{}\"", page_num, desc);
        }
        block.push('\n');
    }
    if !next_descriptions.is_empty() {
        block.push_str("Next pages in this chapter:\n");
        for (page_num, desc) in next_descriptions {
            writeln!(block, "- Page {}: \"{}\"", page_num, desc);
        }
        block.push('\n');
    }
    block
};
```

Le prompt principal (lignes 552-567) reste identique — il dit deja "Based on what you see AND the narrative context above". Le VLM verra naturellement les descriptions avant ET apres.

### 3. Nouveau pass dans le benchmark (director.rs)

Ajouter un **Pass V7** dans `bluelock_sequence` test, juste apres le Pass V6 existant (apres la ligne 1372 de director.rs), et avant le Pass 3 (director) qui commence ligne 1374.

**Copier la structure du V6** (lignes 1195-1372) et modifier uniquement la construction du contexte :

```rust
// ━━━ Pass V7: Bidirectional context (past + future) ━━━━━━━━━━━━

let v7_name = format!("{} (pass V7: bidirectional context)", model_cfg.name);
println!("  {CYAN}{BOLD}━━━ {v7_name} ━━━{RESET}");

// Same description cache as V6
// CONTEXT_BEFORE = 2, CONTEXT_AFTER = 2 (total 4 descriptions, same token budget)
const CTX_BEFORE: usize = 2;
const CTX_AFTER: usize = 2;

// V7 cache: separate file to not conflict with V6
let v7_cache_path = cache_dir.join(format!("pass_v7_bluelock_{}.json", cache_name));
// ... same cache load pattern as V6 ...

for (i, img) in images.iter().enumerate() {
    // Past context: up to CTX_BEFORE descriptions
    let before_start = if i >= CTX_BEFORE { i - CTX_BEFORE } else { 0 };
    let before_descs: Vec<(u32, &str)> = descriptions[before_start..i]
        .iter().map(|(n, d)| (*n, d.as_str())).collect();

    // Future context: up to CTX_AFTER descriptions
    let after_end = std::cmp::min(i + 1 + CTX_AFTER, descriptions.len());
    let after_descs: Vec<(u32, &str)> = descriptions[i+1..after_end]
        .iter().map(|(n, d)| (*n, d.as_str())).collect();

    let result = server.classify_with_context(&img.b64, &before_descs, &after_descs).await;
    // ... same scoring/printing as V6 ...
}
```

### 4. Mettre a jour l'appel V6 existant

L'appel V6 existant (ligne 1299 de director.rs) doit passer un slice vide pour `next_descriptions` pour garder le meme comportement :

```rust
// V6: past only (no future)
let result = server.classify_with_context(&img.b64, &prev_descs, &[]).await;
```

---

## Budget tokens

Fenetre totale = 4 descriptions (2 avant + 2 apres), meme budget que V6 (4 avant) :
- Image : ~1024 tokens
- 4 descriptions × ~80 tokens = ~320 tokens
- Prompt + reponse : ~200 tokens
- Total : ~1544 tokens → OK pour ctx 2048

## Variantes a tester

Si les resultats V7 (2+2) sont bons, on peut aussi tester :
- **V7b : 3+3** (6 descriptions total, ~1784 tokens — serre mais OK)
- **V7c : 4+2** (asymetrique, plus de passe que de futur)
- **V7d : 1+1** (minimal, pour voir si meme peu de contexte bidirectionnel aide)

Pour l'instant, commencer par **2+2** qui est le split le plus equilibre dans le meme budget tokens que V6.

---

## Cache

Fichier : `manga-mood-ai/results/pass_v7_bluelock_{model}.json`
Format : `{ "filename": "mood:intensity" }` (identique aux autres passes)

Pour forcer un re-run : supprimer ce fichier.

---

## Fichiers a modifier

| Fichier | Modification |
|---------|-------------|
| `src-tauri/src/mood/inference.rs` | Ajouter param `next_descriptions` a `classify_with_context()` + modifier le context_block (lignes 532-613) |
| `src-tauri/src/mood/director.rs` | 1. Mettre a jour l'appel V6 existant (ligne 1299) pour passer `&[]` en 3e param. 2. Ajouter Pass V7 entre V6 (ligne 1372) et Pass 3 (ligne 1374) |

**Aucun autre fichier touche.**

---

## Commande

```bash
# Supprimer le cache V6 pour re-run avec la nouvelle signature (3 params au lieu de 2)
# Le cache V6 existant reste compatible car on passe &[] en futur = meme comportement
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

## Resultat attendu

- Page 7 et 8 (regressions V6) : le futur (descriptions pages 9-10 = tension) devrait corriger sadness/comedy → tension
- Page 34 (peaceful classee sadness) : le futur (descriptions pages 35-36 = mystery/tension) devrait aider a detecter la transition
- Objectif : >71% strict, >84% relaxed
