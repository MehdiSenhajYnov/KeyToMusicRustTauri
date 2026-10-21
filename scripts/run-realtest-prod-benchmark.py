#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
import os
import signal
import subprocess
import sys
import time
from collections import OrderedDict
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GROUND_TRUTH_PATH = ROOT / "manga-mood-ai" / "test-images" / "RealTest" / "ground_truth.json"
SCRATCH_RESULT_PATH = ROOT / "manga-mood-ai" / "results" / "realtest_suite_t4b_wide5_selective_hold.json"
PRODUCTION_RESULT_PATH = (
    ROOT / "manga-mood-ai" / "results" / "realtests" / "realtest_suite_t4b_wide5_selective_hold.json"
)
SEED_RESULT_PATH = ROOT / "manga-mood-ai" / "results" / "realtest_suite_t4b_wide5_selective_hold_runtime_prod_8192.json"
STATE_DIR = ROOT / "data" / "realtest-prod-benchmark"
PID_PATH = STATE_DIR / "runner.pid"
STATUS_PATH = STATE_DIR / "status.json"
STOP_FLAG_PATH = STATE_DIR / "stop.requested"
LOG_PATH = STATE_DIR / "runner.log"

EXPERIMENT_ID = "t4b_wide5_selective_hold"
IMAGE_ROOT = ROOT / "manga-mood-ai" / "test-images" / "RealTest"
BENCHMARK_COMMAND = [
    "cargo",
    "run",
    "--manifest-path",
    "src-tauri/Cargo.toml",
]
MAX_CHAPTER_ATTEMPTS = 3
RETRY_DELAY_S = 5

STOP_REQUESTED = False
ACTIVE_CHILD: subprocess.Popen[str] | None = None


def iso_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def ensure_state_dir() -> None:
    STATE_DIR.mkdir(parents=True, exist_ok=True)
    PRODUCTION_RESULT_PATH.parent.mkdir(parents=True, exist_ok=True)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def is_result_map(payload) -> bool:
    return (
        isinstance(payload, dict)
        and bool(payload)
        and all(isinstance(key, str) for key in payload.keys())
        and all(isinstance(value, str) for value in payload.values())
    )


def write_json_atomic(path: Path, payload) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(path.suffix + ".tmp")
    tmp_path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    tmp_path.replace(path)


def write_pid(pid: int | None) -> None:
    if pid is None:
        if PID_PATH.exists():
            PID_PATH.unlink()
        return
    PID_PATH.write_text(f"{pid}\n", encoding="utf-8")


def read_pid() -> int | None:
    if not PID_PATH.exists():
        return None
    try:
        return int(PID_PATH.read_text(encoding="utf-8").strip())
    except ValueError:
        return None


def is_process_running(pid: int | None) -> bool:
    if not pid or pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


def clear_stale_pid() -> None:
    pid = read_pid()
    if pid and not is_process_running(pid):
        write_pid(None)


def read_status() -> dict:
    if not STATUS_PATH.exists():
        return {}
    try:
        payload = load_json(STATUS_PATH)
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def write_status(**updates) -> dict:
    status = read_status()
    status.update(updates)
    status["updatedAt"] = iso_now()
    write_json_atomic(STATUS_PATH, status)
    return status


def install_signal_handlers() -> None:
    def _request_stop(signum, _frame):
        del signum
        request_stop_flag()

    signal.signal(signal.SIGINT, _request_stop)
    signal.signal(signal.SIGTERM, _request_stop)


def request_stop_flag() -> None:
    global STOP_REQUESTED
    STOP_REQUESTED = True
    ensure_state_dir()
    STOP_FLAG_PATH.write_text(f"{iso_now()}\n", encoding="utf-8")


def stop_requested() -> bool:
    return STOP_REQUESTED or STOP_FLAG_PATH.exists()


def chapter_sort_key(chapter_key: str):
    series, _, chapter = chapter_key.partition("/")
    if chapter.isdigit():
        return (series, 0, int(chapter))
    return (series, 1, chapter)


