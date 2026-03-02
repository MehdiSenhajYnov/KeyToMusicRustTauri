"""
Manga Mood AI — Test all models on manga/webtoon images.

Usage:
    # Activate venv first:
    # ./venv/Scripts/activate  (Windows)

    # Test all models on all images:
    python test_models.py

    # Test a specific model:
    python test_models.py --model siglip
    python test_models.py --model moondream
    python test_models.py --model qwen2b
    python test_models.py --model qwen7b
    python test_models.py --model md2         # Moondream 2 latest (4bit, transformers)
    python test_models.py --model md3         # Moondream 3 Preview (4bit bnb, transformers)
    python test_models.py --model minicpm     # MiniCPM-V 4.0 (Ollama)

    # Test on a single image:
    python test_models.py --image test-images/manga/battle.jpg

    # Test with multi-image context (VLMs only, sends previous images as context):
    python test_models.py --model qwen7b --context 3

    # Custom prompt for VLMs:
    python test_models.py --model qwen7b --custom-prompt "What emotion does this scene convey?"
"""

import argparse
import json
import os
import sys
import time
import threading
from pathlib import Path

from PIL import Image
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
from rich.text import Text
from rich import box

console = Console()


# ============================================================
# Performance monitoring
# ============================================================

def _init_gpu_monitor():
    """Try to init NVML for GPU monitoring. Returns True if available."""
    try:
        import pynvml
        pynvml.nvmlInit()
        return True
    except Exception:
        return False

def _get_gpu_stats():
    """Get current GPU usage. Returns (vram_used_mb, vram_total_mb, gpu_util_percent)."""
    try:
        import pynvml
        handle = pynvml.nvmlDeviceGetHandleByIndex(0)
        mem = pynvml.nvmlDeviceGetMemoryInfo(handle)
        util = pynvml.nvmlDeviceGetUtilizationRates(handle)
        return mem.used / 1024**2, mem.total / 1024**2, util.gpu
    except Exception:
        return 0, 0, 0

def _get_cpu_ram_stats():
    """Get current CPU and RAM usage."""
    import psutil
    return psutil.cpu_percent(interval=None), psutil.virtual_memory().used / 1024**2

class PerfMonitor:
    """Background thread that samples CPU/RAM/GPU every 200ms during inference."""

    def __init__(self, has_gpu: bool):
        self.has_gpu = has_gpu
        self.samples = []
        self._running = False
        self._thread = None
        # Warm up psutil cpu_percent (first call always returns 0)
        import psutil
        psutil.cpu_percent(interval=None)

    def measure_baseline(self, duration: float = 2.0):
        """Measure idle stats for `duration` seconds."""
        import psutil
        cpu_samples = []
        ram_samples = []
        gpu_util_samples = []
        vram_samples = []

        end = time.time() + duration
        while time.time() < end:
            cpu_samples.append(psutil.cpu_percent(interval=None))
            ram_samples.append(psutil.virtual_memory().used / 1024**2)
            if self.has_gpu:
                vram, _, gpu_util = _get_gpu_stats()
                vram_samples.append(vram)
                gpu_util_samples.append(gpu_util)
            time.sleep(0.2)

        self.baseline = {
            "cpu": sum(cpu_samples) / len(cpu_samples) if cpu_samples else 0,
            "ram_mb": sum(ram_samples) / len(ram_samples) if ram_samples else 0,
            "gpu_util": sum(gpu_util_samples) / len(gpu_util_samples) if gpu_util_samples else 0,
            "vram_mb": sum(vram_samples) / len(vram_samples) if vram_samples else 0,
        }
        return self.baseline

    def start(self):
        """Start sampling in background."""
        self.samples = []
        self._running = True
        self._thread = threading.Thread(target=self._sample_loop, daemon=True)
        self._thread.start()

    def stop(self):
        """Stop sampling and return stats."""
        self._running = False
        if self._thread:
            self._thread.join(timeout=1)
        return self._compute_stats()

    def _sample_loop(self):
        import psutil
        while self._running:
            sample = {
                "cpu": psutil.cpu_percent(interval=None),
                "ram_mb": psutil.virtual_memory().used / 1024**2,
            }
            if self.has_gpu:
                vram, vram_total, gpu_util = _get_gpu_stats()
                sample["gpu_util"] = gpu_util
                sample["vram_mb"] = vram
                sample["vram_total_mb"] = vram_total
            self.samples.append(sample)
            time.sleep(0.2)

    def _compute_stats(self):
        if not self.samples:
            return None
        baseline = getattr(self, "baseline", {})

        cpu_vals = [s["cpu"] for s in self.samples]
        ram_vals = [s["ram_mb"] for s in self.samples]

        stats = {
            "cpu_avg": sum(cpu_vals) / len(cpu_vals),
            "cpu_peak": max(cpu_vals),
            "cpu_delta": sum(cpu_vals) / len(cpu_vals) - baseline.get("cpu", 0),
            "ram_avg_mb": sum(ram_vals) / len(ram_vals),
            "ram_peak_mb": max(ram_vals),
            "ram_delta_mb": max(ram_vals) - baseline.get("ram_mb", 0),
        }

        if self.has_gpu:
            gpu_vals = [s["gpu_util"] for s in self.samples]
            vram_vals = [s["vram_mb"] for s in self.samples]
            stats["gpu_avg"] = sum(gpu_vals) / len(gpu_vals)
            stats["gpu_peak"] = max(gpu_vals)
            stats["gpu_delta"] = sum(gpu_vals) / len(gpu_vals) - baseline.get("gpu_util", 0)
            stats["vram_avg_mb"] = sum(vram_vals) / len(vram_vals)
            stats["vram_peak_mb"] = max(vram_vals)
            stats["vram_delta_mb"] = max(vram_vals) - baseline.get("vram_mb", 0)
            if self.samples[0].get("vram_total_mb"):
                stats["vram_total_mb"] = self.samples[0]["vram_total_mb"]

        return stats

