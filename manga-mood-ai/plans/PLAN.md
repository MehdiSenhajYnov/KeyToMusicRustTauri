# Manga Mood AI — Plan d'implémentation v5

> **Statut historique :** ce document capture l'etat ou **V6** etait considere comme vainqueur.
>
> **Mise a jour (mars 2026) :**
> - **31 pages Blue Lock :** le meilleur resultat documente est maintenant **V12 multi-image + Qwen3.5-VL 4B** a **23/31 strict, 28/31 relaxed**
> - **RealTest `BL/1` par defaut dans le repo :** **V12 historique + Qwen3-VL-4B-Thinking** a **47/70 strict, 60/70 relaxed (85.7%)**
> - Ce document reste utile pour comprendre le raisonnement qui a mene a V6, mais il n'est plus la reference benchmark actuelle.

## Architecture actuelle : Pipeline V6 (Describe + Classify with Context)

### Pourquoi cette architecture

Après avoir testé 15+ approches sur les systèmes 10 moods et 8 moods dimensionnels :

1. **Le contexte narratif est le levier n°1** — +26 points strict (45% → 71%) avec juste 4 descriptions passées
2. **Les descriptions factuelles éliminent les feedback loops** — contrairement aux labels de mood
3. **Le VLM doit toujours voir l'image** — le text-only classify donne 19%

### Vue d'ensemble

```
Image manga (page N)
    │
    ▼
Étape 1 : describe_page(image_N)
  ┌──────────────────────────────────────────────┐
  │ VLM décrit la page factuellement              │
  │ Pas de classification, pas de mood            │
  │ Output : ~500-800 tokens de description       │
  └──────────────────────────────────────────────┘
    │
    ├──→ description_N (stockée dans buffer)
    │
    ▼
Étape 2 : classify_with_context(image_N, descriptions[N-4..N-1])
  ┌──────────────────────────────────────────────┐
  │ VLM voit : image + 4 descriptions passées     │
  │ Prompt : mood dimensionnel (8 moods × 3)      │
  │ Analyse step-by-step + contexte narratif      │
  └──────────────────────────────────────────────┘
    │
    ├──→ mood + intensité (ex: "tension 3")
    │
    ▼
MoodDirector (smoothing, dwell, transitions)
    │
    ▼
Soundtrack
```

---

## Résultats historiques complets

### Système 10 moods (Phase 1-2)

| Pass | Approche | Strict | Relaxed | Problème |
|------|----------|--------|---------|----------|
| 1 | VLM single-label (GUIDED_V3) | 20/31 (65%) | 25/31 (81%) | Confond intensité/catégorie |
| 2 | Single-label + text refinement | 20/31 (65%) | 25/31 (81%) | Le LLM texte n'a rien changé |
| 3 | Pipeline V2 describe→classify | 6/31 (19%) | 19/31 (61%) | Descriptions perdent l'émotion |
| 4 | Pipeline V3 extract→classify (LLM texte) | 13/31 (42%) | 26/31 (84%) | LLM texte perd l'info visuelle |
| 5 | Hybrid 1-inférence (mood+features) | 14/31 (45%) | 24/31 (77%) | Dual-task neutral-spam |
| 6 | Hybrid + fusion déterministe | 18/31 (58%) | 28/31 (90%) | Bonne fusion (+4) mais base faible |
| 7 | 2 inférences + fusion | 21/31 (68%) | 26/31 (84%) | determination-spam limite fusion |
| — | Context injection (mood labels) | 12/30 (40%) | 18/30 (60%) | Feedback loops |
| — | Scored prompt (10 floats) | 8/31 (26%) | 17/31 (55%) | Le 4B ne sait pas régresser |

### Système 8 moods dimensionnels × 3 intensités (Phase 3)

