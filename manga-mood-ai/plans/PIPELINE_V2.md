# Pipeline V2 — "Describe then Classify"

> **Statut : EVOLUE → Pipeline V6.** L'idee originale (VLM describe → LLM text classify batch) a donne 19% strict (Pass 3). L'evolution (VLM describe → VLM classify with context) a donne **71% strict (V6)**. Voir la section "Resultats reels" en bas.
>
> **Mise a jour (mars 2026) :** ce document est historique. Les references actuelles sont :
> - benchmark de reference: [manga-mood-ai/research/RESEARCH_SYNTHESIS.md](/home/mehdi/Dev/KeyToMusicRustTauri/manga-mood-ai/research/RESEARCH_SYNTHESIS.md)
> - architecture produit active: [docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md)

## Probleme

L'architecture actuelle (1 VLM call par page → classification directe) plafonne a **65% strict / 81% relaxed** sur une sequence de 31 pages Blue Lock.

**Cause racine :** On demande au VLM de faire un **jugement narratif** (quel mood pour la soundtrack ?) a partir d'une **seule observation visuelle** (une page). Le modele confond l'intensite emotionnelle avec la categorie narrative — une page triste mais intense → `emotional_climax` au lieu de `sadness`.

Les approches testees et echouees :
- Injection de contexte dans le prompt VLM → feedback loops (40% strict)
- Scored prompt (10 floats) → le modele ne sait pas scorer (26% strict)
- Refinement texte des labels → le LLM texte n'a pas de raison de douter (65% strict, identique)

## Insight

Le probleme n'est pas le modele. C'est qu'on lui demande de faire **deux choses en meme temps** :
1. **Percevoir** : comprendre ce qui se passe visuellement sur la page
2. **Juger** : decider quel mood correspond pour la soundtrack

Le VLM est bon pour (1) mais mauvais pour (2) sans contexte narratif.
Un LLM texte serait bon pour (2) s'il avait les descriptions de plusieurs pages.

**Solution : separer perception et jugement.**

---

## Architecture proposee

```
Extension navigateur charge le chapitre (ou une partie via lazy loading)
  → Envoie N images au backend KeyToMusic

Etage 1 — VLM Describe (1 inference par page)
  → Input : image de la page
  → Output : description textuelle (2-3 phrases)
  → Le VLM decrit ce qu'il VOIT, sans classifier

Etage 2 — LLM Texte Classify (1 inference pour le batch)
  → Input : N descriptions ordonnees
  → Output : N mood labels
  → Le LLM voit TOUTE la sequence narrative d'un coup

Etage 3 — MoodDirector (inchange)
  → Smoothing, dwell, transitions OST
```

### Pourquoi c'est fondamentalement different

Architecture actuelle :
```
Image → VLM → mood label (souvent faux) → MoodDirector
```

Approches echouees :
```
Image → VLM → mood label (souvent faux) → LLM texte corrige des labels faux → toujours faux
```

Pipeline V2 :
```
Image → VLM → description visuelle (fiable) → LLM texte + contexte narratif → mood label → MoodDirector
```

Chaque composant fait ce qu'il fait **bien** :
- VLM = decrire des images (son coeur de metier)
- LLM texte = raisonner sur une sequence narrative (son coeur de metier)

---

## Etage 1 — VLM Describe

### Prompt

```
Describe this manga page in 2-3 sentences:
- What is happening? (action, dialogue, flashback)
- Character expressions and body language
- Visual atmosphere (shading, effects, panel layout)
```

### Output attendu

```
"A character is crying alone in the rain. Dark heavy shading, close-up on tears.
No action, no other characters. Somber, isolated atmosphere."
```

Pas de classification, pas de mood. Juste de la description factuelle. Le VLM est **bon** pour ca — c'est exactement ce pour quoi les VLM sont entraines.

### Avantages

- Elimine le probleme de confusion entre categories
- La description est fiable meme quand la classification serait fausse
- Le modele n'a pas besoin de connaitre nos 10 categories
- Prompt plus simple = reponse plus rapide et plus fiable

### Cout

- 1 inference VLM par page (identique a maintenant)
- ~1-2s par page (potentiellement plus rapide car prompt plus simple)
- Output plus long (~50-100 tokens vs ~1 token) mais negligeable

---

## Etage 2 — LLM Texte Classify

### Prompt