# ============================================================
# Ground truth
# ============================================================

GROUND_TRUTH = {
    "1.jpg":   "epic_battle",
    "10.jpg":  "emotional_climax",
    "11.jpg":  "emotional_climax",
    "12.jpg":  "emotional_climax",
    "13.png":  "sadness",
    "14.png":  "emotional_climax",
    "15.png":  "emotional_climax",
    "16.png":  "sadness",
    "17.png":  "sadness",
    "18.png":  "peaceful",
    "2.jpeg":  "tension",
    "4.jpg":   "tension",
    "5.png":   "tension",
    "6.jpg":   "romance",
    "7.jpg":   "sadness",
    "8.jpg":   "horror",
    "3.jpeg":  "epic_battle",
    "9.png":   "horror",
}

ACCEPTABLE_ALT = {
    "5.png":   ["emotional_climax"],
    "10.jpg":  ["tension"],
    "3.jpeg":  ["tension"],
    "13.png":  ["emotional_climax"],
    "14.png":  ["sadness"],
    "15.png":  ["sadness"],
    "16.png":  ["peaceful"],
    "17.png":  ["emotional_climax"],
}

def _resolve_image_name(image_name: str) -> str:
    """Resolve a potentially resized image name back to its original."""
    return _RESIZED_NAME_MAP.get(image_name, image_name)

def _is_correct(image_name: str, predicted: str) -> bool:
    name = _resolve_image_name(image_name)
    expected = GROUND_TRUTH.get(name)
    if expected is None:
        return False
    if predicted == expected:
        return True
    return predicted in ACCEPTABLE_ALT.get(name, [])

# ============================================================
# Image resize helper
# ============================================================

# Global map: resized basename -> original basename (populated by resize_image)
_RESIZED_NAME_MAP: dict[str, str] = {}

def resize_image(img_path: str, max_size: int) -> str:
    """Resize image to max_size on longest edge. Returns path to temp resized file."""
    img = Image.open(img_path)
    w, h = img.size
    original_basename = os.path.basename(img_path)

    if max(w, h) <= max_size:
        return img_path

    if w > h:
        new_w = max_size
        new_h = int(h * max_size / w)
    else:
        new_h = max_size
        new_w = int(w * max_size / h)

    resized = img.resize((new_w, new_h), Image.LANCZOS)

    temp_dir = Path("temp_resized")
    temp_dir.mkdir(exist_ok=True)
    out_path = temp_dir / f"{max_size}_{Path(img_path).name}"

    if resized.mode != "RGB":
        resized = resized.convert("RGB")
    out_path = out_path.with_suffix(".jpg")
    resized.save(str(out_path), "JPEG", quality=90)

    # Track name mapping for ground truth scoring
    _RESIZED_NAME_MAP[os.path.basename(str(out_path))] = original_basename

    return str(out_path)

# ============================================================
# Mood categories
# ============================================================

MOOD_CATEGORIES = {
    "epic_battle":      "Intense combat, action, explosions, fighting",
    "tension":          "Suspense, confrontation, threat, menace",
    "sadness":          "Grief, loss, melancholy, crying, tears",
    "comedy":           "Humor, gags, lighthearted, chibi, funny",
    "romance":          "Romantic moments, intimacy, blushing, love",
    "horror":           "Fear, dread, disturbing, creepy, monster",
    "peaceful":         "Calm, daily life, nature, relaxing, slice of life",
    "emotional_climax": "Peak emotional moment, intense joy or pain, dramatic",
    "mystery":          "Mystery, revelation, intrigue, shadow, enigma",
    "chase_action":     "Chase, fast movement, urgency, speed lines",
}