| Pass | Contexte | Strict | Relaxed | Problème |
|------|----------|--------|---------|----------|
| Baseline | aucun | 14/31 (45%) | 23/31 (74%) | Pas de contexte narratif |
| V5 | describe + text-only correct | 15/31 (48%) | 24/31 (77%) | LLM texte ne corrige rien |
| **V6** | **4 passé full** | **22/31 (71%)** | **26/31 (84%)** | **⭐ Meilleur résultat de cette phase** |
| V7 | 2+2 bidirectionnel | 21/31 (68%) | 27/31 (87%) | Futur désoriente mid-arc |
| V8 | 4+2 asymétrique | 19/31 (61%) | 27/31 (87%) | Trop de contexte |
| V9 | 3+3 symétrique | 22/31 (71%) | 26/31 (84%) | Égalité V6, erreurs différentes |
| V10 | 2 full + 5 first-sentence | 16/31 (52%) | 24/31 (77%) | Résumés trop génériques |
| V11 | 2 full + résumé LLM | 19/31 (61%) | 25/31 (81%) | Résumés ajoutent du bruit |

**Meilleur résultat dans cette phase de planification : V6 — 22/31 (71%) strict, 26/31 (84%) relaxed.**

**Conclusion :** Pour cette phase d'exploration des variantes de contexte textuel, V6 est le meilleur pipeline. Cette conclusion a ensuite ete depassee par V12 multi-image sur le benchmark 31 pages, puis par le protocole historique RealTest reproduit sur `BL/1`.

---

## Décisions de design validées

### Émotions manga (8) au lieu d'Ekman (6)

Les émotions Ekman sont trop grossières pour le manga. `surprise` est sorti 11/31 fois (35%) — c'est un label fourre-tout pour toute page visuellement intense.

**Remplacement :** `joy, sadness, anger, fear, determination, shock, nostalgia, neutral`

- `determination` capture le shonen core (pages 9-11, 18-23) — signal manquant pour distinguer vrai climax vs tension intense
- `nostalgia` est un meilleur proxy pour flashback que la détection visuelle
- `shock` est plus actionnable que `surprise` — révélation vs "quelque chose se passe vite"

### GUIDED_V3 obligatoire dans le prompt hybride

Le prompt GUIDED_V3 (step-by-step + key distinctions) est ce qui fait passer le modèle de ~50% à 65%. Le retirer du prompt hybride serait une régression. L'extraction des features est ajoutée APRÈS la classification guidée, pas à la place.

### Application séquentielle des règles (cascading)

Les corrections dépendent des précédentes. Exemple :
- Page 27 corrigée ec → sadness → l'historique devient (26=sadness, 27=sadness, 28=sadness)
- Page 29 : 3+ sadness ✓ → correction possible
- Sans correction page 27 : (26=sadness, 27=ec, 28=sadness) → pas 3+ sadness → page 29 pas corrigée

Les règles DOIVENT être appliquées dans l'ordre des pages, chaque correction mettant à jour l'historique post-correction.

### LLM texte classifieur abandonné

Les approches 4, 5, 6, 7 prouvent que le LLM texte perd l'information visuelle et sur-corrige. La classification vient du VLM (qui voit l'image), les corrections viennent de règles déterministes (qui ne perdent rien).

---

## Phase 1 — Prompt hybride + benchmark ✅ COMPLÉTÉE

### Objectif

Obtenir mood label + features en 1 inférence VLM, sans régression sur la baseline.

### Résultat

**Go/no-go FAIL** — le prompt hybride (Pass 5) dégrade la baseline de 65% → 45%. Le dual-task cause du neutral-spam (16/31).

**Fallback activé** : 2 inférences séparées (GUIDED_V3 + extraction features dédiée avec 8 émotions manga). Implémenté comme `extract_features_manga()` dans inference.rs, testé en Pass 7.

Le `GUIDED_V3_PROMPT` est maintenant une constante `pub(crate)` dans inference.rs, partagée entre `analyze_mood()` et le test benchmark.

### Prompt VLM

```
Analyze this manga page step by step:
1. What are the characters expressing? (faces, posture, gestures)
2. What feeling does the author want the reader to experience?
3. Classify as ONE mood category.

Key distinctions:
- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness,
  triumph/determination = emotional_climax
- tension vs epic_battle: anxious anticipation = tension,
  active combat = epic_battle
- chase_action: ONLY for active pursuit/escape

Categories: epic_battle, tension, sadness, comedy, romance, horror,
peaceful, emotional_climax, mystery, chase_action

Then extract these features:
EMOTION: (joy, sadness, anger, fear, determination, shock, nostalgia, neutral)
INTENSITY: (1-10)
NARRATIVE: (present, flashback, dream, thought)
ATMOSPHERE: 2-3 words
CONTENT: 1 sentence

Reply format:
MOOD: [category]
EMOTION: ...
INTENSITY: ...
NARRATIVE: ...
ATMOSPHERE: ...
CONTENT: ...
```

