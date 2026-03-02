"""
Manga Mood AI — Context-Based Sequential Benchmark.

Tests whether passing previous manga pages as context improves mood detection
accuracy on a continuous sequence (Blue Lock Tome 1, pages 6-36).

Compares: context=0 (baseline) vs context=1 vs context=2 vs context=3

Usage:
    python test_context.py                          # Run all context sizes
    python test_context.py --context 0 2            # Only context=0 and context=2
    python test_context.py --prompt visual           # Use visual prompt
    python test_context.py --prompt visual --context 0 1 2 3
"""

import argparse
import json
import os
import time
import threading
from collections import OrderedDict
from pathlib import Path

import ollama
from PIL import Image
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich import box

console = Console()

MODEL = "qwen3-vl:2b"
SEQUENCE_DIR = Path("test-images/bluelock-sequence")

# ============================================================
# Ground truth — Blue Lock Tome 1 pages 6-36
# ============================================================
# Story arc: JFA meeting → match → defeat → coach speech → melancholy → daily life → mystery letter

SEQUENCE_GROUND_TRUTH = OrderedDict([
    ("BlueLockTome1-006.webp", "tension"),             # JFA boardroom, Anri presenting Blue Lock project, serious discussion
    ("BlueLockTome1-007.webp", "tension"),             # Buratsuta "football is just business", cynical tone, political tension
    ("BlueLockTome1-008.webp", "tension"),             # "Japan will never win" debate, heated confrontation in boardroom
    ("BlueLockTome1-009.webp", "tension"),             # Anri's shocking declaration "we'll never win the World Cup", dramatic close-up
    ("BlueLockTome1-010.webp", "emotional_climax"),    # "It's up to us to create the ultimate player!" — determination, breakthrough speech
    ("BlueLockTome1-011.webp", "mystery"),             # Jinpachi Ego reveal, dramatic silhouette, enigmatic new character
    ("BlueLockTome1-012.webp", "tension"),             # Match start, stadium, 0-1 scoreboard, prefecture final
    ("BlueLockTome1-013.webp", "chase_action"),        # Isagi running with ball, speed lines, action pose, character intro
    ("BlueLockTome1-014.webp", "tension"),             # "NATIONAL!" crowd chanting, Isagi pushing forward, high stakes
    ("BlueLockTome1-015.webp", "tension"),             # Face-off with goalkeeper, dramatic close-up, sweat, decisive moment
    ("BlueLockTome1-016.webp", "tension"),             # "Pass or shoot?" dilemma, teammate calling, internal conflict
    ("BlueLockTome1-017.webp", "tension"),             # "Un pour tous et tous pour un" — team play vs individual, football philosophy
    ("BlueLockTome1-018.webp", "epic_battle"),         # Isagi kicks ball — "un sport qui se joue a 11!", dynamic full-spread action
    ("BlueLockTome1-019.webp", "tension"),             # Goal saved + opponent counter, net impact, fast reversal
    ("BlueLockTome1-020.webp", "chase_action"),        # Opponent striker Kira action pose, speed lines, dynamic movement
    ("BlueLockTome1-021.webp", "sadness"),             # 0-2 scoreboard, "on a perdu", defeat confirmed, stunned faces
    ("BlueLockTome1-022.webp", "sadness"),             # Opponents celebrate, Isagi's team dejected in background, contrast
    ("BlueLockTome1-023.webp", "emotional_climax"),    # Coach post-match speech, "l'objectif c'etait le championnat national", bittersweet
    ("BlueLockTome1-024.webp", "emotional_climax"),    # "Cette defaite restera gravee dans vos memoires" — coach crying, powerful speech
    ("BlueLockTome1-025.webp", "emotional_climax"),    # "Rien n'est vain dans la vie!" — team crying, "meilleure equipe du Japon"
    ("BlueLockTome1-026.webp", "sadness"),             # Walking home, "on n'est qu'une equipe qui a echoue", melancholy atmosphere
    ("BlueLockTome1-027.webp", "sadness"),             # "Je ne serai jamais un grand heros" — watching Noel Noa on TV, defeat reflection
    ("BlueLockTome1-028.webp", "sadness"),             # Flashback to team play, "mon reve ne restera qu'un reve", despair
    ("BlueLockTome1-029.webp", "sadness"),             # "De gagner la coupe du monde..." — dream fading, watching World Cup flashback
    ("BlueLockTome1-030.webp", "sadness"),             # "Aurais-je...? Si j'avais tire..." — regret flashback, what-if
    ("BlueLockTome1-031.webp", "sadness"),             # Flashback to missed shot, imagining the goal, dark shading
    ("BlueLockTome1-032.webp", "sadness"),             # "Mon destin aurait-il ete different?" — existential doubt, walking away
    ("BlueLockTome1-033.webp", "sadness"),             # "Je voulais gagner...!" — lying in bed crying, alarm ringing, raw emotion
    ("BlueLockTome1-034.webp", "peaceful"),            # Next day, daily life, girl with letter, normal conversation
    ("BlueLockTome1-035.webp", "mystery"),             # "POURQUOI SUIS-JE...?!" — JFA selection letter reveal, shock, big eyes
    ("BlueLockTome1-036.webp", "mystery"),             # Arriving at JFA building, "j'etais heureux d'avoir ete selectionne", anticipation
])