# SigLIP text descriptions (richer, for embedding similarity)
SIGLIP_MOOD_TEXTS = {
    "epic_battle":      "manga anime epic battle fight combat explosion action scene with speed lines and impact effects",
    "tension":          "manga anime tense suspenseful confrontation threatening dark dramatic scene with close-up faces",
    "sadness":          "manga anime sad crying tears melancholy lonely grieving emotional scene with rain or darkness",
    "comedy":           "manga anime funny comedy humor chibi super-deformed exaggerated expressions laughing scene",
    "romance":          "manga anime romantic love intimate blushing gentle soft scene with flowers or sparkles",
    "horror":           "manga anime horror scary dark creepy monster disturbing grotesque terrifying scene",
    "peaceful":         "manga anime peaceful calm quiet daily life nature landscape relaxing warm scene",
    "emotional_climax": "manga anime emotional intense dramatic climax powerful splash page overwhelming feelings scene",
    "mystery":          "manga anime mysterious dark shadow silhouette enigmatic hidden revelation suspicion scene",
    "chase_action":     "manga anime chase running speed motion fast pursuit urgent movement speed lines scene",
}

# VLM prompt (standard — for Qwen2.5-VL 7B and similar)
VLM_PROMPT = """Analyze this manga/webtoon page and classify its mood/atmosphere.

Choose ONE primary mood from this list:
- epic_battle: Intense combat, action, explosions
- tension: Suspense, confrontation, threat
- sadness: Grief, loss, melancholy, crying
- comedy: Humor, gags, lighthearted, chibi
- romance: Romantic moments, intimacy
- horror: Fear, dread, disturbing imagery
- peaceful: Calm, daily life, slice of life
- emotional_climax: Peak emotional moment (intense joy or pain)
- mystery: Mystery, revelation, intrigue
- chase_action: Chase, fast movement, urgency

Respond ONLY with this JSON (no other text):
{"mood": "...", "intensity": <1-5>, "secondary_mood": "...", "reason": "<10 words max>"}"""

# Compact prompt — works with all models (Gemma bugs on multi-line list format)
VLM_PROMPT_SHORT = """Classify this manga page's emotional mood as ONE of: epic_battle, tension, sadness, comedy, romance, horror, peaceful, emotional_climax, mystery, chase_action. Reply with ONLY the category name, nothing else."""

VLM_CONTEXT_PROMPT = """I'm reading a manga/webtoon. The previous images are context pages I already read.
The LAST image is the current page I want classified.

Based on the visual style, composition, and scene of the CURRENT (last) page, classify its mood.
Consider the previous pages as narrative context.

Choose ONE primary mood from this list:
- epic_battle: Intense combat, action, explosions
- tension: Suspense, confrontation, threat
- sadness: Grief, loss, melancholy, crying
- comedy: Humor, gags, lighthearted, chibi
- romance: Romantic moments, intimacy
- horror: Fear, dread, disturbing imagery
- peaceful: Calm, daily life, slice of life
- emotional_climax: Peak emotional moment (intense joy or pain)
- mystery: Mystery, revelation, intrigue
- chase_action: Chase, fast movement, urgency

Respond ONLY with this JSON (no other text):
{"mood": "...", "intensity": <1-5>, "secondary_mood": "...", "reason": "<10 words max>"}"""


# ============================================================
# SigLIP 2 test
# ============================================================

def test_siglip(image_paths: list[str]) -> list[dict]:
    """Test SigLIP 2 ViT-B on images using zero-shot classification."""
    console.print("\n[bold cyan]━━━ SigLIP 2 ViT-B (86M, embedding classification) ━━━[/bold cyan]")

    import torch
    from transformers import AutoProcessor, AutoModel

    model_name = "google/siglip2-base-patch16-224"
    console.print(f"Loading model: {model_name} ...")

    load_start = time.time()
    processor = AutoProcessor.from_pretrained(model_name)
    model = AutoModel.from_pretrained(model_name)

    device = "cuda" if torch.cuda.is_available() else "cpu"
    model = model.to(device)
    model.eval()
    load_time = time.time() - load_start
    console.print(f"Model loaded on [bold]{device}[/bold] in {load_time:.1f}s")

    # Pre-encode mood texts
    mood_names = list(SIGLIP_MOOD_TEXTS.keys())
    mood_texts = list(SIGLIP_MOOD_TEXTS.values())

    results = []
    for img_path in image_paths:
        img = Image.open(img_path).convert("RGB")

        inputs = processor(
            text=mood_texts,
            images=img,
            return_tensors="pt",
            padding=True,
            truncation=True,
        ).to(device)

        infer_start = time.time()
        with torch.no_grad():
            outputs = model(**inputs)
            logits = outputs.logits_per_image[0]
            # Use softmax for relative ranking (sigmoid gives near-0 for all)
            probs = torch.softmax(logits, dim=0)
        infer_time = (time.time() - infer_start) * 1000  # ms

        # Sort by probability
        sorted_indices = probs.argsort(descending=True)
        top_moods = []
        for idx in sorted_indices[:3]:
            top_moods.append({
                "mood": mood_names[idx],
                "score": round(probs[idx].item() * 100, 1),  # percentage
            })

        result = {
            "model": "SigLIP 2 ViT-B",
            "image": os.path.basename(img_path),
            "mood": top_moods[0]["mood"],
            "confidence": top_moods[0]["score"],
            "top_3": top_moods,
            "time_ms": round(infer_time, 1),
            "device": device,
        }
        results.append(result)
        _print_result(result)

    return results