def load_ground_truth_chapters() -> OrderedDict[str, list[str]]:
    payload = load_json(GROUND_TRUTH_PATH)
    if not isinstance(payload, dict):
        raise RuntimeError(f"ground truth invalide: {GROUND_TRUTH_PATH}")

    grouped: dict[str, list[str]] = {}
    for rel_path in payload.keys():
        if not isinstance(rel_path, str):
            continue
        parts = rel_path.split("/")
        if len(parts) != 3:
            continue
        chapter_key = f"{parts[0]}/{parts[1]}"
        image_path = IMAGE_ROOT / rel_path
        if not is_benchmarkable_image(image_path):
            continue
        grouped.setdefault(chapter_key, []).append(rel_path)

    ordered = OrderedDict()
    for chapter_key in sorted(grouped.keys(), key=chapter_sort_key):
        ordered[chapter_key] = sorted(grouped[chapter_key])
    return ordered


def is_benchmarkable_image(path: Path) -> bool:
    if not path.exists() or not path.is_file():
        return False
    try:
        data = path.read_bytes()
    except OSError:
        return False
    if not data:
        return False
    return (
        data.startswith(b"\xff\xd8\xff")
        or data.startswith(b"\x89PNG\r\n\x1a\n")
        or (len(data) >= 12 and data.startswith(b"RIFF") and data[8:12] == b"WEBP")
    )


def load_result_map(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}
    payload = load_json(path)
    if not is_result_map(payload):
        raise RuntimeError(f"result map invalide: {path}")
    return dict(payload)


def seed_production_file() -> dict[str, str]:
    existing = load_result_map(PRODUCTION_RESULT_PATH) if PRODUCTION_RESULT_PATH.exists() else {}
    if existing:
        return existing

    if SEED_RESULT_PATH.exists():
        seeded = load_result_map(SEED_RESULT_PATH)
        write_json_atomic(PRODUCTION_RESULT_PATH, seeded)
        return seeded

    write_json_atomic(PRODUCTION_RESULT_PATH, {})
    return {}


def completed_chapters(chapters: OrderedDict[str, list[str]], result_map: dict[str, str]) -> list[str]:
    done = []
    for chapter_key, pages in chapters.items():
        if all(page in result_map for page in pages):
            done.append(chapter_key)
    return done


def chapter_page_counts(chapters: OrderedDict[str, list[str]]) -> dict[str, int]:
    return {chapter_key: len(pages) for chapter_key, pages in chapters.items()}


def summarize_progress(chapters: OrderedDict[str, list[str]], result_map: dict[str, str]) -> tuple[list[str], list[str]]:
    done = completed_chapters(chapters, result_map)
    remaining = [chapter_key for chapter_key in chapters.keys() if chapter_key not in set(done)]
    return done, remaining


def validate_scratch_result(chapter_key: str, expected_pages: list[str]) -> dict[str, str]:
    if not SCRATCH_RESULT_PATH.exists():
        raise RuntimeError(f"resultat scratch absent pour {chapter_key}: {SCRATCH_RESULT_PATH}")

    scratch = load_result_map(SCRATCH_RESULT_PATH)
    expected = set(expected_pages)
    missing = sorted(page for page in expected if page not in scratch)
    if missing:
        preview = ", ".join(missing[:5])
        raise RuntimeError(
            f"resultat scratch incomplet pour {chapter_key}: {len(missing)} page(s) manquante(s) ({preview})"
        )

    return {page: scratch[page] for page in expected_pages}


def update_runtime_status(
    *,
    state_value: str,
    chapters: OrderedDict[str, list[str]],
    result_map: dict[str, str],
    current_chapter: str | None = None,
    last_completed_chapter: str | None = None,
    current_child_pid: int | None = None,
    last_error: str | None = None,
) -> dict:
    completed, remaining = summarize_progress(chapters, result_map)
    page_total = sum(len(pages) for pages in chapters.values())
    all_pages = {page for pages in chapters.values() for page in pages}
    page_done = sum(1 for rel_path in result_map if rel_path in all_pages)
    return write_status(
        state=state_value,
        pid=os.getpid(),
        currentChildPid=current_child_pid,
        command=" ".join(BENCHMARK_COMMAND),
        experimentId=EXPERIMENT_ID,
        currentChapter=current_chapter,
        lastCompletedChapter=last_completed_chapter,
        completedChapters=completed,
        remainingChapters=remaining,
        completedChapterCount=len(completed),
        totalChapterCount=len(chapters),
        completedPageCount=page_done,
        totalPageCount=page_total,
        productionPath=PRODUCTION_RESULT_PATH.relative_to(ROOT).as_posix(),
        scratchPath=SCRATCH_RESULT_PATH.relative_to(ROOT).as_posix(),
        seedPath=SEED_RESULT_PATH.relative_to(ROOT).as_posix(),
        logPath=LOG_PATH.relative_to(ROOT).as_posix(),
        lastError=last_error,
    )