# Acceptable alternatives (moods are subjective, especially in transitions)
SEQUENCE_ACCEPTABLE_ALT = {
    "BlueLockTome1-006.webp": ["peaceful", "mystery"],       # Boardroom could read as calm meeting or mystery setup
    "BlueLockTome1-009.webp": ["emotional_climax"],           # Dramatic declaration could be climactic
    "BlueLockTome1-010.webp": ["tension"],                    # Determination could read as tension
    "BlueLockTome1-011.webp": ["tension", "emotional_climax"],# Ego reveal is dramatic, could be tension or climax
    "BlueLockTome1-013.webp": ["tension", "epic_battle"],     # Running with ball — action or tension
    "BlueLockTome1-014.webp": ["epic_battle", "chase_action"],# Crowd chanting, pushing forward
    "BlueLockTome1-015.webp": ["epic_battle"],                # Face-off could be epic battle too
    "BlueLockTome1-017.webp": ["emotional_climax"],           # Team philosophy speech
    "BlueLockTome1-018.webp": ["chase_action", "tension"],    # Big kick — action moment
    "BlueLockTome1-019.webp": ["epic_battle", "chase_action"],# Goal action
    "BlueLockTome1-020.webp": ["epic_battle", "tension"],     # Striker action
    "BlueLockTome1-021.webp": ["tension"],                    # Defeat can read as tension too
    "BlueLockTome1-022.webp": ["tension"],                    # Celebration/dejection contrast
    "BlueLockTome1-023.webp": ["sadness", "tension"],         # Coach speech is between sadness and climax
    "BlueLockTome1-024.webp": ["sadness"],                    # Coach crying — climax or sadness both valid
    "BlueLockTome1-025.webp": ["sadness"],                    # "Best team" tears — climax or sadness
    "BlueLockTome1-026.webp": ["peaceful"],                   # Walking home — sad but could be calm/reflective
    "BlueLockTome1-030.webp": ["mystery", "tension"],         # "What if" could be mystery or tension
    "BlueLockTome1-032.webp": ["mystery", "peaceful"],        # Existential doubt, walking scene
    "BlueLockTome1-033.webp": ["emotional_climax"],           # "I wanted to win!" is raw emotion — could be climax
    "BlueLockTome1-034.webp": ["comedy"],                     # Daily life could read as light comedy
    "BlueLockTome1-035.webp": ["tension"],                    # Shock could be tension
    "BlueLockTome1-036.webp": ["tension", "peaceful"],        # Arriving at building — anticipation or calm
}