# ============================================================
# Ollama VLM tests
# ============================================================

def test_ollama_vlm(
    model_name: str,
    display_name: str,
    image_paths: list[str],
    context_pages: int = 0,
    custom_prompt: str | None = None,
    prompt_type: str = "standard",
    num_predict: int = 300,
    perf_monitor: PerfMonitor | None = None,
) -> list[dict]:
    """Test an Ollama VLM model on images."""
    console.print(f"\n[bold cyan]━━━ {display_name} (Ollama VLM) ━━━[/bold cyan]")

    import ollama

    # Check model is available, auto-pull if missing
    try:
        ollama.show(model_name)
        console.print(f"Model [bold]{model_name}[/bold] ready")
    except Exception:
        console.print(f"[yellow]Model {model_name} not found locally, pulling...[/yellow]")
        try:
            for progress in ollama.pull(model_name, stream=True):
                status = progress.get("status", "")
                total = progress.get("total") or 0
                completed = progress.get("completed") or 0
                if total > 0:
                    pct = completed / total * 100
                    console.print(f"\r  [dim]{status}: {pct:.0f}% ({completed/1024**3:.1f}/{total/1024**3:.1f} GB)[/dim]", end="")
                elif status:
                    console.print(f"\r  [dim]{status}[/dim]", end="")
            console.print(f"\n[green]Model {model_name} pulled successfully[/green]")
        except Exception as e:
            console.print(f"\n[red]Failed to pull {model_name}: {e}[/red]")
            return []

    results = []

    # Start perf monitoring for the whole model run
    if perf_monitor:
        perf_monitor.start()

    for i, img_path in enumerate(image_paths):
        # Build image list (with context if requested)
        images_to_send = []
        if context_pages > 0 and not custom_prompt:
            start = max(0, i - context_pages)
            for j in range(start, i):
                images_to_send.append(image_paths[j])

        images_to_send.append(img_path)

        # Select prompt based on model config
        if custom_prompt:
            prompt = custom_prompt
        elif context_pages > 0:
            prompt = VLM_CONTEXT_PROMPT
        else:
            prompt = VLM_PROMPT_SHORT

        infer_start = time.time()
        try:
            response = ollama.chat(
                model=model_name,
                messages=[{
                    "role": "user",
                    "content": prompt,
                    "images": images_to_send,
                }],
                options={
                    "temperature": 0.1,
                    "num_predict": num_predict,
                },
            )
            infer_time = (time.time() - infer_start) * 1000

            raw_response = response["message"]["content"]
            parsed = _parse_vlm_response(raw_response)

            try:
                intensity = float(parsed.get("intensity", 0))
            except (ValueError, TypeError):
                intensity = 0

            result = {
                "model": display_name,
                "image": os.path.basename(img_path),
                "mood": parsed.get("mood", "unknown"),
                "confidence": intensity / 5.0 * 100,
                "intensity": intensity,
                "secondary_mood": parsed.get("secondary_mood", ""),
                "reason": parsed.get("reason", ""),
                "time_ms": round(infer_time, 1),
                "raw_response": raw_response,
                "context_pages": context_pages,
            }
        except Exception as e:
            infer_time = (time.time() - infer_start) * 1000
            result = {
                "model": display_name,
                "image": os.path.basename(img_path),
                "mood": "ERROR",
                "confidence": 0,
                "time_ms": round(infer_time, 1),
                "raw_response": str(e),
                "context_pages": context_pages,
            }

        results.append(result)
        _print_result(result)

    # Stop perf monitoring and attach stats to results
    perf_stats = None
    if perf_monitor:
        perf_stats = perf_monitor.stop()
        if perf_stats:
            _print_perf_stats(display_name, perf_stats)
            for r in results:
                r["perf"] = perf_stats

    return results


def _parse_vlm_response(raw: str) -> dict:
    """Try to parse JSON from VLM response (they sometimes add extra text)."""
    # Try direct JSON parse
    try:
        return json.loads(raw.strip())
    except json.JSONDecodeError:
        pass

    # Try to find JSON in the response
    import re
    json_match = re.search(r'\{[^}]+\}', raw)
    if json_match:
        try:
            return json.loads(json_match.group())
        except json.JSONDecodeError:
            pass

    # Last resort: try to find mood keyword
    for mood in MOOD_CATEGORIES:
        if mood in raw.lower():
            return {"mood": mood, "intensity": 3, "reason": "extracted from text"}

    return {"mood": "unknown", "intensity": 0, "reason": raw[:100]}


# ============================================================
# Moondream (HuggingFace Transformers) tests
# ============================================================

