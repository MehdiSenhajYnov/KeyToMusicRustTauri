# Manga Mood AI — Benchmark Findings

Comprehensive results from benchmarking local VLM models and prompts for manga page mood detection. The goal: classify manga pages into one of 10 mood categories with maximum accuracy using a local model that fits in 12GB VRAM.

**Phase 1 test set:** 13-18 isolated manga pages covering all 10 mood categories.
**Phase 2 test set:** 31 consecutive pages from Blue Lock Tome 1 (ch.1, pages 6-36) — tests narrative coherence, not just per-page accuracy.

> Status update (March 2026): this file remains a historical write-up for earlier isolated-image and sequence benchmarks. It is not the canonical product spec. See [RESEARCH_SYNTHESIS.md](./RESEARCH_SYNTHESIS.md) for the benchmark overview and [../../docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md](/home/mehdi/Dev/KeyToMusicRustTauri/docs/MANGA_MOOD_CURRENT_ARCHITECTURE.md) for the active product workflow.

---

## 1. Models Tested

| Model | Score | Time/image | VRAM | Verdict |
|-------|-------|------------|------|---------|
| **Qwen3-VL 2B** (thinking) | **100%** (18/18) | ~2.8s | 38% (~4.7GB) | **Best isolated-image result in this historical phase** — thinking model beats larger models |
| Qwen2.5-VL 7B | 89% (16/18) | ~1.3s | 96% | Faster but less accurate, barely fits in VRAM |
| InternVL3.5 4B | 69% | ~0.5s | 74% | Fast but imprecise |
| Gemma 3 4B | 44% | - | - | Spams "tension" on everything |
| Gemma 3n E2B/E4B | 28% | - | - | Spams "tension" even worse |
| Kimi-VL A3B | - | 30-60s | 100%+ | 18GB VRAM needed, doesn't fit in 12GB |

**Key lesson:** A 2B thinking model beats 4-7B models thanks to `<think>` reasoning. The model "thinks out loud" before classifying, which dramatically improves accuracy on ambiguous pages.

---

## 2. Prompt Evolution

| Prompt | Score | Problem |
|--------|-------|---------|
| SHORT (baseline) | 94% (67% strict) | No guidance, misses 16.png (sadness → mystery) |
| VISUAL (+ visual cues) | 94% | Same miss on 16.png persists |
| NARRATIVE (reader intention) | 78% | Too vague, misses many pages |
| COMBINED (narrative + visual + distinctions) | 67% | Too long → timeouts, model gets confused |
| TWO-PASS (hierarchical) | 61-78% | Pass 1 errors cascade irrecoverably |
| GUIDED (step-by-step + distinctions) | 94% | Fixes 16.png but misses 18.png (peaceful) |
| GUIDED_V2 (+ category descriptions) | 89% | Descriptions cause timeout on 4.jpg |
| **GUIDED_V3** (guided + minimal peaceful hint) | **100%** (18/18) | No misses |

---

## 3. Lessons Learned (Pitfalls to Avoid)

### 3.1 Never say "X can also resemble Y"
The model will see Y everywhere. Example: adding "sadness can have bold text" to VISUAL prompt made everything classify as sadness.

### 3.2 Don't list concrete actions in broad categories
"running" can be chase_action (pursuit), sadness (flashback), or peaceful (jogging). Keep category hints abstract.

### 3.3 Too many descriptions = model loops
The COMBINED prompt with full descriptions for all 10 categories caused 30-60s timeouts. The model enters infinite thinking loops when overwhelmed with text.

### 3.4 Prompt format breaks some models
Multi-line `- category: description` lists cause Gemma models to spam "tension". Format matters as much as content.

### 3.5 Multi-image context makes results worse
Sending previous pages as context (context=1) dropped accuracy from 65% to 35%. The 2B model spams "emotional_climax" when given multiple images. Single image per request is optimal.