# ============================================================
# Prompts
# ============================================================

PROMPT_SHORT = (
    "Analyze this manga page's emotional mood. Categories:\n"
    "- epic_battle: intense fighting, action clash, combat\n"
    "- tension: suspense, confrontation, anticipation, high stakes\n"
    "- sadness: grief, loss, defeat, crying, melancholy\n"
    "- comedy: humor, funny reactions, slapstick, chibi\n"
    "- romance: love, intimacy, tender moments\n"
    "- horror: fear, dark atmosphere, monsters, gore\n"
    "- peaceful: calm, daily life, relaxed conversation\n"
    "- emotional_climax: triumphant speech, breakthrough, peak emotion, resolve\n"
    "- mystery: enigmatic, questioning, investigation\n"
    "- chase_action: pursuit, running, escape, fast movement\n"
    "Reply with ONLY the category name."
)

PROMPT_VISUAL = (
    "Analyze this manga page's emotional mood.\n"
    "Focus on: facial expressions, body language, visual effects (speed lines, "
    "dark shading, screentones, sparkles), panel composition, and text intensity.\n\n"
    "Categories:\n"
    "- epic_battle: intense fighting, action clash, impact effects, combat poses\n"
    "- tension: suspense, confrontation, anticipation, high stakes standoff\n"
    "- sadness: grief, loss, defeat, crying, tears, melancholy, head down\n"
    "- comedy: humor, funny reactions, slapstick, chibi/super-deformed faces\n"
    "- romance: love, intimacy, tender moments, blushing\n"
    "- horror: fear, dark atmosphere, monsters, gore, distorted faces\n"
    "- peaceful: calm, daily life, relaxed conversation, normal expressions\n"
    "- emotional_climax: triumphant speech, breakthrough moment, intense resolve, "
    "motivational words, team rallying, peak emotion (NOT sadness if characters show determination)\n"
    "- mystery: enigmatic, questioning, investigation, hidden information\n"
    "- chase_action: pursuit, running, escape, fast movement, speed lines\n\n"
    "Reply with ONLY the category name."
)

# Context-aware prompt — tells the model about previous pages
PROMPT_CONTEXT = (
    "You are analyzing a sequence of manga pages. "
    "The previous page(s) are provided for context — they show what happened before this page.\n"
    "Analyze the LAST image's emotional mood (the current page being read).\n"
    "Focus on: facial expressions, body language, visual effects (speed lines, "
    "dark shading, screentones, sparkles), panel composition, and text intensity.\n\n"
    "Categories:\n"
    "- epic_battle: intense fighting, action clash, impact effects, combat poses\n"
    "- tension: suspense, confrontation, anticipation, high stakes standoff\n"
    "- sadness: grief, loss, defeat, crying, tears, melancholy, head down\n"
    "- comedy: humor, funny reactions, slapstick, chibi/super-deformed faces\n"
    "- romance: love, intimacy, tender moments, blushing\n"
    "- horror: fear, dark atmosphere, monsters, gore, distorted faces\n"
    "- peaceful: calm, daily life, relaxed conversation, normal expressions\n"
    "- emotional_climax: triumphant speech, breakthrough moment, intense resolve, "
    "motivational words, team rallying, peak emotion (NOT sadness if characters show determination)\n"
    "- mystery: enigmatic, questioning, investigation, hidden information\n"
    "- chase_action: pursuit, running, escape, fast movement, speed lines\n\n"
    "Reply with ONLY the category name for the LAST/current page."
)

PROMPTS = {
    "short": PROMPT_SHORT,
    "visual": PROMPT_VISUAL,
    "context": PROMPT_CONTEXT,
}

MOODS = [
    "epic_battle", "tension", "sadness", "comedy", "romance",
    "horror", "peaceful", "emotional_climax", "mystery", "chase_action",
]

# ============================================================
# Image resize
# ============================================================

