#!/usr/bin/env bash

set -euo pipefail

APP_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/keytomusic"
LLAMA_DIR="$APP_DATA_DIR/bin/llama-server"
LLAMA_BIN="$LLAMA_DIR/llama-server"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODEL_DIR="$REPO_ROOT/manga-mood-ai/models/Qwen3-VL-4B-Thinking"
MODEL_PATH="$MODEL_DIR/Qwen3VL-4B-Thinking-Q4_K_M.gguf"
MMPROJ_PATH="$MODEL_DIR/mmproj-Qwen3VL-4B-Thinking-F16.gguf"
INSTALL_GEMMA4_E4B=0
INSTALL_GEMMA4_E2B=0
FORCE_LLAMA_SERVER=0

MODEL_URL="https://huggingface.co/Qwen/Qwen3-VL-4B-Thinking-GGUF/resolve/main/Qwen3VL-4B-Thinking-Q4_K_M.gguf"
MMPROJ_URL="https://huggingface.co/Qwen/Qwen3-VL-4B-Thinking-GGUF/resolve/main/mmproj-Qwen3VL-4B-Thinking-F16.gguf"
GEMMA4_E4B_DIR="$REPO_ROOT/manga-mood-ai/models/unsloth_gemma-4-E4B-it-GGUF"
GEMMA4_E4B_MODEL_PATH="$GEMMA4_E4B_DIR/gemma-4-E4B-it-Q4_K_M.gguf"
GEMMA4_E4B_MMPROJ_PATH="$GEMMA4_E4B_DIR/mmproj-F16.gguf"
GEMMA4_E4B_MODEL_URL="https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/gemma-4-E4B-it-Q4_K_M.gguf"
GEMMA4_E4B_MMPROJ_URL="https://huggingface.co/unsloth/gemma-4-E4B-it-GGUF/resolve/main/mmproj-F16.gguf"
GEMMA4_E2B_DIR="$REPO_ROOT/manga-mood-ai/models/unsloth_gemma-4-E2B-it-GGUF"
GEMMA4_E2B_MODEL_PATH="$GEMMA4_E2B_DIR/gemma-4-E2B-it-Q4_K_M.gguf"
GEMMA4_E2B_MMPROJ_PATH="$GEMMA4_E2B_DIR/mmproj-F16.gguf"
GEMMA4_E2B_MODEL_URL="https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/gemma-4-E2B-it-Q4_K_M.gguf"
GEMMA4_E2B_MMPROJ_URL="https://huggingface.co/unsloth/gemma-4-E2B-it-GGUF/resolve/main/mmproj-F16.gguf"

usage() {
  cat <<EOF
Usage: $0 [--gemma4-e4b] [--gemma4-e2b] [--force-llama-server]

Installs the default RealTest benchmark stack: llama-server + Qwen3-VL-4B-Thinking.

Optional flags:
  --gemma4-e4b   Also download Gemma 4 E4B (Q4_K_M + mmproj F16)
  --gemma4-e2b   Also download Gemma 4 E2B (Q4_K_M + mmproj F16)
  --force-llama-server  Reinstall llama-server from the latest release
  -h, --help     Show this help
EOF
}

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --gemma4-e4b)
        INSTALL_GEMMA4_E4B=1
        ;;
      --gemma4-e2b)
        INSTALL_GEMMA4_E2B=1
        ;;
      --force-llama-server)
        FORCE_LLAMA_SERVER=1
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        echo "Unknown argument: $1" >&2
        usage >&2
        exit 1
        ;;
    esac
    shift
  done
}

min_size_ok() {
  local path="$1"
  local min_size="$2"

  if [[ ! -f "$path" ]]; then
    return 1
  fi

  local actual_size
  actual_size="$(stat -c%s "$path" 2>/dev/null || echo 0)"
  [[ "$actual_size" -ge "$min_size" ]]
}