### 3.6 Two-pass classification is a dead end (for labels)
Regardless of model size (2B or 7B), errors in pass 1 (e.g., "is this action or dialogue?") cascade irrecoverably into pass 2. Single-pass classification is more robust. **However:** two-pass with different modalities (VLM describe → LLM classify) remains promising — see PIPELINE_V2.md.

### 3.7 Temperature 0 eliminates noise
Random comedy classifications on 18.png (peaceful scene) disappeared with temperature=0. No benefit to any randomness for this task.

### 3.8 A single minimal hint is enough
Adding just "(calm daily life)" after "peaceful" in the category list fixed 18.png without side effects. Full descriptions for every category are counterproductive (see 3.3).

---

## 4. Best Isolated-Image Configuration (Historical Phase 1)

```
Model:        Qwen3-VL 2B (qwen3-vl:2b, Q4_K_M GGUF)
Resize:       672px max dimension (Lanczos3, JPEG encoding)
Temperature:  0.0
Max tokens:   2048 (needed for <think> reasoning)
Context:      2048 tokens
Prompt:       GUIDED_V3 (see below)
Score:        18/18 (100%)
Avg time:     ~2.8s/image
VRAM:         38% (~4.7 GB on 12GB GPU)
```

---

## 5. Best Historical Isolated-Image Prompt (GUIDED_V3)

```
Analyze this manga page step by step:
1. What are the characters expressing? (faces, posture, gestures)
2. What feeling does the author want the reader to experience?
3. Classify as ONE of the categories below.

Key distinctions:
- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness, triumph/determination = emotional_climax
- tension vs epic_battle: anxious anticipation = tension, active combat = epic_battle
- chase_action: ONLY for active pursuit/escape, not flashbacks with movement

Categories: epic_battle, tension, sadness, comedy, romance, horror, peaceful (calm daily life), emotional_climax, mystery, chase_action

Reply with ONLY the category name after your reasoning.
```

### Why it works

1. **Step-by-step structure** forces the thinking model to analyze visual elements before classifying. The `<think>` block contains actual reasoning about faces, posture, and narrative intent.

2. **Three key distinctions** resolve the most frequent confusions:
   - sadness vs emotional_climax (sorrow vs triumph)
   - tension vs epic_battle (anticipation vs active combat)
   - chase_action scope (only active pursuit, not flashback movement)

3. **Categories as bare names** (no descriptions) prevent the model from latching onto specific keywords. The only exception is the minimal hint "(calm daily life)" for peaceful, which resolved the last remaining misclassification.

4. **"Reply with ONLY the category name after your reasoning"** tells the model it can think freely in `<think>` tags but must output a clean category name at the end.

### Response format

The model outputs:
```
<think>
[Internal reasoning about the page — faces, mood, narrative intent, category selection]
</think>
category_name
```

The `</think>` tag is stripped in `parse_mood_response()` and the remaining text is matched against the 10 category names.

---

## 6. Phase 2 — Blue Lock Sequence Benchmark (31 pages, Rust integration test)

After Phase 1 established the best isolated-image model and prompt for that historical phase, Phase 2 tested on a real consecutive manga chapter to evaluate narrative coherence.

### 6.1 Model upgrade: Qwen3-VL 2B → Qwen3.5-VL 4B

The Qwen3.5-VL 4B (Q4_K_M GGUF, ~2.5GB) was tested as a potential upgrade. Both models use the same GUIDED_V3 prompt via llama-server (not Ollama).

| Model | Strict | Relaxed | Avg time | Notes |
|-------|--------|---------|----------|-------|
| Qwen3-VL 2B (Phase 1, 13 imgs) | 100% | 100% | ~1.1s | Best isolated-image result in that phase |
| Qwen3.5-VL 4B (31-page sequence) | 65% (20/31) | 81% (25/31) | ~5.7s | More capable but over-predicts emotional_climax |

The 4B model is better at understanding complex manga scenes but has a specific bias: it confuses **intense emotion** with **emotional_climax**. A page showing a character crying intensely → model sees "strong emotion" → outputs `emotional_climax` instead of `sadness`.