def resize_image(img_path: str, max_size: int = 672) -> str:
    """Resize image to max_size on longest edge. Returns path to temp resized file."""
    img = Image.open(img_path)
    w, h = img.size

    if max(w, h) <= max_size:
        return img_path

    if w > h:
        new_w = max_size
        new_h = int(h * max_size / w)
    else:
        new_h = max_size
        new_w = int(w * max_size / h)

    resized = img.resize((new_w, new_h), Image.LANCZOS)

    temp_dir = Path("temp_resized_ctx")
    temp_dir.mkdir(exist_ok=True)
    out_path = temp_dir / f"{max_size}_{Path(img_path).stem}.jpg"

    if resized.mode != "RGB":
        resized = resized.convert("RGB")
    resized.save(str(out_path), "JPEG", quality=90)
    return str(out_path)


# ============================================================
# Parse mood from response
# ============================================================

def parse_mood(raw: str) -> str:
    import re
    clean = re.sub(r'<think>.*?</think>', '', raw, flags=re.DOTALL).strip()
    if not clean:
        clean = raw.strip()

    lower = clean.lower()
    for mood in MOODS:
        if mood in lower:
            return mood

    return f"?({clean[:30]})"


# ============================================================
# GPU monitoring
# ============================================================

def _get_gpu_stats():
    try:
        import pynvml
        handle = pynvml.nvmlDeviceGetHandleByIndex(0)
        mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
        util = pynvml.nvmlDeviceGetUtilizationRates(handle)
        return mem.used / 1024**2, mem.total / 1024**2, util.gpu
    except Exception:
        return 0, 0, 0


class PerfMonitor:
    def __init__(self):
        self.samples = []
        self._running = False
        self._thread = None
        try:
            import pynvml
            pynvml.nvmlInit()
            self.has_gpu = True
        except Exception:
            self.has_gpu = False

    def start(self):
        self.samples = []
        self._running = True
        self._thread = threading.Thread(target=self._loop, daemon=True)
        self._thread.start()

    def stop(self):
        self._running = False
        if self._thread:
            self._thread.join(timeout=1)
        if not self.samples:
            return {}
        vram_vals = [s.get("vram", 0) for s in self.samples]
        return {
            "vram_peak_mb": max(vram_vals) if vram_vals else 0,
            "vram_total_mb": self.samples[0].get("vram_total", 0),
        }

    def _loop(self):
        while self._running:
            s = {}
            if self.has_gpu:
                vram, total, gpu = _get_gpu_stats()
                s["vram"] = vram
                s["vram_total"] = total
            self.samples.append(s)
            time.sleep(0.2)


# ============================================================
# Run one context configuration
# ============================================================

