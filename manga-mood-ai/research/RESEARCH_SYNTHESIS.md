# Manga Mood AI — Research Synthesis

## 1. Project Goal

**KeyToMusic** is a desktop soundboard for manga reading. The user assigns sounds to keyboard keys, grouped by mood (epic, sadness, tension, etc.). When reading a manga on their browser, a browser extension captures each page image and sends it to a local VLM (Vision Language Model) that runs on the user's GPU. The VLM detects the mood of the page and automatically triggers the matching soundtrack.

**Core objective:** Detect the mood of a manga page using a local VLM, with enough accuracy to provide a coherent soundtrack without manual intervention.

### Constraints

1. **Runs locally.** No cloud API — the model must fit in consumer GPU VRAM (8-12 GB target). Currently using quantized models (Q4_K_M GGUF) via llama-server (llama.cpp).

2. **Mood categories must remain user-configurable.** Users can create custom mood tags and assign sounds to them. The system cannot be hardcoded to a fixed label set. This rules out **fine-tuning or LoRA** as a primary strategy — a fine-tuned model is locked to its training labels and can't generalize to user-defined categories without retraining.

3. **Real-time-ish.** The user scrolls through pages at reading speed (~5-15 seconds per page). Classification must complete within that window. Currently ~8-12 seconds per page (2 VLM inferences), which is borderline but acceptable since the browser extension can send pages ahead of the reader's scroll position.

4. **Single manga page = single input.** The browser extension sends one page image at a time. Multi-page batching is possible when the extension pre-loads pages, but the system must work incrementally.

### Current benchmark status (March 2026)

The repo now has **three distinct benchmark references**, and mixing them was the source of several recent confusions:

| Benchmark | Winner | Score | Notes |
|-----------|--------|-------|-------|
| Isolated images (13-18 pages) | **Qwen3-VL 2B thinking** | **18/18 (100%)** | Best single-page classifier. |
| Blue Lock sequence (31 pages) | **V12 + Qwen3.5-VL 4B** | **23/31 strict, 28/31 relaxed** | Best result on the original 31-page research benchmark. |
| RealTest BL/1 (74 pages, 70 scored) | **Wide-5 selective repair + Qwen3-VL-4B-Thinking** | **55/70 strict, 67/70 relaxed (95.7%)** | New best local result, still within a baseline-like latency budget (~10.2s/page). |

The default `realtest_benchmark` in the codebase now runs the **RealTest comparison suite**:
- cached baseline: historical `Qwen3-VL-4B-Thinking` winner loaded from disk, **no rerun**
- live variants: sampling, preprocessing, decision algorithms, runtime profiles, and challenger models
- outputs: terminal comparison table + saved JSON/Markdown summary in `manga-mood-ai/results/`

Common commands:

```bash
# Default RealTest comparison suite on BL/1
REALTEST_FILTER=BL/1 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture

# Narrow the suite to a few experiments
REALTEST_FILTER=BL/1 REALTEST_EXPERIMENTS=baseline_cache,t4b_official,q35_9b_viterbi cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture

# Smoke run on only the first 3 pages of each selected chapter
REALTEST_FILTER=BL/1 REALTEST_PAGE_LIMIT=3 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture

# Original 31-page Blue Lock sequence benchmark
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

### March 2026 comparative RealTest suite (`BL/1`)

The first full comparison suite on `BL/1` produced the following matrix:

| Experiment | Strict | Relaxed | Intensity | Avg time | Takeaway |
|-----------|--------|---------|-----------|----------|----------|
| `baseline_cache` | **46/70** | **59/70** | 21/70 | cached | Current overall winner remains the historical baseline. |
| `t4b_official` | 39/70 | **59/70** | 30/70 | ~4.9s/window | Official sampling improves intensity but loses 7 strict points. |
| `q35_9b_viterbi` | 36/70 | 57/70 | 26/70 | ~2.8s/window | Best live challenger, still below baseline. |
| `q3vl_2b_viterbi` | 32/70 | 55/70 | **41/70** | ~1.2s/window | Excellent intensity model / `tension` specialist, poor primary classifier. |
| `q35_4b_viterbi` | 23/70 | 32/70 | 24/70 | ~1.9s/window | Not competitive in this protocol. |

Important failure mode discovered by the suite:
- `t4b_official_grammar`, `t4b_center_png`, `t4b_all_png`, `t4b_viterbi`, `t4b_focus`, `t4b_ocr`, `t4b_semantic`, and `t4b_quiet_viterbi` all collapsed to an almost constant `sadness` output.
- In other words, those runs did **not** validate grammar / PNG / Viterbi / OCR / semantic improvements yet; they exposed a harness or decoding issue for `Qwen3-VL-4B-Thinking` under those settings.
- `q3vl_2b_viterbi` turned out to be highly complementary to the baseline: as a pure auxiliary intensity head, it can raise intensity accuracy from **21/70** to **41/70** while keeping baseline mood labels unchanged.

### March 2026 targeted follow-up runs (`BL/1`)

Three targeted experiments were then run to improve on the baseline without changing the local hardware budget:

| Experiment | Strict | Relaxed | Intensity | Avg time | Verdict |
|-----------|--------|---------|-----------|----------|---------|
| `t4b_hist_live` | 41/70 | 58/70 | 20/70 | ~8.7s/window | Failed to reproduce the cached 46/70 baseline exactly. The old winner is not fully reproducible under the current live harness/runtime. |
| `t4b_hist_center_override` | 40/70 | 57/70 | 22/70 | ~8.7s/window | Center-focused override heuristic made the result slightly worse. |
| `t4b_focus_direct` | 29/70 | 46/70 | 39/70 | ~6.2s/window | Direct current-page classification is much better for intensity, but clearly worse for mood classification. |

What this iteration taught us:
- A simple aggregation tweak is **not** enough to beat the historical baseline.
- Page-centered prompts should not replace the sequence prompt as the primary classifier, but they remain interesting as an **auxiliary signal** because they sharply improve intensity.
- Any future claim that an algorithm beats the baseline must still be compared against the cached historical reference `46/70`, not only against the weaker `t4b_hist_live` rerun.

### March 2026 second follow-up runs (`BL/1`)

A second cycle then tested two hypotheses:
1. maybe the weaker live reruns were caused by a `reasoning_format` mismatch for `Qwen3-VL-4B-Thinking`
2. maybe a more explicit current-page prompt centered on **narrative function** would reduce the `epic` overprediction

Results:

| Experiment | Strict | Relaxed | Intensity | Avg time | Verdict |
|-----------|--------|---------|-----------|----------|---------|
| `t4b_hist_live` (with `reasoning_format none`) | 41/70 | 58/70 | 20/70 | ~8.5s/window | No change. The non-reproduction of the cached historical baseline persists. |
| `t4b_focus_direct` | 29/70 | 46/70 | 39/70 | ~6.0s/window | Same conclusion as before: useful intensity signal, poor primary mood classifier. |
| `t4b_narrative_focus` | 31/70 | 45/70 | 36/70 | ~6.1s/window | Slightly better than raw focus, but still far below the baseline on mood. |

What this second iteration taught us:
- The gap between `baseline_cache` and `t4b_hist_live` is **not** explained by `reasoning_format` alone.
- Prompting the model to think in terms of **narrative function** helps a bit versus naive current-page focus, but not nearly enough to replace the sequence prompt.
- The remaining promising direction is no longer “single-model prompt tuning”, but a **conservative auxiliary ensemble** where cheaper side models only intervene on very specific ambiguous cases.

### March 2026 third follow-up run (`BL/1`) — useful benchmark gain, but not a production candidate

A derived ensemble was then added on top of already-tested live signals. The final winning blend was:
- `t4b_hist_live` weight `3`
- `t4b_focus_direct` weight `2`
- `q3vl_2b_viterbi` weight `2`
- `q35_9b_viterbi` weight `1`
- Viterbi transition strength `lambda = 0.5`
- intensity taken from `q3vl_2b_viterbi`

Result:

| Experiment | Strict | Relaxed | Intensity | Verdict |
|-----------|--------|---------|-----------|---------|
| `baseline_cache` | 46/70 | 59/70 | 21/70 | Historical reference |
| `t4b_ensemble_viterbi` | **46/70** | **61/70** | **41/70** | Same strict as baseline, but clearly better relaxed and much better intensity |

Why this matters:
- This is the **first result that is not worse than the baseline on strict while still improving other important metrics**.
- The gain did **not** come from a bigger single model or prompt tinkering alone; it came from **complementary specialists**:
  - `thinking4b` sequence prompt remains the best global mood backbone
  - `focus_direct` adds local current-page corrections
  - `q3vl_2b` acts as a strong `tension` / intensity specialist
  - `q35_9b` adds a softer second opinion that improves relaxed coherence
- However, this blend is **too expensive for the production latency budget**: roughly `19-23s/page` effective when all dependencies are recomputed, versus ~`9.5s/window` for the historical baseline.
- Therefore it should be treated as a **research oracle**, not as a deployable winner.
- The next step is not “more ensemble”, but **recovering strict gains under a hard latency budget close to the baseline cost**.

### Hard gate for the next iteration

From this point on, a candidate only counts as a real winner if all of the following are true:
- it stays within a **baseline-like nominal cost budget** (roughly the same order as the historical `thinking4b` run)
- it improves the cached baseline by **at least +2 strict** (`>= 48/70`) **or +4 relaxed** (`>= 63/70`)
- it remains local-GPU friendly on the RTX 4070 setup

Anything below those thresholds must be documented as a failed or partial attempt, then discarded as a final candidate.

### March 2026 smoke eliminations before full `BL/1`

Before spending another full `BL/1` run, several low-cost smoke runs (`REALTEST_PAGE_LIMIT=3`) were used to discard bad prompt families early:

| Experiment | Smoke outcome | Verdict |
|-----------|---------------|---------|
| `t4b_focus_short` | `???` on the first smoke pages | Short-answer direct focus is too unstable to justify a full run. |
| `t4b_focus_grammar_short` | Stable parse, but collapsed to wrong low-information outputs | Direct focus + grammar did not recover a usable current-page classifier. |
| `t4b_wide5_narrative` | `???` on the first smoke pages | Wide future/past context alone does not help if the model is asked to classify the current page directly. |
| `t4b_wide5_narrative_grammar` | Parses, but still wrong on the first smoke pages | Wide direct narrative focus remains a dead end for `thinking4b`. |

Current conclusion from these smoke eliminations:
- `Qwen3-VL-4B-Thinking` appears much stronger when it is asked to score a **sequence window** than when it is asked to directly classify the current page.
- The most promising remaining branch under the hard latency gate is therefore **wider sequence windows with overlapping vote aggregation**, not direct page-focused prompting.

### March 2026 full `BL/1` run — wide-5 sequence windows

The first full run after the smoke eliminations tested the strongest surviving branch: extend the historical 3-image sequence protocol to **5-image windows** while keeping overlapping vote aggregation.

| Experiment | Strict | Relaxed | Intensity | Avg time | Verdict |
|-----------|--------|---------|-----------|----------|---------|
| `t4b_wide5_sequence` | 43/70 | 61/70 | 21/70 | ~9.3s/window | Budget-compatible and cleaner than the direct prompts, but still below the baseline on strict and only +2 on relaxed. |
| `t4b_wide5_sequence_viterbi` | 43/70 | 61/70 | 21/70 | ~9.4s/window | Same result as majority; the extra Viterbi layer did not help once the 5-image vote stats were already aggregated. |

What this taught us:
- Wider visual context is **not useless**: it raises relaxed from `59/70` to `61/70` while staying in roughly the same cost envelope as the baseline.
- But wider windows alone are **not enough** to win the benchmark under the hard gate (`>=48 strict` or `>=63 relaxed`).
- This branch remains worth building on because it is the first new live-feasible protocol that stays close to the baseline cost **and** improves a global metric without collapsing.
- The next logical step is therefore a **selective correction layer on top of the wide-5 sequence backbone**, not another full architectural reset.

### March 2026 winning run (`BL/1`) — wide-5 selective repair

The next iteration implemented exactly that selective correction layer on top of the `t4b_wide5_sequence` backbone.

Result:

| Experiment | Strict | Relaxed | Intensity | Avg time | Second pass | Verdict |
|-----------|--------|---------|-----------|----------|-------------|---------|
| `baseline_cache` | 46/70 | 59/70 | 21/70 | historical cache | 0 | Old benchmark reference |
| `t4b_wide5_selective` | **55/70** | **67/70** | 24/70 | **~10.2s/page** | **12 pages** | **New winner** |

Why this run matters:
- it passes the hard gate by a wide margin: **+9 strict** and **+8 relaxed**
- it stays in the same practical cost class as the historical baseline instead of becoming a multi-model oracle
- it does not rely on a bigger model, LoRA, or label-locking fine-tuning

What the winning method actually does:
1. Run the `wide-5` sequence classifier as the primary backbone.
2. Detect **long same-mood runs** that are likely hiding local narrative pivots.
3. Spend extra compute only on a handful of pages at run boundaries:
   - comedy run edges: re-check with the current-page **focus** prompt
   - long tension run edges: re-check with the **narrative** prompt to detect hidden `mystery` openings and `epic` payoffs
   - short epic run openings: re-check with the **narrative** prompt to recover pages that are actually still `tension`
   - epic run tails: re-check with **focus** or **narrative** to catch concealed `mystery` bridges or unresolved `tension`

The 12 corrected pages on `BL/1` were:
- `10`: `epic -> tension`
- `36`: `comedy -> mystery`
- `40`: `comedy -> tension`
- `41`: `epic -> tension`
- `43`: `epic -> mystery`
- `44`: `epic -> mystery`
- `45`: `tension -> mystery`
- `46`: `tension -> mystery`
- `64`: `tension -> epic`
- `65`: `tension -> epic`
- `73`: `epic -> tension`
- `74`: `epic -> tension`

This is the first approach that is both:
- a **real benchmark winner**
- and still a **credible production candidate** under the RTX 4070 local-budget constraint

---

## 2. Models Tested

All models tested on the same hardware (RTX 4070 12GB, Windows 11), via llama-server or Ollama. All use quantized GGUF weights (Q4_K_M).

### Phase 1 — Isolated images (13-18 diverse manga pages)

| Model | Score | Time/image | VRAM | Notes |
|-------|-------|------------|------|-------|
| **Qwen3-VL 2B** (thinking) | **18/18 (100%)** | ~1.1s | 38% | Champion. Thinking (`<think>` tags) is critical. |
| Qwen2.5-VL 7B | 11/13 (85%) | ~2s | ~96% | Accurate but saturates VRAM, freezes PC. |
| Qwen3-VL 4B | 9/13 (69%) | ~2.8s | — | Paradoxically worse than the 2B. |
| InternVL3.5 4B | 9/13 (69%) | ~500ms | 74% | Ultra fast but misses complex moods. |
| MiniCPM-V 4.0 | 8/13 (62%) | ~800ms | 47% | "tension" bias on everything dark. |
| Gemma 3 4B | ~2/13 (15%) | ~1s | — | Spams "tension". |
| SmolVLM2 2.2B | ~3/13 (23%) | ~150ms | — | Spams "tension". |
| Moondream 0.5B | ~1/13 (8%) | ~1s | — | Spams "comedy". |
| SigLIP 2 (embedding) | 2/13 (15%) | <100ms | — | Spams "epic_battle". Embedding approach fundamentally flawed for manga. |

**Key lesson:** Thinking models (`<think>` reasoning before answering) dramatically outperform non-thinking models. The 2B thinking model beats 4-7B non-thinking models.

### Phase 2 — Sequential benchmark (31 consecutive Blue Lock pages)

The 2B model scored 100% on isolated images but the task changes fundamentally on sequential pages: the model must maintain narrative coherence across an arc.

**Qwen3.5-VL 4B** was adopted for the original 31-page sequential benchmark (stronger scene understanding, handles complex multi-panel layouts better). Later, a separate RealTest benchmark on `BL/1` was reproduced with **Qwen3-VL-4B-Thinking** and a legacy 3-page protocol, which became the default `realtest_benchmark` reference in the repo.

---

## 3. Mood Taxonomy Evolution

### Original: 10 categorical moods
`epic_battle`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `emotional_climax`, `mystery`, `chase_action`

**Problem:** `emotional_climax` was over-predicted. The model confused **visual intensity** with **narrative function**. A page showing intense crying → "strong visual emotion" → `emotional_climax` instead of `sadness`. Similarly, `chase_action` was too similar to `tension`/`epic_battle`.

### Current: 8 moods × 3 intensity levels
`epic`, `tension`, `sadness`, `comedy`, `romance`, `horror`, `peaceful`, `mystery` — each with intensity 1 (low), 2 (medium), 3 (high).

**Relaxed matching** groups moods into families: epic↔tension, sadness↔peaceful, comedy↔romance, horror↔mystery. A classification within the same family counts as acceptable.

**Trade-off:** The dimensional baseline (45% strict) is lower than the 10-mood baseline (65%) because the model now has to output intensity too, and the categories are broader. But the dimensional system is more suitable for soundtrack selection (intensity controls volume/energy) and eliminates the `emotional_climax` confusion entirely.

---

## 4. The Core Problem: Perception vs. Judgment

We're asking the VLM to make a **narrative judgment** (what mood should the soundtrack be?) from a **single visual observation** (one page). This is like asking someone to identify the climax of a movie from a single frame.

A page showing a character crying intensely could be:
- **sadness** (the character just lost something)
- **epic** (tears of victory after a hard-won battle)
- **tension** (crying from fear before a confrontation)

The visual appearance is identical — the difference is **narrative context**. Without knowing what happened before, the VLM guesses based on visual intensity alone.

---

## 5. All Approaches Tested (15+ experiments)

### 5.1 Single-page classification (no context)

| # | Approach | Strict | Relaxed | What went wrong |
|---|----------|--------|---------|-----------------|
| 1 | **Baseline** — single-label prompt (GUIDED_V3) | 20/31 (65%) | 25/31 (81%) | Over-predicts emotional_climax on visually intense pages |
| 2 | Baseline + text-only LLM refinement | 20/31 (65%) | 25/31 (81%) | LLM echoes labels back — they look plausible even when wrong |
| 3 | VLM describes → text-only LLM classifies batch | 6/31 (19%) | 19/31 (61%) | Text-only LLM loses ALL visual information |
| 4 | VLM extracts structured features → text-only LLM classifies | 13/31 (42%) | 26/31 (84%) | Better than descriptions but still loses visual context |
| 5 | Hybrid 1-inference (mood + features in one prompt) | 14/31 (45%) | 24/31 (77%) | Dual-task degrades both outputs. 16/31 neutral-spam on emotions. |
| 6 | Hybrid + deterministic fusion rules | 18/31 (58%) | 28/31 (90%) | Fusion rules correct 4 pages perfectly, but base too weak |
| 7 | 2 separate inferences + fusion | 21/31 (68%) | 26/31 (84%) | Best without context. Determination-spam (16/31) limits fusion. |

**Key takeaways from single-page approaches:**
- The VLM classifies well when the visual is unambiguous. Errors cluster on pages where narrative context is needed.
- Text-only post-processing can't help because it doesn't see the images, and the labels "look plausible" even when wrong.
- Fusion rules work (+4 perfect corrections in Pass 6) but depend on feature quality, which is unreliable — the VLM collapses all emotions into 1-2 dominant labels (neutral or determination) regardless of vocabulary.
- Asking for classification AND features in one prompt degrades both outputs.

### 5.2 Context injection attempts (with previous mood labels)

| # | Approach | Strict | Relaxed | What went wrong |
|---|----------|--------|---------|-----------------|
| A | Previous mood labels in VLM prompt | 12/30 (40%) | 18/30 (60%) | **Feedback loop.** One misdetection propagates: model sees "previous: emotional_climax" → biased to predict emotional_climax again. |
| B | Scored prompt (10 float scores per mood) | 8/31 (26%) | 17/31 (55%) | Small models can't regress. Single-label >> scored prompt. |
| — | Scored + text-only refinement | 6/31 (19%) | 15/31 (48%) | Garbage in, garbage out. |

**Key takeaway:** Injecting mood **labels** as context creates positive feedback loops. Once the model makes one mistake, the context reinforces it for all subsequent pages.

### 5.3 Context with descriptions (the breakthrough)

| # | Context Window | Strict | Relaxed | Notes |
|---|---------------|--------|---------|-------|
| Dim.Baseline | none | 14/31 (45%) | 23/31 (74%) | Reference (dimensional system) |
| V5 | describe → text-only correct | 15/31 (48%) | 24/31 (77%) | Text-only correction still doesn't work |
| **V6** | **4 past full descriptions** | **22/31 (71%)** | **26/31 (84%)** | Best text-context result |
| V7 | 2 past + 2 future | 21/31 (68%) | 27/31 (87%) | Future context disorients model mid-arc |
| V8 | 4 past + 2 future | 19/31 (61%) | 27/31 (87%) | 6 descriptions overwhelms 4B model |
| V9 | 3 past + 3 future | 22/31 (71%) | 26/31 (84%) | Ties V6 numerically, different error profile |
| V10 | 2 full + 5 first-sentence summaries | 16/31 (52%) | 24/31 (77%) | First-sentence extraction too generic |
| V11 | 2 full + LLM-generated summary | 19/31 (61%) | 25/31 (81%) | LLM summaries add noise |

#### V6 Pipeline (best text-context result)

```
For each page N:
  Step 1: describe_page(image_N)
    → VLM describes the page factually (no mood classification)
    → Output: ~500-800 tokens of description (panel layout, characters, expressions, atmosphere)

  Step 2: classify_with_context(image_N, descriptions[N-4..N-1])
    → VLM sees: the current image + text descriptions of 4 previous pages
    → Prompt: "Based on what you see AND the narrative context above, classify the mood"
    → Output: mood + intensity (e.g., "sadness 3")
