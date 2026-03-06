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
| RealTest BL/1 (74 pages, 70 scored) | **V12 historical protocol + Qwen3-VL-4B-Thinking** | **46/70 strict, 59/70 relaxed (84.3%)** | Historical Windows winner, now reproduced on Linux and used as the default `realtest_benchmark` protocol. |

The default `realtest_benchmark` in the codebase now replays the **historical RealTest protocol**:
- model: `Qwen3-VL-4B-Thinking`
- prompt: historical 3-page sequence prompt
- no grammar constraint
- `max_tokens: 8192`
- no mood shuffle

The former "modern" RealTest variant (`Qwen3.5-4B`, grammar, shuffled mood order, `max_tokens: 50`) is still available for comparison via `REALTEST_PROFILE=modern`, but it is no longer the default benchmark path.

Common commands:

```bash
# Default historical RealTest protocol
REALTEST_FILTER=BL/1 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture

# Modern comparison variant
REALTEST_PROFILE=modern REALTEST_FILTER=BL/1 cargo test --manifest-path src-tauri/Cargo.toml realtest_benchmark -- --ignored --nocapture

# Original 31-page Blue Lock sequence benchmark
cargo test --manifest-path src-tauri/Cargo.toml bluelock_sequence -- --ignored --nocapture
```

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