def run_context_test(
    context_size: int,
    prompt_name: str,
    images: list[tuple[str, str]],  # (filename, full_path)
    resize: int = 672,
) -> dict:
    """
    Run sequential mood detection with N pages of context.

    For context_size=0: just the current page (baseline)
    For context_size=N: previous N pages + current page as images
    """
    # Select prompt: use context-aware prompt when context > 0
    if context_size > 0:
        prompt = PROMPT_CONTEXT
    else:
        prompt = PROMPTS.get(prompt_name, PROMPT_VISUAL)

    label = f"context={context_size}" + (f" ({prompt_name})" if context_size == 0 else "")
    console.print(f"\n[bold cyan]{'━' * 60}[/bold cyan]")
    console.print(f"[bold cyan]  {label} — {len(images)} pages[/bold cyan]")
    console.print(f"[bold cyan]{'━' * 60}[/bold cyan]")

    # Pre-resize all images
    resized_paths = {}
    for filename, full_path in images:
        resized_paths[filename] = resize_image(full_path, resize)

    perf = PerfMonitor()
    perf.start()

    results = []
    total_time = 0

    for i, (filename, full_path) in enumerate(images):
        # Build image list: context pages + current page
        image_paths = []

        if context_size > 0:
            # Add previous N pages as context
            start_ctx = max(0, i - context_size)
            for j in range(start_ctx, i):
                ctx_filename = images[j][0]
                image_paths.append(resized_paths[ctx_filename])

        # Current page is always last
        image_paths.append(resized_paths[filename])

        n_ctx_used = len(image_paths) - 1  # how many context images

        options = {
            "temperature": 0.1,
            "num_predict": 5000,
        }

        chat_kwargs = {
            "model": MODEL,
            "messages": [{
                "role": "user",
                "content": prompt,
                "images": image_paths,
            }],
            "options": options,
        }

        t0 = time.time()
        try:
            resp = ollama.chat(**chat_kwargs)
            elapsed = (time.time() - t0) * 1000
            raw = resp["message"]["content"]
            mood = parse_mood(raw)
        except Exception as e:
            elapsed = (time.time() - t0) * 1000
            mood = f"ERR({str(e)[:30]})"

        total_time += elapsed

        # Check correctness
        expected = SEQUENCE_GROUND_TRUTH.get(filename, "?")
        alts = SEQUENCE_ACCEPTABLE_ALT.get(filename, [])
        correct = mood == expected or mood in alts

        results.append({
            "image": filename,
            "mood": mood,
            "expected": expected,
            "correct": correct,
            "time_ms": round(elapsed, 1),
            "context_images": n_ctx_used,
        })

        # Print inline
        status = "[green]OK[/green]" if correct else "[red]MISS[/red]"
        page_num = filename.replace("BlueLockTome1-", "").replace(".webp", "")
        ctx_info = f"[dim](+{n_ctx_used}ctx)[/dim]" if n_ctx_used > 0 else ""
        console.print(
            f"  p{page_num} {ctx_info:<14} {mood:<22} expected: {expected:<22} "
            f"{status}  {elapsed:.0f}ms"
        )

    perf_stats = perf.stop()

    # Summary
    correct_count = sum(1 for r in results if r["correct"])
    total = len(results)
    avg_time = total_time / total if total else 0

    console.print(
        f"\n  [bold]Score: {correct_count}/{total} ({correct_count / total * 100:.0f}%)[/bold]  "
        f"Avg: {avg_time:.0f}ms  Total: {total_time / 1000:.1f}s"
    )
    if perf_stats.get("vram_peak_mb"):
        vram_pct = perf_stats["vram_peak_mb"] / perf_stats["vram_total_mb"] * 100
        console.print(f"  VRAM peak: {perf_stats['vram_peak_mb']:.0f} MB ({vram_pct:.0f}%)")

    return {
        "label": label,
        "context_size": context_size,
        "results": results,
        "score": correct_count,
        "total": total,
        "avg_time_ms": round(avg_time, 1),
        "total_time_s": round(total_time / 1000, 1),
        "perf": perf_stats,
    }


# ============================================================
# Mood transition analysis
# ============================================================