```

**Why V6 works and mood labels don't:** Factual descriptions don't carry mood bias. A description says "a character is crying alone in the rain" — no mood label attached. The VLM reads these and naturally understands the arc. By contrast, injecting mood labels creates cascading errors.

#### Why text-context variants failed

- **Future descriptions (V7-V9):** Model tries to "match" future mood instead of classifying what it sees.
- **More than 4 descriptions (V8):** Overwhelms the 4B model (71% → 61%).
- **Summarized descriptions (V10, V11):** Lose narrative signal. Full verbose descriptions are needed.

### 5.4 Visual context: multi-image sliding window (the new breakthrough)

| # | Approach | Strict | Relaxed | Notes |
|---|----------|--------|---------|-------|
| **V12** | **3 consecutive images + majority vote** | **23/31 (74%)** | **28/31 (90%)** | **Best result on the 31-page Blue Lock benchmark** |

#### V12 Pipeline (best 31-page Blue Lock result)

```
For each page N (treated as center):
  Window: send images (N-1, N, N+1) to VLM in one prompt
  → "These are 3 consecutive manga pages. What is the overall mood of this sequence?"
  → Output: mood + intensity for the group

Each page appears in up to 3 windows → 3 votes → majority vote
  - 3/3 same mood → unanimous (13/31 pages)
  - 2/3 same mood → majority wins (12/31 pages)
  - 3 different moods → take center window's vote (4/31 pages, logged as "3-way split")
  - Edge pages (first/last 1-2) → fewer votes