### 6.2 Error pattern: emotional_climax over-prediction

All 6 FAIL cases on the 31-page sequence follow the same pattern:

| Page | Expected | Detected | What's on the page |
|------|----------|----------|-------------------|
| 14 | tension | emotional_climax | Intense close-up, dramatic framing |
| 16 | tension | emotional_climax | Dramatic character expression |
| 27 | sadness | emotional_climax | Character crying intensely |
| 29 | sadness | emotional_climax | Emotional scene with tears |
| 31 | sadness | emotional_climax | Intense emotional moment |
| 36 | mystery | chase_action | Ambiguous closing page |

**Root cause:** The VLM classifies based on **visual intensity**, not **narrative function**. `emotional_climax` = narrative turning point, but the model interprets it as "high emotional intensity". This is a fundamental limitation of per-page classification without narrative context.

### 6.3 Approaches tested to improve beyond 65%/81%

#### Approach A: Context injection in VLM prompt

Enriched the VLM prompt with narrative context (previous moods, current soundtrack, dwell count, look-ahead from cache).

**Result: DEGRADED — 40% strict, 60% relaxed (vs baseline 65%/81%)**

Problem: **positive feedback loop**. Once the model misdetected one page as `emotional_climax`, the context for subsequent pages reinforced this error, creating cascading failures. The model saw "previous mood: emotional_climax" and was biased to continue predicting it.

**Code:** `NarrativeContext` struct in `inference.rs`, integrated in `server.rs` and `commands.rs`. Still in codebase but not used in production pipeline.

#### Approach B: Two-pass with scored prompt (10 floats)

Pass 1: VLM outputs 10 mood scores (0.0-1.0) per page. Pass 2: text-only LLM refines scores with narrative context.

**Result: CATASTROPHIC — scored prompt gives 26% strict, 55% relaxed**

The 4B model simply cannot reliably output 10 floating-point scores. The single-label prompt (65% strict) is 2.5x more accurate than the scored prompt (26% strict). Pass 2 refinement couldn't fix garbage input → 19% strict.

**Lesson:** Small models are classification engines, not regression engines. Don't ask for continuous outputs.

#### Approach C: Two-pass with single-label + text-only refinement

Pass 1: VLM single-label classification (baseline). Pass 2: text-only LLM receives all 31 labels, asked to refine based on narrative coherence.

**Result: NO CHANGE — 65% strict, 81% relaxed (identical to baseline)**

The text-only LLM simply echoed back the labels. Without seeing the images, it had no reason to doubt the VLM's classifications. A sequence like `tension → emotional_climax → tension` is narratively plausible, so the LLM validated it.

**Lesson:** Text-only refinement of labels can't improve accuracy because the labels are the only information available, and they look plausible even when wrong.

#### Approach D: Pipeline V2 — Describe then Classify (Pass 3)

VLM describes each page in 2-3 sentences, then LLM text classifies all 31 descriptions in 1 batch inference.

**Result: DEGRADED — 6/31 strict (19%), 19/31 relaxed (61%)**

Problem: descriptions lose emotional specificity. "Intense soccer moment" is ambiguous — could be tension, epic_battle, or emotional_climax. The LLM text over-uses `epic_battle` for any action scene.

#### Approach E: Pipeline V3 — Structured Extract + Classify (Pass 4)

VLM extracts structured features (emotion, intensity, narrative, atmosphere, content) per page, then LLM text classifies from features in batch.

**Result: 13/31 strict (42%), 26/31 relaxed (84%)**

Better than descriptions (84% vs 61% relaxed) but the LLM text classifier loses visual context. The structured features are more informative than free-text descriptions, but classification still degrades vs direct VLM label.

#### Approach F: Hybrid 1-inference (mood + features) (Pass 5)

Single VLM inference asking for GUIDED_V3 mood classification AND feature extraction simultaneously.

