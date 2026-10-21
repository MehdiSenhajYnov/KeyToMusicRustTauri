#!/usr/bin/env python3

from __future__ import annotations

import argparse
import errno
import json
import socket
from datetime import datetime, timezone
from http import HTTPStatus
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.error import URLError
from urllib.parse import parse_qs, urlsplit
from urllib.request import urlopen


ROOT = Path(__file__).resolve().parents[1]
RESULTS_DIR = ROOT / "manga-mood-ai" / "results"
GROUND_TRUTH_PATH = ROOT / "manga-mood-ai" / "test-images" / "RealTest" / "ground_truth.json"
HUB_PATH = "/manga-mood-ai/results/realtest-hub.html"
RESULTS_VIEWER_PATH = "/manga-mood-ai/results/realtest-results-viewer.html"
GROUND_TRUTH_VIEWER_PATH = "/manga-mood-ai/test-images/RealTest/viewer.html"
IMAGE_ROOT = "/manga-mood-ai/test-images/RealTest/"

MOOD_FAMILY = {
    "epic": 1,
    "tension": 1,
    "sadness": 2,
    "peaceful": 2,
    "comedy": 3,
    "romance": 3,
    "horror": 4,
    "mystery": 4,
}
SOUNDTRACK_COMMIT_SUFFIX = "_soundtrack_commit.json"


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def is_result_map(payload) -> bool:
    return (
        isinstance(payload, dict)
        and bool(payload)
        and all(isinstance(key, str) for key in payload.keys())
        and all(isinstance(value, str) for value in payload.values())
    )


def build_method_label(relative_path: Path) -> str:
    label = relative_path.as_posix()
    if label.endswith(SOUNDTRACK_COMMIT_SUFFIX):
        base_label = build_method_label(
            Path(label[: -len(SOUNDTRACK_COMMIT_SUFFIX)] + ".json")
        )
        return f"{base_label} + OST commit"
    if label == "realtests/realtest_suite_t4b_wide5_selective_hold.json":
        return "Production"
    if label == "realtests/realtest_suite_t4b_wide5_selective.json":
        return "Baseline"
    if label.endswith(".json"):
        label = label[:-5]
    return label


def method_priority(source_path: str) -> int:
    if source_path.endswith(SOUNDTRACK_COMMIT_SUFFIX):
        return method_priority(source_path[: -len(SOUNDTRACK_COMMIT_SUFFIX)] + ".json") + 0.5
    if source_path == "realtest_suite_t4b_wide5_selective_hold.json":
        return 3
    if source_path == "realtests/realtest_suite_t4b_wide5_selective_hold.json":
        return 0
    if source_path == "realtests/realtest_suite_t4b_wide5_selective.json":
        return 1
    if source_path == "realtest_suite_t4b_wide5_selective.json":
        return 4
    if source_path.startswith("realtests/realtest_suite_"):
        return 2
    if source_path.startswith("realtest_suite_"):
        return 5
    if source_path.startswith("realtests/realtest_v12_"):
        return 6
    if source_path.startswith("passes/"):
        return 7
    return 8


def parse_tag(raw_tag: str):
    if not isinstance(raw_tag, str):
        return None
    mood_part, _, intensity_part = raw_tag.partition(":")
    mood = mood_part.strip()
    if not mood:
        return None
    try:
        intensity = int(float(intensity_part or "0"))
    except ValueError:
        intensity = 0
    return {"mood": mood, "intensity": intensity}


def classify_status(expected: dict, raw_prediction: str) -> str:
    prediction = parse_tag(raw_prediction)
    if not prediction:
        return "fail"
    if prediction["mood"] == expected["mood"]:
        return "strict"
    if MOOD_FAMILY.get(prediction["mood"], -1) == MOOD_FAMILY.get(expected["mood"], -1):
        return "relaxed"
    return "fail"


def load_ground_truth():
    payload = load_json(GROUND_TRUTH_PATH)
    ground_truth = {}
    for rel_path, entry in payload.items():
        if not isinstance(entry, dict):
            continue
        mood = entry.get("mood")
        if not isinstance(mood, str) or not mood:
            continue
        try:
            intensity = int(float(entry.get("intensity", 0)))
        except ValueError:
            intensity = 0
        try:
            confidence = float(entry.get("confidence", 0))
        except ValueError:
            confidence = 0.0
        ground_truth[rel_path] = {
            "mood": mood,
            "intensity": intensity,
            "confidence": confidence,
            "reasoning": entry.get("reasoning", ""),
        }
    return ground_truth


def compute_method_stats(predictions: dict, ground_truth: dict):
    coverage = 0
    counted = 0
    strict = 0
    relaxed_pass = 0

    for rel_path, raw_prediction in predictions.items():
        expected = ground_truth.get(rel_path)
        if not expected:
            continue
        coverage += 1
        if expected["confidence"] <= 0:
            continue
        counted += 1
        status = classify_status(expected, raw_prediction)
        if status == "strict":
            strict += 1
            relaxed_pass += 1
        elif status == "relaxed":
            relaxed_pass += 1

    return {
        "pageCount": coverage,
        "counted": counted,
        "strict": strict,
        "relaxedPass": relaxed_pass,
    }


def collect_methods(ground_truth: dict):
    methods = []
    for path in sorted(RESULTS_DIR.rglob("*.json")):
        try:
            payload = load_json(path)
        except Exception:
            continue

        if not is_result_map(payload):
            continue

        relative_path = path.relative_to(RESULTS_DIR)
        source_path = relative_path.as_posix()
        stats = compute_method_stats(payload, ground_truth)
        methods.append(
            {
                "id": source_path,
                "label": build_method_label(relative_path),
                "sourcePath": source_path,
                "pageCount": stats["pageCount"],
                "counted": stats["counted"],
                "strict": stats["strict"],
                "relaxedPass": stats["relaxedPass"],
            }
        )

    methods.sort(key=lambda method: (method_priority(method["sourcePath"]), method["label"]))
    return methods


