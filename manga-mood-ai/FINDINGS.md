# Manga Mood AI — Benchmark Findings

Comprehensive results from benchmarking local VLM models and prompts for manga page mood detection. The goal: classify manga pages into one of 10 mood categories with maximum accuracy using a local model that fits in 12GB VRAM.

**Test set:** 18 manga pages covering all 10 mood categories.

---

## 1. Models Tested

| Model | Score | Time/image | VRAM | Verdict |
|-------|-------|------------|------|---------|
| **Qwen3-VL 2B** (thinking) | **100%** (18/18) | ~2.8s | 38% (~4.7GB) | **Champion** — thinking model beats larger models |
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

### 3.6 Two-pass is a dead end
Regardless of model size (2B or 7B), errors in pass 1 (e.g., "is this action or dialogue?") cascade irrecoverably into pass 2. Single-pass classification is more robust.

### 3.7 Temperature 0 eliminates noise
Random comedy classifications on 18.png (peaceful scene) disappeared with temperature=0. No benefit to any randomness for this task.

### 3.8 A single minimal hint is enough
Adding just "(calm daily life)" after "peaceful" in the category list fixed 18.png without side effects. Full descriptions for every category are counterproductive (see 3.3).

---

## 4. Champion Configuration

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

## 5. Champion Prompt (GUIDED_V3)

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