```
You are a manga soundtrack director. Based on these page descriptions,
assign a mood to each page for soundtrack purposes.

Page 6: "Two characters face each other in a dimly lit room. Tense body language,
clenched fists. Speech bubbles with bold text. Dramatic close-ups."
Page 7: "Same scene continues. One character points aggressively. The other looks
down with narrowed eyes. Heavy diagonal speed lines."
...
Page 27: "Character crying intensely, being held by another character. Tears
streaming down. Soft shading, emotional embrace."
...

Categories: epic_battle, tension, sadness, comedy, romance, horror,
peaceful (calm daily life), emotional_climax (narrative turning point),
mystery, chase_action

Key: emotional_climax = a TURNING POINT or revelation, not just intense emotion.
Sadness remains sadness even when very intense.

For each page, output: PAGE N: mood
```

### Pourquoi ca devrait marcher

Le LLM texte voit **la sequence entiere**. Il voit que les pages 24-32 sont toutes des scenes de larmes/tristesse → `sadness` partout, pas `emotional_climax` isole. Il peut distinguer parce que :
- Il voit le pattern "crying alone → crying with friend → sitting silently" = arc de tristesse
- Il voit que le vrai tournant (page 34 : ambiance change vers peaceful) est le seul `emotional_climax` de cette zone
- Il raisonne sur la **narration**, pas sur l'**intensite visuelle** d'une page isolee

### Cout