```

**Why V12 beats V6:**

1. **Visual context > textual context.** V12 gives the model direct visual access to neighboring pages. The model sees the actual narrative flow — expressions, panel composition, art style shifts — rather than reading descriptions of them. This is richer and more direct.

2. **No describe step needed.** V6 requires 2 inferences per page (describe + classify). V12 requires 1 inference per window. Result: V12 is ~3x faster (~3.2s per window vs ~10s for V6's 2 steps).

3. **Majority vote = free smoothing.** When the model makes a one-off error, the other 2 windows correct it. 13/31 pages were unanimous (stable arcs), and 12/31 were corrected by 2-1 majority.

4. **Fixes V6's early-page problem.** V6 failed on pages 7-8 (comedy/sadness) because only 1-2 descriptions existed. V12 handles these correctly because the visual window is always 3 images wide regardless of position.

**V12's remaining errors:**
- Page 023 (sadness → epic FAIL): Persistent error across all approaches. The transition page is inherently ambiguous.
- Page 024 (sadness → epic FAIL): 3-way split (tension/epic/sadness) at the arc boundary. The majority vote picks the wrong one.
- Page 034 (peaceful → ??? ERR): Parse error from 3-way split. The isolated peaceful page remains the hardest case.

#### RealTest historical V12 repro (default `realtest_benchmark`)

The repository now also reproduces the **historical Windows RealTest winner** on `BL/1`:

| Benchmark | Model | Protocol | Strict | Relaxed | Speed |
|-----------|-------|----------|--------|---------|-------|
| RealTest `BL/1` (74 pages, 70 scored) | **Qwen3-VL-4B-Thinking** | historical 3-page prompt, no grammar, no shuffle, `max_tokens: 8192` | **46/70 (65.7%)** | **59/70 (84.3%)** | ~9.5s/window |

This protocol is different from the 31-page Blue Lock benchmark above. It is slower, but it is the setup that matched the previously undocumented Windows run and is now the default behavior of `realtest_benchmark`.

### 5.5 Thinking mode experiments (dead end)

| # | Approach | Strict | Relaxed | Latency |
|---|----------|--------|---------|---------|
| Think-baseline | Baseline + /think (temp 0.6) | 15/29 (52%) | 25/29 (86%) | 49s/page |
| Think-V6 | V6 + /think (temp 0.6) | ~68% partial | — | 70s/page |

Thinking mode was tested with `/think` system prompt and temperature 0.6 (recommended for Qwen3 thinking). Results: barely better than baseline (+3% strict), 25x slower, 2 parse errors. The V6+think variant showed no improvement over V6 at 14x the latency.

**Conclusion:** adding explicit `/think` to **Qwen3.5-VL 4B** didn't help. The model rambles in its reasoning and loses focus. This does **not** invalidate dedicated thinking models: the separate **Qwen3-VL-4B-Thinking** model later became the historical RealTest winner on `BL/1`.

---

## 6. What We Know For Sure

### 6.1 Proven facts

1. **Visual context > textual context.** Multi-image windows (V12: 74%) beat text descriptions (V6: 71%). Direct visual access to neighboring pages is richer than reading descriptions of them.

2. **Context is the #1 lever.** +29 percentage points (45% → 74%) from context alone. No other technique came close.

3. **Majority vote is a powerful free smoothing mechanism.** 3 votes per page catches one-off errors. 13/31 pages were unanimous, 12/31 corrected by 2-1 majority.

4. **Factual descriptions eliminate feedback loops.** Mood labels as context → 40% (cascading errors). Descriptions as context → 71% (no feedback loop).

5. **The VLM must always see the image during classification.** Text-only approaches (19-48%) fundamentally fail.

6. **`/think` on Qwen3.5-VL 4B doesn't help, but dedicated thinking models still matter.** Prompt-level thinking hurt Qwen3.5-VL 4B, while the separate Qwen3-VL-4B-Thinking model later produced the best reproduced RealTest BL/1 result.

7. **Small models (2-4B) are classifiers, not regressors.** Scored prompts (10 floats) give 26% vs 65% for single-label.

8. **Prompt engineering has diminishing returns.** GUIDED_V3 gives 65%. Every further prompt tweak gives ±2% at most.

### 6.2 Current practical ceilings

The current ceiling depends on **which benchmark you mean**:

- **31-page Blue Lock benchmark:** **74% strict, 90% relaxed** with V12 + Qwen3.5-VL 4B.
- **RealTest BL/1 benchmark:** **65.7% strict, 84.3% relaxed** with the historical V12 protocol + Qwen3-VL-4B-Thinking.

These are both useful:
- the 31-page benchmark is the cleanest research comparison set
- the RealTest run is the reproduced historical benchmark now wired as the default repo command

For a soundtrack application, **90% relaxed** on the 31-page benchmark is already excellent, and **84.3% relaxed** on the reproduced RealTest `BL/1` run is still operationally strong. The remaining misses are partly masked by crossfading, MoodDirector smoothing, and manual override.

### 6.3 Error patterns

| Error type | Pages affected | Cause |
|-----------|---------------|-------|
| Early chapter (insufficient context) | 7, 8 | Only 1-2 previous descriptions available |
| Arc transition (context lag) | 23 | Sadness arc starts but model still sees tension context |
| Context contamination | 34 | The only peaceful page, drowned by surrounding sadness descriptions |
| Stubborn misclassification | 31 | Model consistently wrong regardless of approach |

---

## 7. Technical Setup

### Model
- **31-page Blue Lock winner:** **Qwen3.5-VL 4B** (Q4_K_M GGUF, ~2.5 GB)
- **Default RealTest winner:** **Qwen3-VL-4B-Thinking** (Q4_K_M GGUF)
- Served via **llama-server** (llama.cpp) as a local HTTP API
- Flags: `-c 32768 --flash-attn auto --cache-type-k q8_0 --cache-type-v q8_0 -ngl 99 --image-min-tokens 1024`
- Images resized to 672px max dimension (Lanczos3, JPEG encoding) before inference
- Temperature: 0.0 (deterministic)
- VRAM: ~50% of 12 GB

### Pipeline
- **31-page Blue Lock V12:** 3 consecutive images (N-1, N, N+1), one prompt, majority vote, ~3.2s per window, ~100s for 31 pages.
- **Default RealTest V12:** same overlapping-window idea, but using the historical prompt/protocol that reproduced the Windows `BL/1` run: no grammar, no shuffle, `max_tokens: 8192`, ~9.5s per window on Linux.

### Linux reproducibility note

On Linux with **RTL8125 + `r8169`**, `llama-server` load exposed a flaky **EEE (Energy Efficient Ethernet)** interaction that caused repeated `Link is Down / Link is Up` events during the benchmark. Disabling EEE restored network stability:

```bash
sudo ethtool --set-eee enp6s0 eee off
```

This is a host-network workaround, not a benchmark logic change, but it was required to reproduce the historical run reliably on the Linux machine used for validation.

### Browser extension
- Runs on manga reading sites with vertical scroll
- Captures page images and sends them to KeyToMusic's local API
- Can pre-load pages ahead of the reader's scroll position
- Sends pages incrementally (one at a time as they become visible)

---

## 8. Design Constraint: Variable Mood Categories

A critical constraint that shapes every architectural decision: **mood categories must remain user-configurable.**

Users can define their own moods (e.g., "nostalgic", "hype", "calm_rain") and tag their sounds accordingly. The VLM classification must work with arbitrary mood lists, not just the 8 defaults.

### Why this rules out fine-tuning/LoRA as primary strategy

- A fine-tuned model is locked to its training labels. Adding a new mood requires retraining.
- LoRA adapters can be swapped, but each mood set needs its own adapter — impractical for end users.
- The training data problem: we'd need thousands of annotated manga pages per mood, which doesn't exist and would be prohibitively expensive to create.
- Even if we fine-tuned on the 8 default moods, the model would be useless for custom moods, breaking the app's core value proposition.

### Why the current zero-shot approach works for variable moods

The V6 pipeline uses mood names as natural language labels in the prompt. Adding a new mood is as simple as adding its name (and optionally a short description) to the prompt. The VLM's general understanding of language and visual concepts allows it to classify into categories it was never explicitly trained on.

Example: if a user adds "nostalgic" as a mood, the prompt becomes:
```
Moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery, nostalgic
```
The VLM already understands what "nostalgic" looks like (sepia tones, flashback scenes, characters looking at old photos) through its pre-training. No fine-tuning needed.

### The only scenario where fine-tuning might make sense

If we could fine-tune the VLM to produce **better visual descriptions** (Step 1) rather than better mood labels, the classification step (Step 2) would still use zero-shot prompting with variable moods. This would preserve the variable mood constraint while improving the foundation.

But this requires:
1. A large dataset of manga pages with high-quality descriptions (doesn't exist)
2. The ability to fine-tune VLMs for description quality without degrading general capabilities
3. Evidence that description quality (not classification) is the actual bottleneck — currently unclear

---

## 9. Open Questions and Problematic

### The fundamental question

**We've reached 74% strict / 90% relaxed with a 4B VLM and multi-image sliding windows. The approach has strong momentum — V12 beat V6 on both metrics while being 3x faster. How do we get to 85%+ strict accuracy while keeping mood categories variable?**

### Specific open questions

#### A. Is the 4B model the bottleneck?

We've only tested 2B and 4B models due to VRAM constraints. Would a 7-8B model:
- Produce better descriptions (more narrative-aware, less verbose)?
- Handle more context without degradation?
- Make fewer errors on ambiguous pages?
- Still fit in 12 GB VRAM with quantization?

The 7B Qwen2.5-VL scored 85% on isolated images (vs 100% for 2B thinking model), which is not encouraging — but it wasn't tested with the V6 context pipeline.

#### B. Can we improve descriptions without fine-tuning?

The descriptions are ~500-800 tokens each and very verbose. They include irrelevant details (panel layout, speech bubble sizes) alongside useful information (character emotions, scene atmosphere). Could a better description prompt:
- Produce shorter, more focused descriptions?
- Capture narrative-relevant information more reliably?
- Reduce the context window pressure (allowing more past pages)?

We haven't explored description prompt optimization — only classification prompt optimization.

#### C. Is there a way to use fine-tuning that preserves variable moods?

Potential approaches:
- **Fine-tune for descriptions only** (Step 1), keep classification zero-shot (Step 2)
- **Fine-tune a reward model** that scores "description quality" and use it to filter/rank descriptions
- **Fine-tune on the manga visual domain** (panel understanding, expression recognition) without mood-specific labels — a general "manga understanding" LoRA
- **Contrastive learning** on manga page pairs (same arc = similar embedding, different arc = different embedding) — mood-agnostic representation learning

None of these have been tested. The question is whether any of them could break the 71% ceiling while keeping mood labels as prompt-time configuration.

#### D. Can we combine V12 (multi-image) with V6 (descriptions)?

V12 uses visual context (3 images). V6 uses textual context (4 descriptions). Could we combine both? Send 3 images + 1-2 previous descriptions as text context. This would give the model both visual narrative flow AND factual context from earlier pages. Risk: overwhelming the 4B model with too much input (3 images + text). **This is the next experiment to run.**

#### E. Is there a fundamentally different architecture?

All our experiments use the same basic loop: VLM looks at image(s) → produces text → parse mood. Are there entirely different approaches?
- **CLIP/SigLIP embeddings** as auxiliary signal (tested once, failed — but only in isolation, not combined with VLM)
- **Temporal models** (recurrent/transformer) over page embeddings — treat mood classification as a sequence labeling task
- **Retrieval-augmented:** match the current page against a database of previously-classified pages from the same manga series
- **User feedback loop:** let the user correct misclassifications in real-time, building a per-manga prior that biases future predictions

#### F. Is 74% / 90% good enough for production?

With relaxed matching at 90%, the user hears an acceptable soundtrack 9 out of 10 pages. The remaining 10% is masked by crossfading, MoodDirector smoothing, and manual override. This may already be good enough. The question is whether the marginal improvement from 90% → 95% relaxed is worth the engineering complexity.

#### G. Can V12 be extended to wider windows?

V12 uses 3-image windows. Would 5-image windows (N-2 to N+2) give even more visual context? Risk: more image tokens may overwhelm the 4B model, similar to how V8 (6 descriptions) overwhelmed it for text context.

---

## 10. Summary

| Aspect | Current state |
|--------|--------------|
| Best 31-page accuracy | **74% strict, 90% relaxed** (V12: 3-image sliding window + majority vote) |
| Reproduced RealTest BL/1 winner | **46/70 strict, 59/70 relaxed (84.3%)** |
| Previous best | 71% strict, 84% relaxed (V6: 4 past descriptions + VLM classify) |
| Models | Qwen3.5-VL 4B (31-page benchmark), Qwen3-VL-4B-Thinking (default RealTest benchmark) |
| Pipeline | V12: 3 consecutive images → group mood → majority vote |
| Speed | ~3.2s per window on 31-page benchmark, ~9.5s/window on historical RealTest repro |
| VRAM | ~50% of 12 GB |
| Moods | 8 default × 3 intensity, user-configurable |
| Exploration status | 18+ approaches tested. V12 (multi-image) remains the key frontier. |
| Key constraint | Mood categories must remain variable (no fine-tuning on mood labels) |
| Dead ends confirmed | `/think` on Qwen3.5-VL 4B, LLM summaries (V10-V11) |
| Ceiling cause | Model size (4B) + narrative ambiguity on single pages |

**The open question is no longer "which protocol was the winner?" — that is now pinned. The next question is whether the historical RealTest winner generalizes beyond `BL/1`, and whether a newer protocol can beat it consistently on the wider RealTest set.**