Le VLM fait son raisonnement dans les `<think>` tags, output le mood EN PREMIER (bénéficie du raisonnement guidé), puis remplit les features mécaniquement.

### Parsing hybride

**Format attendu :**
```
<think>...reasoning...</think>
MOOD: sadness
EMOTION: nostalgia
INTENSITY: 7
NARRATIVE: flashback
ATMOSPHERE: dark, nostalgic
CONTENT: Character remembers past victories while feeling defeated
```

**Robustesse :**
- Strip `<think>...</think>` tags avant parsing
- Regex par champ : `(?i)MOOD:\s*(.+)`, `(?i)EMOTION:\s*(.+)`, etc.
- Fallback par champ si manquant ou invalide

**Normalisation des émotions hors-liste :**

Le VLM peut écrire des synonymes au lieu des 8 émotions canoniques. Table de normalisation :

```
excitement, triumph, happiness, relief       → joy
despair, grief, regret, loneliness           → sadness
rage, fury, frustration                      → anger
anxiety, dread, panic, terror                → fear
resolve, willpower, ambition                 → determination
awe, disbelief, astonishment                 → shock
longing, wistful, bittersweet                → nostalgia
calm, indifferent, blank                     → neutral
```

Si l'émotion est vraiment inconnue → fallback `neutral`.

**Fallback MOOD manquant → proxy depuis émotion :**

```
joy → comedy            sadness → sadness
anger → tension          fear → horror
determination → epic_battle   shock → tension
nostalgia → sadness       neutral → peaceful
```

Cas rare (le VLM suit généralement le format).

### Implémentation

```rust
pub struct HybridResult {
    pub mood: MoodCategory,
    pub features: PageFeatures,
}

pub struct PageFeatures {
    pub emotion: String,      // 8 émotions manga normalisées
    pub intensity: u8,        // 1-10
    pub narrative: String,    // present, flashback, dream, thought
    pub atmosphere: String,   // 2-3 mots
    pub content: String,      // 1 phrase
}

// inference.rs
pub async fn extract_hybrid(&self, image_base64: &str) -> Result<HybridResult, String>
pub(crate) fn parse_hybrid_response(content: &str) -> Result<HybridResult, String>
fn normalize_emotion(raw: &str) -> String
```

### Triple critère go/no-go

Run sur les 31 pages Blue Lock. Trois critères à valider :

| # | Critère | Seuil | Si FAIL |
|---|---------|-------|---------|
| 1 | mood label strict accuracy | ≥ 60% | Fallback 2 inférences séparées (GUIDED_V3 + extraction émotions manga) |
| 2 | concentration max d'une émotion | ≤ 8/31 (~26%) | Phase 2 ignore l'émotion, utilise narrative + intensity |
| 3 | pages narrative ≠ present | ≥ 3/31 | Phase 2 ignore le narrative, utilise émotion + intensity |
| — | Si 2 ET 3 fail simultanément | — | Features trop bruitées → fallback 2 inférences séparées |

### Livrable

- `extract_hybrid()` + `parse_hybrid_response()` + `normalize_emotion()` dans inference.rs
- Pass 5 dans le test `bluelock_sequence`
- Cache des résultats dans `manga-mood-ai/results/pass5_hybrid_*.json`

---

## Phase 2 — Règles de fusion déterministes ✅ COMPLÉTÉE

### Objectif

Corriger les faux positifs identifiés par le benchmark Phase 1. Les règles sont conçues APRÈS le benchmark, basées sur les erreurs réelles observées.

### Résultat

Fusion implémentée dans `fuse_mood()` (director.rs). Testée en Pass 6 (sur hybrid) et Pass 7 (sur 2 inférences séparées).

- **Pass 6** (hybrid base 45% + fusion) : 18/31 (58%), +4 corrections exactes, 28/31 relaxed (90%)
- **Pass 7** (baseline 65% + fusion) : 21/31 (68%), +1 correction, 26/31 relaxed (84%)

La fusion marche — le bottleneck est la qualité des features, pas les règles.

### Règle pré-validée — Anti-faux-positif emotional_climax en sadness-arc