- 1 seule inference pour le batch entier (texte pur, pas d'image)
- ~1-3s pour 30 pages (les descriptions font ~50-100 tokens chacune = ~2000-3000 tokens input)
- Utilise le meme modele llama-server (Qwen3.5 4B en mode texte, sans mmproj)

---

## Contexte de l'extension navigateur

L'extension tourne sur des sites de lecture manga avec **disposition verticale** (scroll continu). Elle recupere les images de la page — parfois tout le chapitre d'un coup, parfois incrementalement au scroll.

### Scenario 1 : Chapitre entier disponible

```
Extension detecte N images du chapitre
  → POST /api/analyze-batch (N images)
  → Backend:
    1. VLM describe les N pages (sequentiel ou parallele)
    2. LLM texte classify le batch (1 inference)
    3. MoodDirector smooth les transitions
  → Response: mood par page
  → L'extension signal le mood au fur et a mesure du scroll
```

Avantage : le LLM texte a une vision **omnisciente** du chapitre.

### Scenario 2 : Pages chargees incrementalement

```
Extension envoie les pages au fur et a mesure du scroll
  → Backend accumule les descriptions dans un buffer
  → Apres chaque nouvelle page:
    1. VLM describe la nouvelle page
    2. LLM texte re-classify avec toutes les descriptions accumulees
    3. Peut reviser retroactivement les moods des pages precedentes
```

Avantage : fonctionne en streaming. Le cout du LLM texte est negligeable (texte pur).

### Scenario 3 : Hybride (le plus probable)

```
Extension detecte les images deja chargees (ex: 10 premieres pages)
  → Batch initial : VLM describe 10 + LLM classify 10
Au scroll, nouvelles pages chargees :
  → VLM describe incrementalement
  → LLM re-classify periodiquement (toutes les 3-5 nouvelles pages)
  → Revision retroactive si le contexte elargi change l'interpretation
```

---

## Latence estimee

| Operation | Temps | Frequence |
|-----------|-------|-----------|
| VLM describe (par page) | ~1-2s | A chaque nouvelle page |
| LLM texte classify (batch) | ~1-3s | Apres chaque batch (3-5 pages) |
| MoodDirector | <1ms | A chaque page |

**Temps total par page :** ~2-3s (vs ~5.7s actuellement avec le 4B).
Potentiellement plus rapide parce que le prompt "describe" est plus simple que le prompt "classify".

Pour le temps reel : la page precedente est deja decrite quand l'utilisateur arrive a la suivante. Seule la description de la page courante + le re-classify sont sur le chemin critique.

---

## Ce qu'on peut tester

Sur nos 31 images Blue Lock :

1. Prompt "describe" sur chaque page → 31 descriptions (31 inferences VLM)
2. Descriptions dans un seul prompt LLM texte → 31 moods (1 inference texte)
3. Comparer avec le baseline (65%/81%) et le ground truth

### Commande de test

```bash
cargo test --manifest-path src-tauri/Cargo.toml bluelock_describe_classify -- --ignored --nocapture
```

### Score attendu

Si les descriptions sont bonnes (le VLM est fort pour decrire) et que le LLM texte raisonne bien sur la sequence, on devrait voir **80-90% strict** parce qu'on attaque le probleme a la racine : le LLM texte ne va pas confondre "personnage qui pleure intensement" avec un climax narratif quand il voit que les 8 pages autour decrivent toutes des scenes de tristesse.

---

## Implementation

### Nouveaux endpoints

- `POST /api/describe` : image → description textuelle (VLM, etage 1)
- `POST /api/classify-batch` : descriptions[] → moods[] (LLM texte, etage 2)
- `POST /api/analyze-v2` : images[] → moods[] (pipeline complet, etage 1+2+3)

### Nouvelles methodes dans `LlamaServer`

```rust
/// Etage 1: VLM describe (1 image → description textuelle)
pub async fn describe_page(&self, image_base64: &str) -> Result<String, String>

/// Etage 2: LLM texte classify (N descriptions → N moods)
pub async fn classify_batch(&self, descriptions: &[(u32, &str)]) -> Result<Vec<(u32, MoodCategory)>, String>
```

### Buffer de descriptions

```rust
/// Accumule les descriptions pour le batch classify
pub struct DescriptionBuffer {
    pages: BTreeMap<u32, String>,  // page_num → description
    last_classify_at: u32,         // derniere page ou on a fait un classify
}
```

### Pas de changement au MoodDirector

L'etage 3 reste identique. Le MoodDirector recoit des moods (quelle que soit leur origine) et fait le smoothing/transitions.

---

## Resultats reels

### Pipeline V2 original (describe → text-only classify)

**Resultat : 6/31 strict (19%), 19/31 relaxed (61%) — ECHEC**

L'idee etait bonne mais l'implementation a un defaut fatal : le LLM texte **ne voit pas les images**. Il classe a partir de descriptions qui perdent la specificite emotionnelle. "Intense soccer moment" est ambigu — tension, epic, ou climax ?

### Evolution : Pipeline V6 (describe → VLM classify with context)

L'insight cle : au lieu de faire classifier par un LLM texte sans images, **le VLM lui-meme reclassifie en voyant l'image + les descriptions precedentes**.

```
Architecture V6 (ce qui marche) :

Pour chaque page N :
  1. describe_page(image_N) → description factuelle (~500-800 tokens)
  2. classify_with_context(image_N, descriptions[N-4..N-1]) → mood + intensite
     Le VLM voit : l'image + 4 descriptions precedentes + prompt de classification
```

**Resultat V6 : 22/31 strict (71%), 26/31 relaxed (84%) — MEILLEUR RESULTAT**

**Pourquoi V6 marche et V2 original non :**
- V2 : LLM texte classifie SANS image → perd l'info visuelle → 19%
- V6 : VLM classifie AVEC image + contexte textuel → garde les deux → 71%

L'etage 1 (describe) est preserv tel quel. C'est l'etage 2 qui a change radicalement : au lieu d'un batch text-only, c'est une classification VLM par page avec contexte.

### Methodes implementees (inference.rs)

```rust
/// Etage 1: VLM describe (inchange)
pub async fn describe_page(&self, image_base64: &str) -> Result<String, String>

/// Etage 2 V6: VLM classify avec image + contexte
pub async fn classify_with_context(
    &self,
    image_base64: &str,
    previous_descriptions: &[(u32, &str)],  // 4 pages passees
    next_descriptions: &[(u32, &str)],       // vide en V6
    narrative_arc: &[(u32, &str)],           // vide en V6
) -> Result<MoodTag, String>
```

### Exploration des variantes de contexte

| Variante | Contexte | Strict | Relaxed |
|----------|----------|--------|---------|
| V6 | 4 passe full | **22/31 (71%)** | 26/31 (84%) |
| V7 | 2+2 bidirectionnel | 21/31 (68%) | 27/31 (87%) |
| V8 | 4+2 asymetrique | 19/31 (61%) | 27/31 (87%) |
| V9 | 3+3 symetrique | 22/31 (71%) | 26/31 (84%) |
| V10 | 2 full + 5 first-sentence | 16/31 (52%) | 24/31 (77%) |
| V11 | 2 full + resume LLM | 19/31 (61%) | 25/31 (81%) |

**Conclusion :** V6 (4 descriptions passees, pas de futur, pas de resumes) est le meilleur resultat de cette famille de pipelines contextuels. Cette conclusion a ensuite ete depassee par V12 multi-image, et le benchmark RealTest par defaut du repo rejoue maintenant le protocole historique `Qwen3-VL-4B-Thinking`.