def test_moondream_tf(
    hf_model_name: str,
    display_name: str,
    image_paths: list[str],
    use_bnb_4bit: bool = False,
    perf_monitor: PerfMonitor | None = None,
) -> list[dict]:
    """Test a Moondream model loaded via HuggingFace transformers."""
    console.print(f"\n[bold cyan]━━━ {display_name} (Transformers) ━━━[/bold cyan]")

    import torch
    from transformers import AutoModelForCausalLM

    device = "cuda" if torch.cuda.is_available() else "cpu"
    dl_hint = "~10GB" if use_bnb_4bit else "~2.5GB"
    console.print(f"Loading [bold]{hf_model_name}[/bold] (first run downloads {dl_hint})...")

    load_start = time.time()
    try:
        if use_bnb_4bit:
            from transformers import BitsAndBytesConfig
            bnb_config = BitsAndBytesConfig(
                load_in_4bit=True,
                bnb_4bit_quant_type="nf4",
                bnb_4bit_use_double_quant=True,
                bnb_4bit_compute_dtype=torch.bfloat16,
            )
            model = AutoModelForCausalLM.from_pretrained(
                hf_model_name,
                trust_remote_code=True,
                quantization_config=bnb_config,
                device_map={"": device},
            )
            # Try torch.compile (may fail on Windows without triton)
            try:
                model.compile()
                console.print("[dim]torch.compile() OK[/dim]")
            except Exception as e:
                console.print(f"[dim yellow]torch.compile() skipped (OK on Windows): {str(e)[:80]}[/dim yellow]")
        else:
            model = AutoModelForCausalLM.from_pretrained(
                hf_model_name,
                trust_remote_code=True,
                torch_dtype=torch.bfloat16,
                device_map={"": device},
            )

        load_time = time.time() - load_start
        console.print(f"Model loaded on [bold]{device}[/bold] in {load_time:.1f}s")
    except Exception as e:
        console.print(f"[red]Failed to load model: {e}[/red]")
        console.print("[dim]If out-of-memory, this model may need more VRAM than available.[/dim]")
        return []

    prompt = ("Analyze this manga page's emotional mood. Categories:\n"
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
              "Reply with ONLY the category name.")

    results = []

    if perf_monitor:
        perf_monitor.start()

    for img_path in image_paths:
        img = Image.open(img_path).convert("RGB")

        infer_start = time.time()
        try:
            answer_data = model.query(image=img, question=prompt, reasoning=False)
            raw_response = answer_data.get("answer", str(answer_data)) if isinstance(answer_data, dict) else str(answer_data)
            infer_time = (time.time() - infer_start) * 1000

            parsed = _parse_vlm_response(raw_response)

            result = {
                "model": display_name,
                "image": os.path.basename(img_path),
                "mood": parsed.get("mood", "unknown"),
                "confidence": parsed.get("intensity", 0) / 5.0 * 100,
                "reason": parsed.get("reason", ""),
                "time_ms": round(infer_time, 1),
                "raw_response": raw_response,
            }
        except Exception as e:
            infer_time = (time.time() - infer_start) * 1000
            result = {
                "model": display_name,
                "image": os.path.basename(img_path),
                "mood": "ERROR",
                "confidence": 0,
                "time_ms": round(infer_time, 1),
                "raw_response": str(e),
            }

        results.append(result)
        _print_result(result)

    perf_stats = None
    if perf_monitor:
        perf_stats = perf_monitor.stop()
        if perf_stats:
            _print_perf_stats(display_name, perf_stats)
            for r in results:
                r["perf"] = perf_stats

    # Free GPU memory for next model
    try:
        del model
        torch.cuda.empty_cache()
        console.print("[dim]GPU memory freed[/dim]")
    except Exception:
        pass

    return results


# ============================================================
# Display helpers
# ============================================================

MOOD_COLORS = {
    "epic_battle": "red",
    "tension": "dark_orange",
    "sadness": "blue",
    "comedy": "yellow",
    "romance": "magenta",
    "horror": "dark_red",
    "peaceful": "green",
    "emotional_climax": "bright_magenta",
    "mystery": "purple",
    "chase_action": "orange1",
}

def _print_perf_stats(model_name: str, stats: dict):
    """Print performance stats for a model run."""
    console.print(f"\n  [bold yellow]⚡ Performance: {model_name}[/bold yellow]")

    cpu_line = f"    CPU:  avg {stats['cpu_avg']:.1f}%  peak {stats['cpu_peak']:.1f}%  (delta +{stats['cpu_delta']:.1f}%)"
    ram_delta = stats['ram_delta_mb']
    ram_line = f"    RAM:  peak +{ram_delta:.0f} MB"
    console.print(cpu_line)
    console.print(ram_line)

    if "gpu_avg" in stats:
        gpu_line = f"    GPU:  avg {stats['gpu_avg']:.1f}%  peak {stats['gpu_peak']:.1f}%  (delta +{stats['gpu_delta']:.1f}%)"
        vram_peak = stats['vram_peak_mb']
        vram_delta = stats['vram_delta_mb']
        vram_total = stats.get('vram_total_mb', 0)
        vram_pct = (vram_peak / vram_total * 100) if vram_total else 0
        vram_line = f"    VRAM: peak {vram_peak:.0f} MB / {vram_total:.0f} MB ({vram_pct:.0f}%)  (delta +{vram_delta:.0f} MB)"
        console.print(gpu_line)
        console.print(vram_line)

        if vram_pct > 90:
            console.print(f"    [bold red]⚠ VRAM > 90% — risque de freeze PC ![/bold red]")
        elif vram_pct > 70:
            console.print(f"    [yellow]⚠ VRAM > 70% — attention sur PC avec moins de VRAM[/yellow]")