**Result: DEGRADED — 14/31 strict (45%), 24/31 relaxed (77%)**

Problem: the VLM loses focus when asked for mood + features in one prompt. Neutral-spam: 16/31 emotion=neutral. The dual-task degrades both outputs — worse mood (45% vs 65%) and worse features (16/31 neutral).

#### Approach G: Hybrid + Deterministic Fusion (Pass 6)

Same as Pass 5, then apply deterministic fusion rules (e.g., emotional_climax → sadness if 3+ prior sadness pages and emotion ≠ joy(≥8)).

**Result: 18/31 strict (58%), 28/31 relaxed (90%)**

Fusion rules work perfectly — all 4 corrections are exact (pages 27, 28, 29, 31). But base is too weak (45% → 58%, still below 65% baseline). The fusion can't compensate for the degraded hybrid base.

#### Approach H: 2 Separate Inferences + Fusion (Pass 7)

Two separate VLM inferences per page: GUIDED_V3 for mood (same as baseline) + dedicated feature extraction (8 manga emotions: joy, sadness, anger, fear, determination, shock, nostalgia, neutral). Then fusion rules applied on baseline mood + dedicated features.

**Result: 21/31 strict (68%), 26/31 relaxed (84%) — +1/+1 from baseline**

The dedicated feature extraction avoids the neutral-spam of Pass 5, but introduces **determination-spam** (16/31). Fusion corrects only 1 page (27) because the features don't produce `nostalgia` where needed for the sadness-arc rule to trigger on pages 29 and 31.

**Key finding:** Separating inferences preserves the baseline mood (65%) and allows modest fusion improvement (+1 strict). But the emotion extraction still biases toward a single dominant emotion (determination instead of neutral), limiting fusion effectiveness.

### 6.4 Summary of all approaches

| # | Approach | Strict | Relaxed | Key issue |
|---|----------|--------|---------|-----------|
| 1 | **Baseline** (GUIDED_V3 single-label) | **20/31 (65%)** | **25/31 (81%)** | Reference |
| A | Context injection in VLM prompt | 12/30 (40%) | 18/30 (60%) | Feedback loops |
| B | Scored prompt (10 floats) | 8/31 (26%) | 17/31 (55%) | Model can't regress |
| — | Scored + text refinement | 6/31 (19%) | 15/31 (48%) | Garbage in garbage out |
| C | Single-label + text refinement | 20/31 (65%) | 25/31 (81%) | LLM echoes labels back |
| D | Pipeline V2 describe→classify | 6/31 (19%) | 19/31 (61%) | Descriptions lose emotion |
| E | Pipeline V3 extract→classify | 13/31 (42%) | 26/31 (84%) | LLM text loses visual info |
| F | Hybrid 1-inference (mood+features) | 14/31 (45%) | 24/31 (77%) | Dual-task degrades both |
| G | Hybrid + fusion | 18/31 (58%) | 28/31 (90%) | Good fusion, weak base |
| **H** | **2 inferences + fusion** | **21/31 (68%)** | **26/31 (84%)** | **Best strict, determination-spam limits fusion** |

### 6.5 Key insight: perception vs judgment

The fundamental problem is that we ask the VLM to make a **narrative judgment** (what mood should the soundtrack be?) from a **single visual observation** (one page). This is like asking someone to identify the climax of a movie from a single frame — impossible without narrative context.

But approaches that inject context (A) create feedback loops, and approaches that refine labels (B, C) can't access the visual information. And all feature extraction approaches (D-H) suffer from emotion-spam: the VLM collapses diverse emotions into 1-2 dominant labels (neutral, determination) regardless of the emotion vocabulary offered.

**Best result within this document's hybrid/fusion track:** Pass 7 (2 inferences + fusion) at 68% strict / 84% relaxed. The fusion rules are proven effective (+4 corrections in Pass 6) but are limited by the quality of extracted features. Improving feature diversity is the key to further gains.