```
Pour chaque page (dans l'ordre, cascading) :
  Si mood_label == emotional_climax
  ET 3+ pages précédentes (historique post-correction) == sadness
  ET (emotion ≠ joy avec intensity ≥ 8
      OU narrative ∈ {flashback, dream}) :
    → override sadness
    → mettre à jour historique post-correction
```

Le OR combine deux signaux indépendants :
- **Émotion** : dans un arc de tristesse, tout sauf de la joie intense = faux positif
- **Narrative** : un flashback/rêve dans un arc de tristesse = sadness, même si l'émotion est joy(8+)

Le seul cas qui résiste à l'override : `joy(≥8) + narrative=present` — un vrai climax joyeux dans le présent.

**Tracé sur les données connues :**

| Page | Baseline | emotion | narrative | Règle trigger ? | Résultat |
|------|----------|---------|-----------|-----------------|----------|
| 27 | ec | neutral(1) | present | neutral ≠ joy(≥8) → OUI | → sadness ✓ |
| 28 | ? | joy(8) | flashback | joy(8) = joy(≥8) → NON, MAIS flashback → OUI (OR) | → sadness ✓ |
| 29 | ec | joy(10) | present | joy(10) = joy(≥8) → NON, present → NON | → garde ec |
| 31 | ec | fear(9) | present | fear ≠ joy(≥8) → OUI | → sadness ✓ |

Note : page 29 reste ec avec cette règle. Si c'est un problème, le prompt hybride avec émotions manga pourrait changer la donne (page 29 = rêve de victoire → `nostalgia` au lieu de `joy` → la règle triggerait).

```
// NOTE: rule requires ≥3 existing sadness pages as "anchor" in the
// post-correction history. If the VLM puts ec on ALL pages of a
// sadness arc (no seed), the cascade never starts. This is theoretical
// — data shows the VLM correctly detects the "pure" sadness pages
// (24-26) that serve as anchors.
```

### Règles additionnelles

Conçues APRÈS le benchmark Phase 1, basées sur les erreurs réelles. Exemples de patterns possibles :
- Anti-faux-positif ec en tension-arc
- Anti-faux-positif ec en action-arc (si les données le justifient)
- Correction narrative-aware pour d'autres catégories

### Structure du code

```rust
// director.rs

/// Apply deterministic fusion rules sequentially (cascading corrections).
/// Each correction updates the post-correction history for subsequent pages.
pub fn fuse_mood(
    pages: &[(u32, MoodCategory, &PageFeatures)],
) -> Vec<(u32, MoodCategory)> {
    let mut history: Vec<MoodCategory> = Vec::new();
    let mut results = Vec::new();
    for &(page_num, mood, features) in pages {
        let fused = apply_rules(mood, features, &history);
        history.push(fused);
        results.push((page_num, fused));
    }
    results
}

// TODO: replace hardcoded category names ("emotional_climax", "sadness")
// with property checks (is_climax, is_sustained) when custom categories
// are supported (Phase 3).

fn rule_anti_ec_sadness_arc(
    mood: MoodCategory,
    features: &PageFeatures,
    history: &[MoodCategory],
) -> Option<MoodCategory> { ... }
```

Fonctions séparées par type de règle pour faciliter le refactoring futur.

### Livrable

- `fuse_mood()` + règles individuelles dans director.rs
- Pass 6 dans le test `bluelock_sequence`

---

## Phase 3 — Outlier rule + catégories flexibles (si Phase 2 < 80%)

### Outlier isolé avec feature consistency check

```
Si page N-2 est isolée :
  mood(N-2) ≠ mood(N-3) ET mood(N-2) ≠ mood(N-1)
  ET mood(N-3) == mood(N-1) (majorité claire) :

  Vérifier cohérence features de N-2 :
    Si features contredisent le mood (ex: mood=comedy, emotion=sadness)
      → écraser vers le mood majoritaire
    Si features supportent le mood (ex: mood=comedy, emotion=joy(8+))
      → garder (changement de scène légitime)
```

La contrainte de cohérence features évite d'écraser des changements de scène légitimes (1 page peaceful entre 2 pages tension = le personnage rentre chez lui).

### Catégories flexibles

Refactorer les noms hardcodés vers des propriétés sur `MoodCategoryDef` :