def _print_result(result: dict):
    mood = result.get("mood", "unknown")
    color = MOOD_COLORS.get(mood, "white")
    time_ms = result.get("time_ms", 0)
    raw_image_name = result.get("image", "?")
    image_name = _resolve_image_name(raw_image_name)

    # Check against ground truth
    expected = GROUND_TRUTH.get(image_name, "?")
    correct = _is_correct(raw_image_name, mood)
    status = "[green]OK[/green]" if correct else "[red]MISS[/red]"

    line = f"  {image_name:<14} [{color}]{mood:<22}[/{color}] expected: {expected:<22} {status}  {time_ms:.0f}ms"
    console.print(line)

    # Show raw response if it looks wrong
    if mood in ("unknown", "ERROR"):
        raw = result.get("raw_response", "")
        if raw:
            console.print(f"    [dim red]Raw: {raw[:200]}[/dim red]")


def print_summary(all_results: list[dict]):
    """Print a comparison table with scores per model."""
    # Group results by model
    from collections import defaultdict
    by_model = defaultdict(list)
    for r in all_results:
        by_model[r.get("model", "?")].append(r)

    # Score summary table
    console.print("\n")
    score_table = Table(
        title="Model Comparison",
        box=box.ROUNDED,
        show_lines=True,
    )
    score_table.add_column("Model", style="bold")
    score_table.add_column("Score", justify="center")
    score_table.add_column("Accuracy", justify="center")
    score_table.add_column("Avg Time", justify="right")
    score_table.add_column("Misses", style="red")

    for model_name, results in by_model.items():
        correct = sum(1 for r in results if _is_correct(r.get("image", ""), r.get("mood", "")))
        total = len(results)
        pct = (correct / total * 100) if total > 0 else 0
        avg_time = sum(r.get("time_ms", 0) for r in results) / total if total > 0 else 0
        misses = [_resolve_image_name(r.get("image", "?")) for r in results if not _is_correct(r.get("image", ""), r.get("mood", ""))]

        pct_color = "green" if pct >= 80 else "yellow" if pct >= 60 else "red"
        score_table.add_row(
            model_name,
            f"{correct}/{total}",
            f"[{pct_color}]{pct:.0f}%[/{pct_color}]",
            f"{avg_time:.0f}ms",
            ", ".join(misses) if misses else "[green]none[/green]",
        )

    console.print(score_table)

    # Per-image detail table
    console.print()
    detail_table = Table(
        title="Per-Image Detail",
        box=box.ROUNDED,
        show_lines=True,
    )
    detail_table.add_column("Image", style="dim")
    detail_table.add_column("Expected")
    for model_name in by_model:
        detail_table.add_column(model_name, max_width=30)

    # Collect all images in order
    all_images = list(dict.fromkeys(_resolve_image_name(r.get("image", "?")) for r in all_results))
    for image_name in all_images:
        expected = GROUND_TRUTH.get(image_name, "?")
        row = [image_name, expected]
        for model_name, results in by_model.items():
            match = next((r for r in results if _resolve_image_name(r.get("image", "")) == image_name), None)
            if match:
                mood = match.get("mood", "?")
                time_ms = match.get("time_ms", 0)
                correct = _is_correct(image_name, mood)
                color = "green" if correct else "red"
                row.append(f"[{color}]{mood}[/{color}] ({time_ms:.0f}ms)")
            else:
                row.append("-")
        detail_table.add_row(*row)

    console.print(detail_table)

    # Save results
    results_path = Path("results/benchmark.json")
    results_path.parent.mkdir(exist_ok=True)
    with open(results_path, "w", encoding="utf-8") as f:
        clean = []
        for r in all_results:
            c = {k: v for k, v in r.items()}
            clean.append(c)
        json.dump(clean, f, indent=2, ensure_ascii=False)
    console.print(f"\n[dim]Results saved to {results_path}[/dim]")