def append_log(message: str) -> None:
    ensure_state_dir()
    with LOG_PATH.open("a", encoding="utf-8") as handle:
        handle.write(message)
        if not message.endswith("\n"):
            handle.write("\n")


def run_chapter(chapter_key: str) -> int:
    global ACTIVE_CHILD

    if SCRATCH_RESULT_PATH.exists():
        SCRATCH_RESULT_PATH.unlink()

    env = os.environ.copy()
    env["REALTEST_FILTER"] = chapter_key
    env["REALTEST_EXPERIMENTS"] = EXPERIMENT_ID
    env["KEYTOMUSIC_RUN_REALTEST_BENCHMARK"] = "1"

    append_log(f"\n[{iso_now()}] ==== benchmark {chapter_key} ====")

    process = subprocess.Popen(
        BENCHMARK_COMMAND,
        cwd=ROOT,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    ACTIVE_CHILD = process

    assert process.stdout is not None
    for line in process.stdout:
        sys.stdout.write(line)
        sys.stdout.flush()

    return_code = process.wait()
    ACTIVE_CHILD = None
    return return_code


def run_runner() -> int:
    global ACTIVE_CHILD

    ensure_state_dir()
    clear_stale_pid()
    existing_pid = read_pid()
    if existing_pid and existing_pid != os.getpid() and is_process_running(existing_pid):
        print(f"Runner deja actif avec PID {existing_pid}.", file=sys.stderr)
        return 1

    install_signal_handlers()
    if STOP_FLAG_PATH.exists():
        STOP_FLAG_PATH.unlink()

    write_pid(os.getpid())

    chapters = load_ground_truth_chapters()
    result_map = seed_production_file()
    update_runtime_status(state_value="running", chapters=chapters, result_map=result_map)

    last_completed = None

    try:
        for chapter_key, pages in chapters.items():
            if all(page in result_map for page in pages):
                last_completed = chapter_key
                continue

            if stop_requested():
                update_runtime_status(
                    state_value="stopped",
                    chapters=chapters,
                    result_map=result_map,
                    current_chapter=None,
                    last_completed_chapter=last_completed,
                )
                return 0

            update_runtime_status(
                state_value="running",
                chapters=chapters,
                result_map=result_map,
                current_chapter=chapter_key,
                last_completed_chapter=last_completed,
            )

            chapter_result = None
            last_attempt_error = None
            for attempt in range(1, MAX_CHAPTER_ATTEMPTS + 1):
                append_log(
                    f"[{iso_now()}] lancement {chapter_key} tentative {attempt}/{MAX_CHAPTER_ATTEMPTS}"
                )
                return_code = run_chapter(chapter_key)

                if return_code == 0:
                    try:
                        chapter_result = validate_scratch_result(chapter_key, pages)
                        break
                    except Exception as exc:
                        last_attempt_error = str(exc)
                else:
                    last_attempt_error = f"Benchmark {chapter_key} termine avec code {return_code}"

                if stop_requested():
                    break

                if attempt < MAX_CHAPTER_ATTEMPTS:
                    append_log(
                        f"[{iso_now()}] echec {chapter_key} tentative {attempt}/{MAX_CHAPTER_ATTEMPTS}: "
                        f"{last_attempt_error}. Retry dans {RETRY_DELAY_S}s."
                    )
                    time.sleep(RETRY_DELAY_S)

            if chapter_result is None:
                message = last_attempt_error or f"Benchmark {chapter_key} a echoue sans resultat exploitable"
                update_runtime_status(
                    state_value="failed",
                    chapters=chapters,
                    result_map=result_map,
                    current_chapter=chapter_key,
                    last_completed_chapter=last_completed,
                    last_error=message,
                )
                append_log(f"[{iso_now()}] {message}")
                return 1

            result_map.update(chapter_result)
            write_json_atomic(PRODUCTION_RESULT_PATH, result_map)
            last_completed = chapter_key
            update_runtime_status(
                state_value="running",
                chapters=chapters,
                result_map=result_map,
                current_chapter=None,
                last_completed_chapter=last_completed,
            )

            if stop_requested():
                update_runtime_status(
                    state_value="stopped",
                    chapters=chapters,
                    result_map=result_map,
                    current_chapter=None,
                    last_completed_chapter=last_completed,
                )
                return 0

        update_runtime_status(
            state_value="completed",
            chapters=chapters,
            result_map=result_map,
            current_chapter=None,
            last_completed_chapter=last_completed,
        )
        append_log(f"[{iso_now()}] benchmark termine")
        return 0
    except Exception as exc:
        update_runtime_status(
            state_value="failed",
            chapters=chapters,
            result_map=result_map,
            current_chapter=None,
            last_completed_chapter=last_completed,
            last_error=str(exc),
        )
        append_log(f"[{iso_now()}] erreur: {exc}")
        raise
    finally:
        ACTIVE_CHILD = None
        if STOP_FLAG_PATH.exists():
            STOP_FLAG_PATH.unlink()
        write_pid(None)


def start_runner() -> int:
    ensure_state_dir()
    clear_stale_pid()
    pid = read_pid()
    if pid and is_process_running(pid):
        print(f"Runner deja actif (PID {pid}).")
        return 0

    with LOG_PATH.open("a", encoding="utf-8") as log_handle:
        process = subprocess.Popen(
            [sys.executable, str(Path(__file__).resolve()), "run"],
            cwd=ROOT,
            stdout=log_handle,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )

    time.sleep(1.0)
    write_pid(process.pid)
    print(f"Runner lance en arriere-plan (PID {process.pid}).")
    print(f"Log: {LOG_PATH.relative_to(ROOT).as_posix()}")
    print(f"Stop: python3 scripts/run-realtest-prod-benchmark.py stop")
    print(f"Status: python3 scripts/run-realtest-prod-benchmark.py status")
    return 0


def stop_runner() -> int:
    ensure_state_dir()
    request_stop_flag()
    status = read_status()
    pid = read_pid()
    state_value = status.get("state") or "unknown"
    if pid and is_process_running(pid):
        write_status(state="stopping", pid=pid)
        print(f"Arret demande pour le runner PID {pid}. Il s'arretera apres le chapitre en cours.")
    else:
        print("Aucun runner actif. Le prochain lancement verra quand meme la demande d'arret effacee au demarrage.")
    print(f"Etat precedent: {state_value}")
    return 0


def status_runner() -> int:
    ensure_state_dir()
    clear_stale_pid()
    status = read_status()
    pid = read_pid()
    running = is_process_running(pid)

    if not status:
        print("Aucun statut disponible.")
        return 0

    print(f"etat: {status.get('state', 'unknown')}")
    print(f"pid: {pid or '-'} {'(actif)' if running else '(inactif)'}")
    print(f"chapitre courant: {status.get('currentChapter') or '-'}")
    print(f"dernier chapitre termine: {status.get('lastCompletedChapter') or '-'}")
    print(
        "progression: "
        f"{status.get('completedChapterCount', 0)}/{status.get('totalChapterCount', 0)} chapitres, "
        f"{status.get('completedPageCount', 0)}/{status.get('totalPageCount', 0)} pages"
    )
    if status.get("lastError"):
        print(f"erreur: {status['lastError']}")
    print(f"log: {status.get('logPath') or LOG_PATH.relative_to(ROOT).as_posix()}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Runner resumable pour le benchmark Production RealTest.")
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("run", help="Lance le runner au premier plan.")
    subparsers.add_parser("start", help="Lance le runner en arriere-plan.")
    subparsers.add_parser("stop", help="Demande un arret propre apres le chapitre en cours.")
    subparsers.add_parser("status", help="Affiche le statut du runner.")
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()

    if args.command == "run":
        return run_runner()
    if args.command == "start":
        return start_runner()
    if args.command == "stop":
        return stop_runner()
    if args.command == "status":
        return status_runner()

    parser.error(f"commande inconnue: {args.command}")
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