download_file() {
  local url="$1"
  local target="$2"

  local tmp
  tmp="$(mktemp "${target##*/}.XXXXXX.tmp")"
  trap 'rm -f "$tmp"' RETURN

  echo "Downloading $(basename "$target")..."
  curl --fail --location --progress-bar "$url" -o "$tmp"
  mv "$tmp" "$target"
  trap - RETURN
}

resolve_linux_release_url() {
  python3 - <<'PY'
import json
import os
import shutil
import subprocess
import sys
import urllib.request

backend_override = os.environ.get("KEYTOMUSIC_LLAMA_BACKEND")
prefer_vulkan = False
if backend_override == "vulkan":
    prefer_vulkan = True
elif backend_override == "cpu":
    prefer_vulkan = False
else:
    vulkaninfo = shutil.which("vulkaninfo")
    if vulkaninfo:
        result = subprocess.run(
            [vulkaninfo, "--summary"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        prefer_vulkan = result.returncode == 0

patterns = ["ubuntu-vulkan-x64", "ubuntu-x64"] if prefer_vulkan else ["ubuntu-x64"]

with urllib.request.urlopen("https://api.github.com/repos/ggml-org/llama.cpp/releases/latest") as r:
    data = json.load(r)

assets = data.get("assets", [])
for pattern in patterns:
    for asset in assets:
        name = asset.get("name", "")
        if pattern not in name:
            continue
        if pattern == "ubuntu-x64" and any(tag in name for tag in ("rocm", "vulkan", "opencl", "s390x")):
            continue
        print(asset["browser_download_url"])
        raise SystemExit(0)

raise SystemExit(f"No matching llama.cpp Linux asset found for patterns {patterns}")
PY
}

repair_linux_runtime_links() {
  local dir="$1"

  local pairs=(
    "libmtmd.so.0:libmtmd.so.0."
    "libllama.so.0:libllama.so.0."
    "libggml.so.0:libggml.so.0."
    "libggml-base.so.0:libggml-base.so.0."
  )

  local pair
  for pair in "${pairs[@]}"; do
    local link_name="${pair%%:*}"
    local prefix="${pair#*:}"
    local link_path="$dir/$link_name"

    if [[ -L "$link_path" ]]; then
      continue
    fi

    if [[ -f "$link_path" && "$(stat -c%s "$link_path" 2>/dev/null || echo 0)" -gt 1000000 ]]; then
      continue
    fi

    rm -f "$link_path"

    local target_name
    target_name="$(
      find "$dir" -maxdepth 1 -type f -name "${prefix}*" -printf '%f\n' | sort | tail -n 1
    )"

    if [[ -z "$target_name" ]]; then
      echo "Missing shared library matching ${prefix}" >&2
      exit 1
    fi

    ln -s "$target_name" "$link_path"
  done
}

install_llama_server() {
  local backend_override="${KEYTOMUSIC_LLAMA_BACKEND:-}"
  local want_vulkan=0
  if [[ "$backend_override" == "vulkan" ]]; then
    want_vulkan=1
  fi

  if [[ "$FORCE_LLAMA_SERVER" -eq 1 ]]; then
    echo "Forcing llama-server reinstall from the latest release..."
    rm -rf "$LLAMA_DIR"
  fi

  if min_size_ok "$LLAMA_BIN" 1000000; then
    if [[ "$want_vulkan" -eq 1 && ! -f "$LLAMA_DIR/libggml-vulkan.so" ]]; then
      echo "Existing llama-server is not a Vulkan build, reinstalling..."
      rm -rf "$LLAMA_DIR"
      mkdir -p "$LLAMA_DIR"
    else
      echo "llama-server already installed: $LLAMA_BIN"
      repair_linux_runtime_links "$LLAMA_DIR"
      return
    fi
  fi

  if min_size_ok "$LLAMA_BIN" 1000000; then
    echo "llama-server already installed: $LLAMA_BIN"
    repair_linux_runtime_links "$LLAMA_DIR"
    return
  fi

  mkdir -p "$LLAMA_DIR"

  echo "Resolving latest llama.cpp Linux release..."
  local release_url
  release_url="$(resolve_linux_release_url)"

  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN

  echo "Downloading llama-server archive..."
  curl --fail --location --progress-bar "$release_url" -o "$tmp_dir/llama-server.tar.gz"

  mkdir -p "$tmp_dir/extract"
  tar -xzf "$tmp_dir/llama-server.tar.gz" -C "$tmp_dir/extract"

  find "$tmp_dir/extract" -type f -print0 | while IFS= read -r -d '' file; do
    cp "$file" "$LLAMA_DIR/$(basename "$file")"
  done

  repair_linux_runtime_links "$LLAMA_DIR"
  chmod +x "$LLAMA_BIN"
  trap - RETURN
  rm -rf "$tmp_dir"

  if ! min_size_ok "$LLAMA_BIN" 1000000; then
    echo "llama-server install failed: $LLAMA_BIN missing or too small" >&2
    exit 1
  fi

  echo "llama-server installed: $LLAMA_BIN"
}

install_models() {
  mkdir -p "$MODEL_DIR"

  if min_size_ok "$MODEL_PATH" 1500000000; then
    echo "Model already present: $MODEL_PATH"
  else
    download_file "$MODEL_URL" "$MODEL_PATH"
  fi

  if min_size_ok "$MMPROJ_PATH" 500000000; then
    echo "mmproj already present: $MMPROJ_PATH"
  else
    download_file "$MMPROJ_URL" "$MMPROJ_PATH"
  fi
}

install_optional_model() {
  local label="$1"
  local dir="$2"
  local model_path="$3"
  local mmproj_path="$4"
  local model_url="$5"
  local mmproj_url="$6"
  local model_min_size="$7"
  local mmproj_min_size="$8"

  mkdir -p "$dir"

  if min_size_ok "$model_path" "$model_min_size"; then
    echo "$label model already present: $model_path"
  else
    download_file "$model_url" "$model_path"
  fi

  if min_size_ok "$mmproj_path" "$mmproj_min_size"; then
    echo "$label mmproj already present: $mmproj_path"
  else
    download_file "$mmproj_url" "$mmproj_path"
  fi
}

install_optional_models() {
  if [[ "$INSTALL_GEMMA4_E4B" -eq 1 ]]; then
    install_optional_model \
      "Gemma 4 E4B" \
      "$GEMMA4_E4B_DIR" \
      "$GEMMA4_E4B_MODEL_PATH" \
      "$GEMMA4_E4B_MMPROJ_PATH" \
      "$GEMMA4_E4B_MODEL_URL" \
      "$GEMMA4_E4B_MMPROJ_URL" \
      4500000000 \
      900000000
  fi

  if [[ "$INSTALL_GEMMA4_E2B" -eq 1 ]]; then
    install_optional_model \
      "Gemma 4 E2B" \
      "$GEMMA4_E2B_DIR" \
      "$GEMMA4_E2B_MODEL_PATH" \
      "$GEMMA4_E2B_MMPROJ_PATH" \
      "$GEMMA4_E2B_MODEL_URL" \
      "$GEMMA4_E2B_MMPROJ_URL" \
      2800000000 \
      900000000
  fi
}

main() {
  parse_args "$@"

  if [[ "$INSTALL_GEMMA4_E4B" -eq 1 || "$INSTALL_GEMMA4_E2B" -eq 1 ]]; then
    FORCE_LLAMA_SERVER=1
  fi

  echo "Installing RealTest benchmark dependencies into: $APP_DATA_DIR"
  echo "This installs the current benchmark setup: llama-server + Qwen3-VL-4B-Thinking."
  echo "Benchmark models are stored in: $MODEL_DIR"

  install_llama_server
  install_models
  install_optional_models

  cat <<EOF

Done.

You can now rerun:
  KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 REALTEST_FILTER=BL/1 REALTEST_EXPERIMENTS=t4b_wide5_selective_hold cargo run --manifest-path src-tauri/Cargo.toml

Or start the resumable Production runner:
  python3 scripts/run-realtest-prod-benchmark.py start

Gemma 4 smoke run examples:
  KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 REALTEST_FILTER=BL/1 REALTEST_PAGE_LIMIT=1 REALTEST_EXPERIMENTS=gemma4_e4b_wide5_selective_hold cargo run --manifest-path src-tauri/Cargo.toml
  KEYTOMUSIC_RUN_REALTEST_BENCHMARK=1 REALTEST_FILTER=BL/1 REALTEST_PAGE_LIMIT=1 REALTEST_EXPERIMENTS=gemma4_e2b_wide5_selective_hold cargo run --manifest-path src-tauri/Cargo.toml

Installed files:
  $LLAMA_BIN
  $MODEL_PATH
  $MMPROJ_PATH
EOF
  if [[ "$INSTALL_GEMMA4_E4B" -eq 1 ]]; then
    cat <<EOF
  $GEMMA4_E4B_MODEL_PATH
  $GEMMA4_E4B_MMPROJ_PATH
EOF
  fi

  if [[ "$INSTALL_GEMMA4_E2B" -eq 1 ]]; then
    cat <<EOF
  $GEMMA4_E2B_MODEL_PATH
  $GEMMA4_E2B_MMPROJ_PATH
EOF
  fi
}

main "$@"