def analyze_transitions(all_runs: dict):
    """Show where mood transitions happen vs ground truth."""
    console.print("\n")
    console.print(Panel.fit(
        "[bold]Mood Transition Analysis[/bold]\n"
        "Shows where the model detects mood changes vs ground truth",
        border_style="magenta",
    ))

    images = list(SEQUENCE_GROUND_TRUTH.keys())

    table = Table(box=box.SIMPLE, show_lines=False, padding=(0, 1))
    table.add_column("Page", style="dim", width=6)
    table.add_column("GT Mood", style="bold", min_width=18)
    table.add_column("GT Change", width=10)

    for ctx_label in sorted(all_runs.keys()):
        table.add_column(f"ctx={ctx_label}", min_width=18)
        table.add_column("Chg", width=5)

    prev_gt = None
    for i, img in enumerate(images):
        gt_mood = SEQUENCE_GROUND_TRUTH[img]
        page = img.replace("BlueLockTome1-", "").replace(".webp", "")
        gt_changed = gt_mood != prev_gt if prev_gt else False
        gt_change_str = "[yellow]>>>[/yellow]" if gt_changed else ""

        row = [page, gt_mood, gt_change_str]

        for ctx_label in sorted(all_runs.keys()):
            run = all_runs[ctx_label]
            result = next((r for r in run["results"] if r["image"] == img), None)
            if result:
                mood = result["mood"]
                correct = result["correct"]

                # Check if this is a transition from previous
                prev_result = next(
                    (r for r in run["results"] if r["image"] == images[i - 1]),
                    None
                ) if i > 0 else None
                prev_mood = prev_result["mood"] if prev_result else None
                changed = mood != prev_mood if prev_mood else False

                mood_str = f"[green]{mood}[/green]" if correct else f"[red]{mood}[/red]"
                change_str = "[yellow]>>>[/yellow]" if changed else ""
                row.extend([mood_str, change_str])
            else:
                row.extend(["—", ""])

        table.add_row(*row)
        prev_gt = gt_mood

    console.print(table)

    # Count transition accuracy
    console.print("\n[bold]Transition Detection:[/bold]")
    gt_transitions = 0
    prev_gt = None
    gt_transition_pages = []
    for img in images:
        gt_mood = SEQUENCE_GROUND_TRUTH[img]
        if prev_gt and gt_mood != prev_gt:
            gt_transitions += 1
            gt_transition_pages.append(img)
        prev_gt = gt_mood

    console.print(f"  Ground truth has [bold]{gt_transitions}[/bold] mood transitions")

    for ctx_label in sorted(all_runs.keys()):
        run = all_runs[ctx_label]
        model_transitions = 0
        correct_transitions = 0
        prev_mood = None
        for i, img in enumerate(images):
            result = next((r for r in run["results"] if r["image"] == img), None)
            if result:
                mood = result["mood"]
                if prev_mood and mood != prev_mood:
                    model_transitions += 1
                    # Check if GT also has a transition here
                    if img in gt_transition_pages:
                        correct_transitions += 1
                prev_mood = mood

        console.print(
            f"  ctx={ctx_label}: {model_transitions} transitions detected, "
            f"{correct_transitions}/{gt_transitions} match GT transitions"
        )


# ============================================================
# Main
# ============================================================