**Open questions (Phase 2):**
- Can a larger model (7B+) produce better emotion diversity?
- Can visual manga cues (wavy borders, white backgrounds) improve narrative detection?
- Would fine-tuning on manga emotion data eliminate the spam problem?

---

## 7. Phase 3 — Dimensional Moods + Context Descriptions Pipeline

After Phase 2 established the ceiling for single-page classification (~68% strict with fusion), Phase 3 explored two fundamental changes: a simplified mood taxonomy and contextual classification using page descriptions.

### 7.1 Dimensional mood system (8 moods × 3 intensity)

Replaced the 10 categorical moods with 8 moods + intensity levels:

**Removed:** `emotional_climax` (over-predicted, confused with visual intensity), `chase_action` (too similar to tension/epic)

**New system:**
- 8 moods: `epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery`
- 3 intensity levels: 1 (low), 2 (medium), 3 (high)
- **Relaxed matching:** mood families (epic↔tension, sadness↔peaceful, comedy↔romance, horror↔mystery)

**Baseline without context:** 14/31 strict (45%), 23/31 relaxed (74%) — lower than the 10-mood baseline (65%) because the model now has to assign intensity, and the mood categories are broader.

### 7.2 The breakthrough: descriptions as context (V6)

**Problem:** Single-page classification has a structural ceiling. The model can't distinguish "intense sadness" from "epic moment" without knowing what happened before.

**Previous failed context attempts:**
- Mood label injection → feedback loops (65% → 40%)
- Text-only correction of labels → LLM echoes labels back (0% improvement)
- Multi-image VLM → model spams emotional_climax (65% → 35%)

**Solution: factual descriptions as context.**

Architecture:
```
For each page N:
  Step 1: describe_page(image_N) → factual text description (no mood)
  Step 2: classify_with_context(image_N, descriptions[N-4..N-1]) → mood + intensity
```

**Why this works and labels don't:**
- Descriptions are factual ("a character crying in the rain") — no mood bias to create feedback loops
- The VLM still sees the image during classification (unlike text-only approaches that got 19%)
- The context helps the VLM understand narrative arc (consecutive crying scenes = sadness, not climax)

**V6 Result: 22/31 strict (71%), 26/31 relaxed (84%) — BEST RESULT IN THIS V6 FAMILY.**

The sadness arc (pages 22-33) is almost entirely fixed. The model now sees 3-4 descriptions of crying/grief scenes before classifying the current page, so it correctly identifies "more sadness" instead of "climax."

### 7.3 Context window exploration (V6–V11)

After V6 proved context works, we systematically tested different window configurations:

| Pass | Context Window | Strict | Relaxed | Notes |
|------|---------------|--------|---------|-------|
| V6 | 4 past, 0 future | **22/31 (71%)** | 26/31 (84%) | **Best result in this V6-V11 family** |
| V7 | 2 past, 2 future | 21/31 (68%) | 27/31 (87%) | Fixes pages 7-8 but regresses 9,17,25 |
| V8 | 4 past, 2 future | 19/31 (61%) | 27/31 (87%) | Too much context overwhelms model |
| V9 | 3 past, 3 future | 22/31 (71%) | 26/31 (84%) | Ties V6 on numbers, different error profile |
| V10 | 2 full + 5 first-sentence | 16/31 (52%) | 24/31 (77%) | First sentences too generic |
| V11 | 2 full + LLM summary | 19/31 (61%) | 25/31 (81%) | LLM summaries add noise |

**Key findings:**

1. **4 past full descriptions is optimal for the 4B model.** Adding more (V8: 6 total) overwhelms it.
2. **Future context doesn't help strict accuracy.** The model tries to "match" the future mood instead of classifying what it sees, causing regressions in the middle of arcs.
3. **Summarized context regresses.** Both first-sentence extraction (V10: 52%) and LLM-generated summaries (V11: 61%) lose the narrative signal that full descriptions provide.
4. **Descriptions are much longer than expected.** Each description is ~500-800 tokens (multi-paragraph), not 2-3 sentences. 4 descriptions = ~2500-3000 tokens of context. The model handles this well with `-c 32768`.