def resolve_method_path(source_path: str) -> Path:
    if not source_path:
        raise ValueError("parametre `source` manquant")
    results_root = RESULTS_DIR.resolve()
    candidate = (results_root / source_path).resolve()
    try:
        candidate.relative_to(results_root)
    except ValueError as exc:
        raise ValueError("source hors dossier results") from exc
    if not candidate.is_file():
        raise ValueError("fichier methode introuvable")
    return candidate


class RealTestViewerHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def do_GET(self):
        parsed = urlsplit(self.path)

        if self.maybe_redirect_page(parsed.path):
            return

        if parsed.path == "/api/catalog":
            self.handle_catalog()
            return

        if parsed.path == "/api/ground-truth":
            self.handle_ground_truth()
            return

        if parsed.path == "/api/method":
            self.handle_method(parsed.query)
            return

        super().do_GET()

    def do_HEAD(self):
        parsed = urlsplit(self.path)
        if self.maybe_redirect_page(parsed.path):
            return
        super().do_HEAD()

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def maybe_redirect_page(self, path: str) -> bool:
        redirects = {
            "": HUB_PATH,
            "/": HUB_PATH,
            "/realtest": HUB_PATH,
            "/viewer": RESULTS_VIEWER_PATH,
            "/compare": RESULTS_VIEWER_PATH,
            "/ground-truth": GROUND_TRUTH_VIEWER_PATH,
        }
        target = redirects.get(path)
        if not target:
            return False
        self.send_response(HTTPStatus.FOUND)
        self.send_header("Location", target)
        self.end_headers()
        return True

    def handle_catalog(self):
        ground_truth = load_ground_truth()
        methods = collect_methods(ground_truth)
        payload = {
            "generatedAt": datetime.now(timezone.utc).isoformat(),
            "groundTruthCount": len(ground_truth),
            "groundTruthPath": GROUND_TRUTH_PATH.relative_to(ROOT).as_posix(),
            "imageRoot": IMAGE_ROOT,
            "methods": methods,
        }
        self.send_json(payload)

    def handle_ground_truth(self):
        payload = load_json(GROUND_TRUTH_PATH)
        self.send_json(payload)

    def handle_method(self, query_string: str):
        query = parse_qs(query_string)
        source_path = query.get("source", [""])[0]

        try:
            method_path = resolve_method_path(source_path)
            payload = load_json(method_path)
        except ValueError as error:
            self.send_error(HTTPStatus.BAD_REQUEST, str(error))
            return
        except json.JSONDecodeError as error:
            self.send_error(HTTPStatus.INTERNAL_SERVER_ERROR, f"json invalide: {error}")
            return

        if not is_result_map(payload):
            self.send_error(HTTPStatus.BAD_REQUEST, "ce fichier n'est pas une result map")
            return

        relative_path = method_path.relative_to(RESULTS_DIR)
        response = {
            "id": relative_path.as_posix(),
            "label": build_method_label(relative_path),
            "sourcePath": relative_path.as_posix(),
            "pageCount": len(payload),
            "predictions": payload,
        }
        self.send_json(response)

    def send_json(self, payload, status: HTTPStatus = HTTPStatus.OK):
        body = json.dumps(payload, ensure_ascii=True, separators=(",", ":")).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


class ReusableThreadingHTTPServer(ThreadingHTTPServer):
    allow_reuse_address = True


def parse_args():
    parser = argparse.ArgumentParser(description="Serveur local pour le RealTest viewer.")
    parser.add_argument("--host", default="127.0.0.1", help="Host d'ecoute. Defaut: 127.0.0.1")
    parser.add_argument("--port", type=int, default=8766, help="Port d'ecoute. Defaut: 8766")
    return parser.parse_args()


def hub_url(host: str, port: int) -> str:
    return f"http://{host}:{port}/realtest"


def existing_viewer_is_reachable(host: str, port: int) -> bool:
    try:
        with urlopen(hub_url(host, port), timeout=1.5) as response:
            final_url = getattr(response, "url", "")
            return response.status == HTTPStatus.OK and "realtest-hub.html" in final_url
    except URLError:
        return False
    except OSError:
        return False


def suggest_next_port(host: str, start_port: int) -> int | None:
    max_port = min(65535, start_port + 20)
    for port in range(start_port, max_port + 1):
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            try:
                sock.bind((host, port))
            except OSError:
                continue
        return port
    return None


def main():
    args = parse_args()
    try:
        server = ReusableThreadingHTTPServer((args.host, args.port), RealTestViewerHandler)
    except OSError as error:
        if error.errno == errno.EADDRINUSE:
            current_hub_url = hub_url(args.host, args.port)
            if existing_viewer_is_reachable(args.host, args.port):
                print(f"RealTest viewer already running: {current_hub_url}")
                return
            suggested_port = suggest_next_port(args.host, args.port + 1)
            if suggested_port is not None:
                raise SystemExit(
                    f"Port {args.port} deja utilise. Essaye --port {suggested_port} "
                    f"ou ferme le process qui ecoute sur {current_hub_url}."
                ) from error
            raise SystemExit(
                f"Port {args.port} deja utilise et aucun port libre n'a ete trouve a proximite."
            ) from error
        raise

    current_hub_url = hub_url(args.host, args.port)
    print(f"RealTest hub: {current_hub_url}")
    print("Stop: Ctrl+C")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