def main():
    parser = argparse.ArgumentParser(description="Context-based sequential benchmark")
    parser.add_argument("--context", nargs="+", type=int, default=[0, 1, 2, 3],
                        help="Context sizes to test (default: 0 1 2 3)")
    parser.add_argument("--prompt", choices=["short", "visual"], default="visual",
                        help="Prompt for context=0 baseline (context>0 always uses context prompt)")
    parser.add_argument("--resize", type=int, default=672,
                        help="Max image dimension (default: 672)")
    args = parser.parse_args()

    console.print(Panel.fit(
        "[bold]Manga Mood AI — Context-Based Sequential Benchmark[/bold]\n"
        f"Model: {MODEL}\n"
        f"Sequence: Blue Lock Tome 1 (pages 6-36, {len(SEQUENCE_GROUND_TRUTH)} pages)\n"
        f"Context sizes: {args.context}\n"
        f"Baseline prompt: {args.prompt} | Resize: {args.resize}px",
        border_style="cyan",
    ))

    # Collect images in order
    images = []
    for filename in SEQUENCE_GROUND_TRUTH.keys():
        full_path = SEQUENCE_DIR / filename
        if full_path.exists():
            images.append((filename, str(full_path)))
        else:
            console.print(f"[yellow]Warning: {filename} not found[/yellow]")

    console.print(f"\nFound [bold]{len(images)}[/bold] / {len(SEQUENCE_GROUND_TRUTH)} sequence images")

    # Check model
    try:
        ollama.show(MODEL)
        console.print(f"[green]Model {MODEL} ready[/green]")
    except Exception:
        console.print(f"[red]Model {MODEL} not found. Run: ollama pull {MODEL}[/red]")
        return

    # Run each context size
    all_runs = {}
    for ctx_size in args.context:
        run_data = run_context_test(
            context_size=ctx_size,
            prompt_name=args.prompt,
            images=images,
            resize=args.resize,
        )
        all_runs[ctx_size] = run_data

    # ── Comparison table ──
    console.print("\n")
    table = Table(
        title="Context Comparison — Sequential Blue Lock",
        box=box.ROUNDED,
        show_lines=True,
    )
    table.add_column("Config", style="bold", min_width=20)
    table.add_column("Score", justify="center", min_width=14)
    table.add_column("Avg Time", justify="right")
    table.add_column("Total Time", justify="right")
    table.add_column("Slowdown", justify="right")
    table.add_column("Misses", style="dim", max_width=50)

    baseline_avg = all_runs.get(0, {}).get("avg_time_ms", 1)

    for ctx_size in args.context:
        run = all_runs[ctx_size]
        score = run["score"]
        total = run["total"]
        pct = score / total * 100
        avg = run["avg_time_ms"]
        total_time = run["total_time_s"]

        slowdown = avg / baseline_avg if baseline_avg > 0 else 0
        slowdown_str = f"{slowdown:.1f}x" if ctx_size > 0 else "—"

        misses = [
            r["image"].replace("BlueLockTome1-", "").replace(".webp", "")
            for r in run["results"]
            if not r["correct"]
        ]

        if pct >= 80:
            score_style = "bold green"
        elif pct >= 65:
            score_style = "bold yellow"
        else:
            score_style = "bold red"

        table.add_row(
            run["label"],
            f"[{score_style}]{score}/{total} ({pct:.0f}%)[/{score_style}]",
            f"{avg:.0f}ms",
            f"{total_time:.1f}s",
            slowdown_str,
            ", ".join(misses) if misses else "—",
        )

    console.print(table)

    # ── Per-page detail ──
    console.print("\n")
    detail = Table(
        title="Per-Page Detail",
        box=box.ROUNDED,
        show_lines=True,
    )
    detail.add_column("Page", style="dim", width=6)
    detail.add_column("Expected", style="bold", min_width=18)

    for ctx_size in args.context:
        detail.add_column(f"ctx={ctx_size}", min_width=22)

    for filename in SEQUENCE_GROUND_TRUTH.keys():
        expected = SEQUENCE_GROUND_TRUTH[filename]
        page = filename.replace("BlueLockTome1-", "").replace(".webp", "")
        row = [page, expected]

        for ctx_size in args.context:
            run = all_runs[ctx_size]
            result = next((r for r in run["results"] if r["image"] == filename), None)
            if result:
                mood = result["mood"]
                ms = result["time_ms"]
                if result["correct"]:
                    row.append(f"[green]{mood}[/green] ({ms:.0f}ms)")
                else:
                    row.append(f"[red]{mood}[/red] ({ms:.0f}ms)")
            else:
                row.append("—")

        detail.add_row(*row)

    console.print(detail)

    # ── Transition analysis ──
    analyze_transitions(all_runs)

    # Cleanup
    temp_dir = Path("temp_resized_ctx")
    if temp_dir.exists():
        import shutil
        shutil.rmtree(temp_dir, ignore_errors=True)
        console.print("\n[dim]Cleaned up temp resized images[/dim]")

    # Save results
    results_path = Path("results/context_benchmark.json")
    results_path.parent.mkdir(exist_ok=True)
    save_data = {}
    for ctx_size in args.context:
        run = all_runs[ctx_size]
        save_data[f"context_{ctx_size}"] = {
            "label": run["label"],
            "context_size": ctx_size,
            "score": run["score"],
            "total": run["total"],
            "avg_time_ms": run["avg_time_ms"],
            "total_time_s": run["total_time_s"],
            "results": run["results"],
        }
    with open(results_path, "w", encoding="utf-8") as f:
        json.dump(save_data, f, indent=2, ensure_ascii=False)
    console.print(f"[dim]Results saved to {results_path}[/dim]")


if __name__ == "__main__":
    main()