### 7.4 Description discovery

A critical unexpected finding: VLM descriptions of manga pages are extremely verbose (~500-800 tokens each), containing detailed analysis with section headers, character identification, and visual composition notes. Examples from the cached descriptions:

- A simple dialogue page generates 600+ tokens describing panel layouts, character positions, speech bubble sizes, shading techniques, and inferred emotions.
- Action pages generate 800+ tokens with blow-by-blow panel descriptions.

This means 4 descriptions ≈ 2500-3000 tokens of context, which works well with the 32K context window but would be prohibitive at the original 2048 context limit. The switch to `-c 32768` was necessary to enable the V6 pipeline.

### 7.5 Summary of all approaches (Phase 1–3)

| # | Approach | Strict | Relaxed | System |
|---|----------|--------|---------|--------|
| 1 | Baseline (GUIDED_V3 single-label) | 20/31 (65%) | 25/31 (81%) | 10 moods |
| 2 | + text refinement | 20/31 (65%) | 25/31 (81%) | 10 moods |
| 3 | Describe → text-only classify | 6/31 (19%) | 19/31 (61%) | 10 moods |
| 4 | Extract → text-only classify | 13/31 (42%) | 26/31 (84%) | 10 moods |
| 5 | Hybrid 1-inference | 14/31 (45%) | 24/31 (77%) | 10 moods |
| 6 | Hybrid + fusion | 18/31 (58%) | 28/31 (90%) | 10 moods |
| 7 | 2 inferences + fusion | 21/31 (68%) | 26/31 (84%) | 10 moods |
| — | Context injection (mood labels) | 12/30 (40%) | 18/30 (60%) | 10 moods |
| Dim.1 | Dimensional baseline | 14/31 (45%) | 23/31 (74%) | 8 moods ×3 |
| Dim.V5 | Describe + text-only correct | 15/31 (48%) | 24/31 (77%) | 8 moods ×3 |
| **Dim.V6** | **4 past descriptions + VLM classify** | **22/31 (71%)** | **26/31 (84%)** | **8 moods ×3** |
| Dim.V7 | 2+2 bidirectional | 21/31 (68%) | 27/31 (87%) | 8 moods ×3 |
| Dim.V8 | 4+2 asymmetric | 19/31 (61%) | 27/31 (87%) | 8 moods ×3 |
| Dim.V9 | 3+3 symmetric | 22/31 (71%) | 26/31 (84%) | 8 moods ×3 |
| Dim.V10 | 2 full + 5 first-sentence | 16/31 (52%) | 24/31 (77%) | 8 moods ×3 |
| Dim.V11 | 2 full + LLM summary | 19/31 (61%) | 25/31 (81%) | 8 moods ×3 |

### 7.6 Key insight: context > architecture

The single biggest improvement across all experiments came from providing **factual descriptions of previous pages as context** (+26 percentage points strict, 45%→71%). No other technique came close:

- Fusion rules: +3 points (65%→68%)
- 2 inferences: +3 points (65%→68%)
- Dimensional moods: -20 points baseline (65%→45%) but recoverable with context

The lesson: for narrative classification tasks, **context is more important than model size, prompt engineering, or post-processing rules.** A 4B model with 4 previous descriptions outperforms the same model with sophisticated fusion rules but no context.

### 7.7 Remaining limitations

- **Pages 7-8** (early in chapter): Only 1-2 descriptions available, not enough context → still misclassified
- **Page 23** (sadness → epic): Transition page where sadness arc starts but model sees previous tension context
- **Page 34** (peaceful): The only peaceful page in the test set, consistently misclassified as sadness due to surrounding sadness context
- **Historical ceiling of this phase:** ~71% strict with the 4B model on the old sequence benchmark. Later V12 multi-image and the reproduced RealTest historical protocol moved the reference point upward on their respective benchmarks.