def _print_perf_summary(all_perf: dict, baseline: dict):
    """Print a comparison table of performance across all tested models."""
    table = Table(
        title="Performance Summary (vs baseline)",
        box=box.ROUNDED,
        show_lines=True,
    )
    table.add_column("Model", style="bold")
    table.add_column("CPU avg", justify="right")
    table.add_column("CPU peak", justify="right")
    table.add_column("RAM delta", justify="right")
    table.add_column("GPU avg", justify="right")
    table.add_column("GPU peak", justify="right")
    table.add_column("VRAM peak", justify="right")
    table.add_column("VRAM %", justify="right")

    # Baseline row
    vram_total = 0
    base_vram_pct = ""
    if baseline.get("vram_mb"):
        # Get total from first model's stats
        for stats in all_perf.values():
            if stats.get("vram_total_mb"):
                vram_total = stats["vram_total_mb"]
                break
        base_vram_pct = f"{baseline['vram_mb'] / vram_total * 100:.0f}%" if vram_total else ""

    table.add_row(
        "[dim]Baseline (idle)[/dim]",
        f"[dim]{baseline['cpu']:.1f}%[/dim]",
        f"[dim]—[/dim]",
        f"[dim]—[/dim]",
        f"[dim]{baseline.get('gpu_util', 0):.1f}%[/dim]",
        f"[dim]—[/dim]",
        f"[dim]{baseline.get('vram_mb', 0):.0f} MB[/dim]",
        f"[dim]{base_vram_pct}[/dim]",
    )

    for model_name, stats in all_perf.items():
        vram_peak = stats.get("vram_peak_mb", 0)
        vram_t = stats.get("vram_total_mb", vram_total)
        vram_pct = (vram_peak / vram_t * 100) if vram_t else 0

        # Color code VRAM usage
        if vram_pct > 90:
            vram_style = "bold red"
        elif vram_pct > 70:
            vram_style = "yellow"
        else:
            vram_style = "green"

        table.add_row(
            model_name,
            f"{stats['cpu_avg']:.1f}%",
            f"{stats['cpu_peak']:.1f}%",
            f"+{stats['ram_delta_mb']:.0f} MB",
            f"{stats.get('gpu_avg', 0):.1f}%",
            f"{stats.get('gpu_peak', 0):.1f}%",
            f"[{vram_style}]{vram_peak:.0f} MB[/{vram_style}]",
            f"[{vram_style}]{vram_pct:.0f}%[/{vram_style}]",
        )

    console.print("\n")
    console.print(table)
    console.print(f"\n[dim]VRAM total: {vram_total:.0f} MB | Baseline VRAM: {baseline.get('vram_mb', 0):.0f} MB[/dim]")


# ============================================================
# Main
# ============================================================

# Model configs: key -> (display_name, ollama_name, prompt_type, num_predict)
# prompt_type: "short" = descriptive prompt, "siglip" = embedding, "moondream2_tf"/"moondream3_tf" = transformers
MODEL_MAP = {
    "siglip":    ("SigLIP 2 ViT-B",  None,                                "siglip",   0),
    "moondream": ("Moondream 0.5B",   "moondream",                         "short",    5000),
    "qwen2b":    ("Qwen3-VL 2B",     "qwen3-vl:2b",                       "short",    5000),
    "qwen4b":    ("Qwen3-VL 4B",     "qwen3-vl:4b",                       "short",    5000),
    "qwen3b":    ("Qwen2.5-VL 3B",   "qwen2.5vl:3b",                      "short",    5000),
    "qwen7b":    ("Qwen2.5-VL 7B",   "qwen2.5vl:7b",                      "short",    5000),
    "gemma4b":   ("Gemma 3 4B",      "gemma3:4b",                          "short",    5000),
    "gemma3n2b": ("Gemma 3n E2B",   "gemma3n:e2b",                        "short",    5000),
    "gemma3n4b": ("Gemma 3n E4B",   "gemma3n:e4b",                        "short",    5000),
    "kimi3b":    ("Kimi-VL A3B Thinking", "richardyoung/kimi-vl-a3b-thinking", "short", 5000),
    "internvl":  ("InternVL3.5 4B",  "blaifa/InternVL3_5:4B",              "short",    5000),
    "smolvlm":   ("SmolVLM2 2.2B",   "richardyoung/smolvlm2-2.2b-instruct", "short",  5000),
    "minicpm":   ("MiniCPM-V 4.0",  "openbmb/minicpm-v4",                   "short",  5000),
    "md2":       ("Moondream 2 (2B)","moondream/moondream-2b-2025-04-14",     "moondream2_tf", 0),
    "md3":       ("Moondream 3 Preview","moondream/moondream3-preview",       "moondream3_tf", 0),
}

def collect_images(image_arg: str | None, dir_arg: str | None = None) -> list[str]:
    """Collect test images from test-images/, a specific directory, or a specific path."""
    if image_arg:
        if os.path.isfile(image_arg):
            return [image_arg]
        else:
            console.print(f"[red]Image not found: {image_arg}[/red]")
            sys.exit(1)

    if dir_arg:
        test_dir = Path(dir_arg)
        if not test_dir.is_dir():
            console.print(f"[red]Directory not found: {dir_arg}[/red]")
            sys.exit(1)
        images = []
        for ext in ("*.jpg", "*.jpeg", "*.png", "*.webp"):
            images.extend(str(p) for p in test_dir.rglob(ext))
        images.sort()
        if not images:
            console.print(f"[red]No images found in {dir_arg}[/red]")
            sys.exit(1)
        return images

    images = []
    test_dir = Path("test-images")
    if not test_dir.exists():
        console.print("[red]No test-images/ directory found. Create it and add manga/webtoon images.[/red]")
        sys.exit(1)

    # Exclude subdirectories used by other benchmarks (e.g. bluelock-sequence for context test)
    exclude_dirs = {"bluelock-sequence"}
    for ext in ("*.jpg", "*.jpeg", "*.png", "*.webp"):
        for p in test_dir.rglob(ext):
            if not any(part in exclude_dirs for part in p.parts):
                images.append(str(p))

    images.sort()
    if not images:
        console.print("[red]No images found in test-images/. Add .jpg/.png/.webp files.[/red]")
        console.print("[dim]Expected structure:[/dim]")
        console.print("[dim]  test-images/manga/   (black & white manga pages)[/dim]")
        console.print("[dim]  test-images/webtoon/  (color webtoon pages)[/dim]")
        sys.exit(1)

    return images