```rust
pub struct MoodCategoryDef {
    pub name: String,
    pub description: String,
    pub is_climax: bool,      // emotional_climax, boss_fight_theme, etc.
    pub is_sustained: bool,   // sadness, tension, etc.
    pub is_action: bool,      // epic_battle, chase_action, tension, etc.
}
```

Les règles de fusion référencent les propriétés (`is_climax`) au lieu des noms (`emotional_climax`). L'utilisateur peut créer des catégories custom et les tagger.

### Enrichissement du prompt (si nécessaire)

- Indices visuels manga pour narrative : "wavy borders, white backgrounds, soft focus = flashback/dream"
- OCR des bulles comme 6ème champ `DIALOGUE`
- Sous-émotions : `triumph`, `regret`, `excitement` si les 8 ne suffisent pas

---

## Phase 4 — Validation élargie

Tester sur 2-3 séquences manga d'autres genres :
- Slice-of-life (transitions peaceful → comedy → romance)
- Seinen dark (transitions horror → mystery → tension)
- Autre shonen (confirmer la généralisation des règles de climax)

---

## Intégration dans le pipeline de production

### Endpoints HTTP (server.rs)

Les endpoints existants (v1) restent pour la rétrocompatibilité :
- `POST /api/analyze` — baseline (1 mood label)
- `GET /api/status` — état du serveur
- `GET /api/moods` — liste des catégories

Nouveaux endpoints (v2) :
- `POST /api/extract` — image → PageFeatures (extraction seule)
- `POST /api/analyze-v2` — image → HybridResult + fusion rules (pipeline complet)
- `POST /api/classify-batch` — PageFeatures[] + catégories → moods[] (batch, pour extension navigateur)

### Scénarios d'usage

**Batch (chapitre complet) :**
```
Extension envoie N images → extract_hybrid() par page
→ fuse_mood() sur le lot → MoodDirector → mood par page
```

**Incrémental (scroll) :**
```
Nouvelle page → extract_hybrid() → ajouter au buffer
→ fuse_mood() sur toutes les pages accumulées → MoodDirector
```

### Buffer (dans AppState)

```rust
pub struct DescriptionBuffer {
    pub pages: BTreeMap<u32, HybridResult>,
    pub fused_moods: BTreeMap<u32, MoodCategory>,
}
```

---

## Risques et mitigations

| Risque | Mitigation |
|--------|------------|
| Le prompt hybride dégrade le mood label (trop de champs distraient le VLM) | Triple go/no-go. Fallback 2 inférences séparées si <60%. |
| Les émotions manga sont aussi bruitées que Ekman (`determination` partout au lieu de `surprise` partout) | Critère 2 du go/no-go : max 8/31 sur une émotion. |
| Le narrative est toujours `present` | Critère 3 du go/no-go : ≥3 pages non-present. |
| La cascade n'a pas de seed sadness (VLM rate toutes les pages de l'arc) | Théorique — les données montrent que les pages "pures" (24-26) sont bien détectées. Comment dans le code. |
| L'outlier rule écrase des changements de scène légitimes | Feature consistency check (Phase 3). |
| Les règles sont hardcodées pour les catégories par défaut | TODOs dans le code. Property tags en Phase 3. |

---

## Métriques de succès

| Métrique | Baseline (dim.) | Objectif initial | V6 (meilleur) | Status |
|----------|-----------------|------------------|---------------|--------|
| Strict accuracy | 45% (14/31) | ≥ 80% (25/31) | **71% (22/31)** | ❌ Non atteint |
| Relaxed accuracy | 74% (23/31) | ≥ 92% (29/31) | **84% (26/31)** | ❌ Non atteint |
| Gain vs baseline | — | — | **+26 points strict** | ✅ Amélioration majeure |
| Coût par page | ~5s (1 inférence) | ~3-4s | ~8-12s (2 inférences) | ⚠️ 2x plus lent |

**Conclusion :** Avec le pipeline V6 seul, l'objectif 80%/92% n'etait pas atteignable. Cette limite a ensuite ete depassee par V12 multi-image sur le 31 pages Blue Lock, et la reference RealTest du repo est desormais le protocole historique `Qwen3-VL-4B-Thinking` reproduit a 84.3% relaxed sur `BL/1`.

---

## Commande de test

```bash
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```