def main():
    parser = argparse.ArgumentParser(description="Test manga mood AI models")
    parser.add_argument("--model", choices=list(MODEL_MAP.keys()), help="Test specific model only")
    parser.add_argument("--image", help="Test on a specific image file")
    parser.add_argument("--context", type=int, default=0, help="Number of context pages for VLMs (multi-image)")
    parser.add_argument("--custom-prompt", help="Custom prompt for VLMs (overrides default)")
    parser.add_argument("--dir", help="Test on all images in a specific directory (e.g. test-images/bluelock-sequence)")
    parser.add_argument("--resize", type=int, default=0, help="Resize images to max N px before inference (e.g. 672)")
    args = parser.parse_args()

    console.print(Panel.fit(
        "[bold]Manga Mood AI — Model Benchmark[/bold]\n"
        "Testing vision models on manga/webtoon mood classification",
        border_style="cyan",
    ))

    images = collect_images(args.image, args.dir)
    console.print(f"\nFound [bold]{len(images)}[/bold] test images")

    # Apply resize if requested
    if args.resize > 0:
        console.print(f"[cyan]Resizing images to max {args.resize}px...[/cyan]")
        images = [resize_image(img, args.resize) for img in images]
        console.print(f"  [dim]Resized {len(images)} images[/dim]")

    # Init performance monitoring
    has_gpu = _init_gpu_monitor()
    perf_monitor = PerfMonitor(has_gpu)

    console.print("\n[dim]Measuring baseline performance (2s)...[/dim]")
    baseline = perf_monitor.measure_baseline(duration=2.0)
    if has_gpu:
        console.print(f"  [dim]Baseline — CPU: {baseline['cpu']:.1f}%  RAM: {baseline['ram_mb']:.0f} MB  GPU: {baseline['gpu_util']:.1f}%  VRAM: {baseline['vram_mb']:.0f} MB[/dim]")
    else:
        console.print(f"  [dim]Baseline — CPU: {baseline['cpu']:.1f}%  RAM: {baseline['ram_mb']:.0f} MB[/dim]")

    models_to_test = [args.model] if args.model else list(MODEL_MAP.keys())
    all_results = []
    all_perf = {}

    for model_key in models_to_test:
        display_name, ollama_name, prompt_type, num_predict = MODEL_MAP[model_key]

        # Create fresh monitor for each model (shares baseline)
        model_monitor = PerfMonitor(has_gpu)
        model_monitor.baseline = perf_monitor.baseline

        if model_key == "siglip":
            results = test_siglip(images)
        elif prompt_type == "moondream2_tf":
            results = test_moondream_tf(
                hf_model_name=ollama_name,
                display_name=display_name,
                image_paths=images,
                use_bnb_4bit=False,
                perf_monitor=model_monitor,
            )
        elif prompt_type == "moondream3_tf":
            results = test_moondream_tf(
                hf_model_name=ollama_name,
                display_name=display_name,
                image_paths=images,
                use_bnb_4bit=True,
                perf_monitor=model_monitor,
            )
        else:
            results = test_ollama_vlm(
                model_name=ollama_name,
                display_name=display_name,
                image_paths=images,
                context_pages=args.context,
                custom_prompt=args.custom_prompt,
                prompt_type=prompt_type,
                num_predict=num_predict,
                perf_monitor=model_monitor,
            )
        # Print inline score for this model
        if results:
            correct = sum(1 for r in results if _is_correct(r.get("image", ""), r.get("mood", "")))
            total = len(results)
            avg_ms = sum(r.get("time_ms", 0) for r in results) / total if total else 0
            pct = correct / total * 100 if total else 0
            pct_color = "green" if pct >= 80 else "yellow" if pct >= 60 else "red"
            console.print(f"\n  Score: [{pct_color}]{correct}/{total} ({pct:.0f}%)[/{pct_color}]  Avg: {avg_ms:.0f}ms")

        all_results.extend(results)

        # Collect perf for final summary
        if results and results[0].get("perf"):
            all_perf[display_name] = results[0]["perf"]

    if all_results:
        print_summary(all_results)

    if all_perf:
        _print_perf_summary(all_perf, baseline)

    # Cleanup temp resized images
    temp_dir = Path("temp_resized")
    if temp_dir.exists():
        import shutil
        shutil.rmtree(temp_dir)
        console.print("[dim]Cleaned up temp resized images[/dim]")


if __name__ == "__main__":
    main()
