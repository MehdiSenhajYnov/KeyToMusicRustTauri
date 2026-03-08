use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};

use crate::mood::director::MoodScores;
use crate::types::{BaseMood, MoodCategory, MoodIntensity, MoodTag};

use super::llama_manager;

/// Dimensional prompt: 8 base moods + 3 intensity levels.
pub(crate) const MOOD_INTENSITY_PROMPT: &str = "\
Analyze this manga page step by step:
1. What are the characters expressing? (faces, posture, gestures)
2. What feeling does the author want the reader to experience?
3. Classify as ONE mood from the list below.
4. Rate the intensity from 1 (low) to 3 (high).

Moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery

Reply format: mood intensity
Example: tension 2";

pub const ACTIVE_MOOD_MODEL_NAME: &str = "Qwen3-VL-4B-Thinking";

/// Structured features extracted from a manga page (Layer 1).
/// Universal extraction — does NOT depend on any user-defined categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageFeatures {
    /// Dominant emotion (joy, sadness, anger, fear, determination, shock, nostalgia, neutral)
    pub emotion: String,
    /// Emotional intensity (1-10)
    pub intensity: u8,
    /// Narrative mode (present, flashback, dream, thought)
    pub narrative: String,
    /// Visual atmosphere in 2-3 words
    pub atmosphere: String,
    /// Content summary in 1 sentence
    pub content: String,
}

/// Hybrid result: mood label + structured features from a single VLM inference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub mood: MoodCategory,
    pub features: PageFeatures,
}

/// A running llama-server instance.
pub struct LlamaServer {
    process: Child,
    pub port: u16,
}

#[derive(Debug, Clone, Default)]
pub struct LlamaServerStartOptions {
    pub reasoning_format: Option<String>,
    pub context_size: Option<u32>,
    pub parallel_slots: Option<u32>,
    pub gpu_layers: Option<u32>,
    pub runtime_intent: Option<LlamaRuntimeIntent>,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum LlamaRuntimeIntent {
    #[default]
    AppDefault,
    BenchmarkPrimary,
    ResearchLarge,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct ResolvedRuntimeProfile {
    context_size: u32,
    parallel_slots: u32,
    gpu_layers: u32,
}

fn parse_env_u32(key: &str) -> Option<u32> {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
}

fn model_size_bytes(model_path: &str) -> Option<u64> {
    std::fs::metadata(Path::new(model_path))
        .ok()
        .map(|meta| meta.len())
}

fn runtime_intent_name(intent: LlamaRuntimeIntent) -> &'static str {
    match intent {
        LlamaRuntimeIntent::AppDefault => "app_default",
        LlamaRuntimeIntent::BenchmarkPrimary => "benchmark_primary",
        LlamaRuntimeIntent::ResearchLarge => "research_large",
    }
}

fn derive_runtime_intent(options: &LlamaServerStartOptions) -> LlamaRuntimeIntent {
    if let Some(intent) = options.runtime_intent {
        intent
    } else if options.context_size.unwrap_or(0) >= 32768 || options.parallel_slots.unwrap_or(0) > 1
    {
        LlamaRuntimeIntent::ResearchLarge
    } else {
        LlamaRuntimeIntent::AppDefault
    }
}

fn default_context_size(intent: LlamaRuntimeIntent, _model_bytes: Option<u64>) -> u32 {
    if let Some(value) = parse_env_u32("KEYTOMUSIC_LLAMA_CONTEXT_SIZE") {
        return value;
    }

    match intent {
        LlamaRuntimeIntent::AppDefault => 8_192,
        LlamaRuntimeIntent::BenchmarkPrimary => 8_192,
        LlamaRuntimeIntent::ResearchLarge => 32_768,
    }
}

fn default_parallel_slots(intent: LlamaRuntimeIntent) -> u32 {
    if let Some(value) = parse_env_u32("KEYTOMUSIC_LLAMA_PARALLEL") {
        return value.max(1);
    }

    match intent {
        LlamaRuntimeIntent::ResearchLarge => 4,
        LlamaRuntimeIntent::AppDefault | LlamaRuntimeIntent::BenchmarkPrimary => 1,
    }
}

fn min_supported_vram_mib(model_bytes: Option<u64>) -> u32 {
    match model_bytes.unwrap_or_default() {
        0 => 6_144,
        bytes if bytes <= 3_000_000_000 => 4_096,
        bytes if bytes <= 6_000_000_000 => 6_144,
        _ => 8_192,
    }
}

fn detect_total_vram_mib() -> Option<u32> {
    if let Some(value) = parse_env_u32("KEYTOMUSIC_LLAMA_VRAM_MIB") {
        return Some(value);
    }

    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next()?.trim().parse::<u32>().ok()
}

fn choose_gpu_layers(model_path: &str, explicit_layers: Option<u32>) -> Result<u32, String> {
    if let Some(value) = explicit_layers.or_else(|| parse_env_u32("KEYTOMUSIC_LLAMA_GPU_LAYERS")) {
        return Ok(value);
    }

    let model_bytes = model_size_bytes(model_path);
    let min_vram = min_supported_vram_mib(model_bytes);
    let detected_vram = detect_total_vram_mib();

    if let Some(total_vram) = detected_vram {
        if total_vram < min_vram {
            return Err(format!(
                "GPU memory too low for manga mood ({} MiB detected, need roughly >= {} MiB).",
                total_vram, min_vram
            ));
        }

        let layers = match total_vram {
            v if v >= 20_480 => 99,
            v if v >= 16_384 => 80,
            v if v >= 12_288 => 64,
            v if v >= 10_240 => 48,
            v if v >= 8_192 => 32,
            v if v >= 6_144 => 24,
            _ => 16,
        };
        return Ok(layers);
    }

    let conservative = match model_bytes.unwrap_or_default() {
        0 => 32,
        bytes if bytes <= 3_000_000_000 => 64,
        bytes if bytes <= 6_000_000_000 => 48,
        _ => 32,
    };
    Ok(conservative)
}

fn gpu_layer_fallbacks(initial: u32) -> Vec<u32> {
    let mut values = vec![initial];
    for candidate in [80, 64, 48, 32, 24, 16] {
        if candidate < initial {
            values.push(candidate);
        }
    }
    values
}

fn context_fallbacks(initial: u32) -> Vec<u32> {
    let mut values = vec![initial];
    for candidate in [12_288, 8_192, 4_096] {
        if candidate < initial {
            values.push(candidate);
        }
    }
    values
}

fn build_runtime_candidates(
    base: ResolvedRuntimeProfile,
    lock_context: bool,
    lock_parallel: bool,
    lock_gpu_layers: bool,
) -> Vec<ResolvedRuntimeProfile> {
    let context_variants = if lock_context {
        vec![base.context_size]
    } else {
        context_fallbacks(base.context_size)
    };
    let parallel_variants = if lock_parallel || base.parallel_slots == 1 {
        vec![base.parallel_slots]
    } else {
        vec![base.parallel_slots, 1]
    };
    let gpu_variants = if lock_gpu_layers {
        vec![base.gpu_layers]
    } else {
        gpu_layer_fallbacks(base.gpu_layers)
    };

    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    for context_size in context_variants {
        for parallel_slots in &parallel_variants {
            for gpu_layers in &gpu_variants {
                let candidate = ResolvedRuntimeProfile {
                    context_size,
                    parallel_slots: *parallel_slots,
                    gpu_layers: *gpu_layers,
                };
                if seen.insert(candidate) {
                    ordered.push(candidate);
                }
            }
        }
    }
    ordered
}

fn resolve_runtime_profile(
    model_path: &str,
    options: &LlamaServerStartOptions,
) -> Result<ResolvedRuntimeProfile, String> {
    let intent = derive_runtime_intent(options);
    let model_bytes = model_size_bytes(model_path);
    let context_size = options
        .context_size
        .unwrap_or_else(|| default_context_size(intent, model_bytes));
    let parallel_slots = options
        .parallel_slots
        .unwrap_or_else(|| default_parallel_slots(intent));
    let gpu_layers = choose_gpu_layers(model_path, options.gpu_layers)?;

    Ok(ResolvedRuntimeProfile {
        context_size,
        parallel_slots: parallel_slots.max(1),
        gpu_layers,
    })
}

#[cfg(target_os = "linux")]
pub(crate) fn lower_process_priority(pid: u32) {
    let pid = pid.to_string();

    let _ = std::process::Command::new("renice")
        .args(["+10", "-p", &pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let _ = std::process::Command::new("ionice")
        .args(["-c", "2", "-n", "7", "-p", &pid])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(target_os = "linux"))]
pub(crate) fn lower_process_priority(_pid: u32) {}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn lower_current_process_priority() {
    lower_process_priority(std::process::id());
}

impl LlamaServer {
    async fn start_once_with_profile(
        server_path: &Path,
        model_path: &str,
        mmproj_path: &str,
        reasoning_format: Option<&str>,
        runtime: ResolvedRuntimeProfile,
        log_path: &Path,
    ) -> Result<Self, String> {
        let port = find_free_port().map_err(|e| format!("Failed to find free port: {}", e))?;
        let stderr_file = std::fs::File::create(log_path)
            .map_err(|e| format!("Failed to create llama-server log: {}", e))?;

        tracing::info!(
            "Starting llama-server: {:?} -m {} --mmproj {} --port {} -c {} -np {} -ngl {}",
            server_path,
            model_path,
            mmproj_path,
            port,
            runtime.context_size,
            runtime.parallel_slots,
            runtime.gpu_layers
        );

        let mut cmd = Command::new(server_path);
        cmd.args([
            "-m",
            model_path,
            "--mmproj",
            mmproj_path,
            "--port",
            &port.to_string(),
            "-c",
            &runtime.context_size.to_string(),
            "-np",
            &runtime.parallel_slots.to_string(),
            "--flash-attn",
            "auto",
            "--cache-type-k",
            "q8_0",
            "--cache-type-v",
            "q8_0",
            "-ngl",
            &runtime.gpu_layers.to_string(),
            "--image-min-tokens",
            "1024",
        ]);

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file));

        if let Some(reasoning_format) = reasoning_format {
            cmd.args(["--reasoning-format", reasoning_format]);
        }

        #[cfg(target_os = "windows")]
        {
            // CREATE_NO_WINDOW (0x08000000) | BELOW_NORMAL_PRIORITY_CLASS (0x00004000)
            cmd.creation_flags(0x08004000);
        }

        let process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start llama-server: {}", e))?;

        if let Some(pid) = process.id() {
            lower_process_priority(pid);
        }

        let mut server = Self { process, port };
        if let Err(err) = server.wait_for_ready(120).await {
            server.stop();
            return Err(err);
        }

        tracing::info!("llama-server started on port {}", port);
        Ok(server)
    }

    /// Start llama-server with the given model and mmproj paths.
    pub async fn start(model_path: &str, mmproj_path: &str) -> Result<Self, String> {
        Self::start_with_options(
            model_path,
            mmproj_path,
            LlamaServerStartOptions {
                reasoning_format: reasoning_format_from_env(),
                context_size: None,
                parallel_slots: None,
                gpu_layers: None,
                runtime_intent: Some(LlamaRuntimeIntent::ResearchLarge),
            },
        )
        .await
    }

    /// Start llama-server with explicit startup options.
    pub async fn start_with_options(
        model_path: &str,
        mmproj_path: &str,
        options: LlamaServerStartOptions,
    ) -> Result<Self, String> {
        llama_manager::ensure_mood_runtime_supported()?;

        let server_path = llama_manager::find_llama_server()
            .ok_or_else(|| "llama-server not installed".to_string())?;

        let log_dir = crate::storage::config::get_app_data_dir().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let log_path = log_dir.join("llama-server.log");
        let runtime_intent = derive_runtime_intent(&options);
        let base_profile = resolve_runtime_profile(model_path, &options)?;
        let lock_context = options.context_size.is_some()
            || parse_env_u32("KEYTOMUSIC_LLAMA_CONTEXT_SIZE").is_some();
        let lock_parallel = options.parallel_slots.is_some()
            || parse_env_u32("KEYTOMUSIC_LLAMA_PARALLEL").is_some();
        let lock_gpu_layers =
            options.gpu_layers.is_some() || parse_env_u32("KEYTOMUSIC_LLAMA_GPU_LAYERS").is_some();
        let runtime_candidates =
            build_runtime_candidates(base_profile, lock_context, lock_parallel, lock_gpu_layers);

        tracing::info!(
            "Resolved llama runtime: intent={} base={:?} candidates={:?}",
            runtime_intent_name(runtime_intent),
            base_profile,
            runtime_candidates
        );

        let mut failures = Vec::new();
        for (index, runtime) in runtime_candidates.iter().enumerate() {
            tracing::info!(
                "Trying llama runtime candidate {}/{}: {:?}",
                index + 1,
                runtime_candidates.len(),
                runtime
            );

            match Self::start_once_with_profile(
                &server_path,
                model_path,
                mmproj_path,
                options.reasoning_format.as_deref(),
                *runtime,
                &log_path,
            )
            .await
            {
                Ok(server) => return Ok(server),
                Err(err) => {
                    tracing::warn!(
                        "llama-server startup failed for candidate {:?}: {}",
                        runtime,
                        err
                    );
                    failures.push(format!(
                        "c={} np={} ngl={} => {}",
                        runtime.context_size, runtime.parallel_slots, runtime.gpu_layers, err
                    ));
                }
            }
        }

        Err(format!(
            "Failed to start llama-server after {} runtime candidate(s): {}",
            runtime_candidates.len(),
            failures.join(" | ")
        ))
    }

    /// Poll GET /health until the server responds 200.
    async fn wait_for_ready(&mut self, timeout_secs: u64) -> Result<(), String> {
        let url = format!("http://127.0.0.1:{}/health", self.port);
        let client = reqwest::Client::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        tracing::info!(
            "Waiting for llama-server to be ready (timeout {}s)...",
            timeout_secs
        );

        loop {
            // Check if process crashed
            if let Ok(Some(status)) = self.process.try_wait() {
                let log_path = crate::storage::config::get_app_data_dir()
                    .join("logs")
                    .join("llama-server.log");
                let stderr = std::fs::read_to_string(&log_path).unwrap_or_default();
                let last_lines: String = stderr
                    .lines()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
                    .join("\n");
                tracing::error!(
                    "llama-server process exited with {}\nLast log lines:\n{}",
                    status,
                    last_lines
                );
                return Err(format!(
                    "llama-server crashed on startup (exit {}). Check logs at {:?}",
                    status, log_path
                ));
            }

            if tokio::time::Instant::now() > deadline {
                tracing::error!("llama-server failed to respond within {}s", timeout_secs);
                return Err("llama-server failed to start within timeout".to_string());
            }

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    tracing::info!("llama-server is ready (health check OK)");
                    return Ok(());
                }
                Ok(resp) => {
                    tracing::debug!(
                        "llama-server health check returned {}, retrying...",
                        resp.status()
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    tracing::debug!("llama-server health check failed: {}, retrying...", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Analyze a manga page image and detect the mood + intensity (dimensional system).
    pub async fn analyze_mood(&self, image_base64: &str) -> Result<MoodTag, String> {
        tracing::info!(
            "analyze_mood: sending image ({} bytes b64) to llama-server port {}",
            image_base64.len(),
            self.port
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/jpeg;base64,{}", image_base64)
                            }
                        },
                        {
                            "type": "text",
                            "text": MOOD_INTENSITY_PROMPT
                        }
                    ]
                }
            ],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        tracing::debug!("analyze_mood: POST {}", url);
        let response: reqwest::Response =
            client.post(&url).json(&body).send().await.map_err(|e| {
                tracing::error!(
                    "analyze_mood: failed to send request to llama-server: {}",
                    e
                );
                format!("Failed to send to llama-server: {}", e)
            })?;

        tracing::debug!(
            "analyze_mood: llama-server responded with HTTP {}",
            response.status()
        );

        if !response.status().is_success() {
            let status = response.status();
            let text: String = response.text().await.unwrap_or_default();
            tracing::error!("analyze_mood: llama-server error HTTP {}: {}", status, text);
            return Err(format!("llama-server returned HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response.json::<serde_json::Value>().await.map_err(|e| {
            tracing::error!("analyze_mood: failed to parse JSON response: {}", e);
            format!("Failed to parse response: {}", e)
        })?;

        tracing::debug!(
            "analyze_mood: raw response: {}",
            serde_json::to_string(&json).unwrap_or_default()
        );

        let result = parse_mood_intensity_response(&json);
        match &result {
            Ok(tag) => tracing::info!("analyze_mood: detected {:?} {:?}", tag.mood, tag.intensity),
            Err(e) => tracing::error!("analyze_mood: failed to parse mood from response: {}", e),
        }
        result
    }

    /// Pipeline V2 — Stage 1: VLM describes the page visually (no classification).
    ///
    /// Returns a 2-3 sentence textual description of the manga page.
    /// The VLM only describes what it sees — expressions, actions, atmosphere.
    pub async fn describe_page(&self, image_base64: &str) -> Result<String, String> {
        tracing::info!(
            "describe_page: sending image ({} bytes b64) to llama-server port {}",
            image_base64.len(),
            self.port
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/jpeg;base64,{}", image_base64)
                            }
                        },
                        {
                            "type": "text",
                            "text": "Describe this manga page in detail. Focus on the emotions conveyed, the characters' feelings, and the overall atmosphere. What would a reader feel looking at this page?"
                        }
                    ]
                }
            ],
            "max_tokens": 512,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("describe_page: request failed: {}", e);
            format!("Failed to send to llama-server: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server returned HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        // Strip </think> tags if present
        let text = if let Some(pos) = content.find("</think>") {
            content[pos + 8..].trim()
        } else {
            content.trim()
        };

        tracing::info!("describe_page: got {} chars description", text.len());
        Ok(text.to_string())
    }

    /// Summarize multiple page descriptions into a concise narrative arc (2-3 sentences).
    /// Text-only inference, no image needed. Fast (~1-2s).
    pub async fn summarize_descriptions(
        &self,
        descriptions: &[(u32, &str)], // [(page_num, full_description)]
    ) -> Result<String, String> {
        if descriptions.is_empty() {
            return Ok(String::new());
        }

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let mut input = String::from("Here are descriptions of consecutive manga pages:\n\n");
        for (page_num, desc) in descriptions {
            use std::fmt::Write;
            let _ = writeln!(input, "Page {}: {}\n", page_num, desc);
        }

        let prompt = format!(
            "{}\nSummarize the narrative arc of these pages in 2-3 sentences. \
             Focus on: what is happening, the emotional progression, and the atmosphere. \
             Be factual and concise.",
            input
        );

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 300,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("summarize request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        // Strip </think> tags if present
        let text = if let Some(pos) = content.find("</think>") {
            content[pos + 8..].trim()
        } else {
            content.trim()
        };

        Ok(text.to_string())
    }

    /// V6: Classify mood with image + contextual descriptions of previous pages.
    /// Unlike text-only classify, the VLM sees the current image.
    /// Unlike mood-label injection, the context is factual descriptions (no bias).
    pub async fn classify_with_context(
        &self,
        image_base64: &str,
        previous_descriptions: &[(u32, &str)], // [(page_num, description)] — full detail, nearby
        next_descriptions: &[(u32, &str)],     // [(page_num, description)] — empty = V6 behavior
        narrative_summary: &str,               // single summary string, empty = no arc
    ) -> Result<MoodTag, String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        // Build context block: arc summary → detailed (nearby) → future
        let context_block = {
            use std::fmt::Write;
            let mut block = String::new();
            if !narrative_summary.is_empty() {
                block.push_str("Narrative arc of earlier pages:\n");
                block.push_str(narrative_summary);
                block.push_str("\n\n");
            }
            if !previous_descriptions.is_empty() {
                block.push_str("Previous pages (detailed):\n");
                for (page_num, desc) in previous_descriptions {
                    let _ = writeln!(block, "- Page {}: \"{}\"", page_num, desc);
                }
                block.push('\n');
            }
            if !next_descriptions.is_empty() {
                block.push_str("Upcoming pages:\n");
                for (page_num, desc) in next_descriptions {
                    let _ = writeln!(block, "- Page {}: {}", page_num, desc);
                }
                block.push('\n');
            }
            block
        };

        let prompt_text = format!(
            "{}Look at this manga page. Based on what you see AND the narrative context above, \
             what is the mood of THIS page for soundtrack purposes?\n\
             \n\
             Analyze step by step:\n\
             1. What are the characters expressing? (faces, posture, gestures)\n\
             2. What feeling does the author want the reader to experience?\n\
             3. Classify as ONE mood from the list below.\n\
             4. Rate the intensity from 1 (low) to 3 (high).\n\
             \n\
             Moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery\n\
             \n\
             Reply format: mood intensity\n\
             Example: tension 2",
            context_block
        );

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:image/jpeg;base64,{}", image_base64)
                            }
                        },
                        {
                            "type": "text",
                            "text": prompt_text
                        }
                    ]
                }
            ],
            "max_tokens": 5000,
            "temperature": 0.1
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("classify_with_context: request failed: {}", e);
            format!("classify_with_context request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        parse_mood_intensity_response(&json)
    }

    /// Pipeline V2 — Stage 2: Text-only batch classification of page descriptions.
    ///
    /// Takes N page descriptions and classifies them all in a single inference call.
    /// The LLM sees the full narrative sequence and can reason about arcs.
    pub async fn classify_batch(
        &self,
        descriptions: &[(u32, &str)],
    ) -> Result<Vec<(u32, MoodCategory)>, String> {
        if descriptions.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "classify_batch: classifying {} page descriptions",
            descriptions.len()
        );

        let mut page_lines = String::new();
        for (page_num, desc) in descriptions {
            use std::fmt::Write;
            let _ = writeln!(page_lines, "Page {}: \"{}\"", page_num, desc);
        }

        let prompt = format!(
            "You are a manga soundtrack director. Based on these page descriptions, \
             assign a mood to each page for soundtrack purposes.\n\
             \n\
             {}\
             \n\
             Categories: epic, tension, sadness, comedy, romance, horror, \
             peaceful (calm daily life), mystery\n\
             \n\
             Keep the base mood even when the scene is very intense. \
             Intensity is handled separately from the mood label.\n\
             \n\
             For each page, output: PAGE N: mood",
            page_lines
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("classify_batch: request failed: {}", e);
            format!("classify_batch request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("classify_batch HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse classify_batch response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in classify_batch response".to_string())?;

        // Strip </think> tags
        let text = if let Some(pos) = content.find("</think>") {
            &content[pos + 8..]
        } else {
            content
        };

        tracing::info!("classify_batch: response:\n{}", text);

        // Parse "PAGE N: mood" lines
        let re = regex::Regex::new(r"(?i)(?:PAGE|MOOD)\s+(\d+)\s*:\s*(?:mood\s*=\s*)?(\w+)")
            .map_err(|e| format!("Regex error: {}", e))?;

        let mut classified: Vec<(u32, MoodCategory)> = Vec::new();
        for cap in re.captures_iter(text) {
            if let (Ok(num), Some(mood)) = (
                cap[1].parse::<u32>(),
                MoodCategory::from_str_opt(&cap[2].to_lowercase()),
            ) {
                classified.push((num, mood));
            }
        }

        if classified.is_empty() {
            return Err(format!(
                "No moods parsed from classify_batch: '{}'",
                &text[..text.len().min(300)]
            ));
        }

        tracing::info!(
            "classify_batch: parsed {}/{} pages",
            classified.len(),
            descriptions.len()
        );

        Ok(classified)
    }

    /// Two-pass refinement: text-only batch pass that refines Pass 1 mood classifications
    /// based on narrative coherence across all pages.
    ///
    /// Takes Pass 1 MoodScores for every page and returns refined moods.
    /// Uses text-only inference (no image) — same llama-server, zero extra VRAM.
    pub async fn refine_moods_batch(
        &self,
        page_results: &[(u32, MoodScores)],
    ) -> Result<Vec<(u32, MoodCategory)>, String> {
        if page_results.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "refine_moods_batch: refining {} pages via text-only pass",
            page_results.len()
        );

        // Format top-3 moods per page
        let mut page_lines = String::new();
        for (page_num, scores) in page_results {
            let mut sorted: Vec<(MoodCategory, f32)> = MoodCategory::ALL
                .iter()
                .map(|&m| (m, scores.get(m)))
                .collect();
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let top3: String = sorted
                .iter()
                .take(3)
                .map(|(m, s)| format!("{}={:.2}", m.as_str(), s))
                .collect::<Vec<_>>()
                .join(", ");

            use std::fmt::Write;
            let _ = writeln!(page_lines, "Page {}: {}", page_num, top3);
        }

        let prompt = format!(
            "You are a narrative mood director for manga soundtrack selection.\n\
             \n\
             Below are visual mood analysis results for consecutive manga pages.\n\
             Each page shows the top 3 detected moods with confidence scores (0.0-1.0).\n\
             \n\
             {}\
             \n\
             For each page, determine the FINAL mood for soundtrack selection.\n\
             \n\
             Guidelines:\n\
             - High confidence (dominant score > 0.55): keep the detected mood\n\
             - Low confidence (< 0.35): use surrounding context to resolve\n\
             - Common manga arcs: mystery/peaceful → tension → epic → sadness → peaceful\n\
             - Same mood typically persists for 3-8 consecutive pages\n\
             - Preserve genuine transitions — don't over-smooth\n\
             \n\
             Reply with EXACTLY one line per page:\n\
             PAGE [number]: [mood_name]\n\
             \n\
             Available moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery",
            page_lines
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("refine_moods_batch: request failed: {}", e);
            format!("Refinement request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Refinement HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refinement response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in refinement response".to_string())?;

        // Remove thinking tags
        let text = if let Some(pos) = content.find("</think>") {
            &content[pos + 8..]
        } else {
            content
        };

        tracing::debug!("refine_moods_batch: response:\n{}", text);

        // Parse "PAGE N: mood" lines
        let re = regex::Regex::new(r"(?i)(?:PAGE|MOOD)\s+(\d+)\s*:\s*(?:mood\s*=\s*)?(\w+)")
            .map_err(|e| format!("Regex error: {}", e))?;

        let mut refined: Vec<(u32, MoodCategory)> = Vec::new();
        for cap in re.captures_iter(text) {
            if let (Ok(num), Some(mood)) = (
                cap[1].parse::<u32>(),
                MoodCategory::from_str_opt(&cap[2].to_lowercase()),
            ) {
                refined.push((num, mood));
            }
        }

        if refined.is_empty() {
            return Err(format!(
                "No moods parsed from refinement: '{}'",
                &text[..text.len().min(300)]
            ));
        }

        tracing::info!(
            "refine_moods_batch: parsed {}/{} pages",
            refined.len(),
            page_results.len()
        );

        Ok(refined)
    }

    /// Two-pass refinement (label-based): takes single-label Pass 1 results and
    /// uses text-only narrative reasoning to refine them.
    ///
    /// Input: `(page_number, detected_mood_str)` for each page.
    /// Returns: refined mood per page.
    pub async fn refine_moods_from_labels(
        &self,
        page_labels: &[(u32, &str)],
    ) -> Result<Vec<(u32, MoodCategory)>, String> {
        if page_labels.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "refine_moods_from_labels: refining {} pages via text-only narrative pass",
            page_labels.len()
        );

        let mut page_lines = String::new();
        for (page_num, mood) in page_labels {
            use std::fmt::Write;
            let _ = writeln!(page_lines, "Page {}: {}", page_num, mood);
        }

        let prompt = format!(
            "You are a narrative mood director for manga soundtrack selection.\n\
             \n\
             Below are visual mood analysis results for consecutive manga pages, \
             each analyzed independently by a vision AI model.\n\
             \n\
             {}\
             \n\
             For each page, determine the FINAL mood for soundtrack selection.\n\
             \n\
             Guidelines:\n\
             - The visual analysis is usually correct — only change if narrative context clearly suggests an error\n\
             - Isolated outliers (one different mood surrounded by 3+ pages of the same mood) are likely errors\n\
             - Common manga arcs: mystery/peaceful → tension → epic → sadness → peaceful\n\
             - Same mood typically persists for 3-8 consecutive pages\n\
             - Genuine transitions should be preserved — don't over-smooth\n\
             - When unsure, keep the original visual analysis result\n\
             \n\
             Reply with EXACTLY one line per page:\n\
             PAGE [number]: [mood_name]\n\
             \n\
             Available moods: epic, tension, sadness, comedy, romance, horror, peaceful, mystery",
            page_lines
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("refine_moods_from_labels: request failed: {}", e);
            format!("Refinement request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Refinement HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse refinement response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in refinement response".to_string())?;

        // Remove thinking tags
        let text = if let Some(pos) = content.find("</think>") {
            &content[pos + 8..]
        } else {
            content
        };

        tracing::info!("refine_moods_from_labels: response:\n{}", text);

        // Parse "PAGE N: mood" lines
        let re = regex::Regex::new(r"(?i)(?:PAGE|MOOD)\s+(\d+)\s*:\s*(?:mood\s*=\s*)?(\w+)")
            .map_err(|e| format!("Regex error: {}", e))?;

        let mut refined: Vec<(u32, MoodCategory)> = Vec::new();
        for cap in re.captures_iter(text) {
            if let (Ok(num), Some(mood)) = (
                cap[1].parse::<u32>(),
                MoodCategory::from_str_opt(&cap[2].to_lowercase()),
            ) {
                refined.push((num, mood));
            }
        }

        if refined.is_empty() {
            return Err(format!(
                "No moods parsed from refinement: '{}'",
                &text[..text.len().min(300)]
            ));
        }

        tracing::info!(
            "refine_moods_from_labels: parsed {}/{} pages",
            refined.len(),
            page_labels.len()
        );

        Ok(refined)
    }

    /// Pipeline V5 — Correct proposed moods using narrative context from descriptions.
    ///
    /// Takes per-page mood labels (from VLM) + descriptions (from VLM) and asks
    /// a text-only LLM to confirm or correct each mood based on the full sequence.
    pub async fn correct_moods_batch(
        &self,
        pages: &[(u32, &str, &str)], // (page_num, proposed_mood, description)
    ) -> Result<Vec<(u32, MoodCategory)>, String> {
        if pages.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "correct_moods_batch: correcting {} pages via text-only narrative pass",
            pages.len()
        );

        let mut page_lines = String::new();
        for (page_num, mood, description) in pages {
            use std::fmt::Write;
            let _ = writeln!(
                page_lines,
                "Page {}: mood={} | \"{}\"",
                page_num, mood, description
            );
        }

        let prompt = format!(
            "You are a music director for a manga reader app. A vision model analyzed each page \
             and proposed a mood for the soundtrack. It is usually correct, but it sometimes \
             confuses visual intensity with the type of scene.\n\
             \n\
             For each page you have:\n\
             - The mood proposed by the vision model\n\
             - A description of what happens on the page\n\
             \n\
             Your task: confirm the mood if correct, or correct it based on narrative context.\n\
             \n\
             Available moods: epic, tension, sadness, comedy, romance, horror, \
             peaceful, mystery\n\
             \n\
             Key distinctions:\n\
             - Intense crying/despair stays sadness\n\
             - Intense confrontation or unresolved threat stays tension until there is real payoff\n\
             - Clear payoff, triumph, or release belongs to epic\n\
             - A scene can be visually intense but emotionally sad — trust the description\n\
             - When unsure, keep the proposed mood\n\
             \n\
             Pages:\n\
             {}\
             \n\
             For each page, reply: PAGE N: mood",
            page_lines
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("correct_moods_batch: request failed: {}", e);
            format!("Correction request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("Correction HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse correction response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in correction response".to_string())?;

        // Remove thinking tags
        let text = if let Some(pos) = content.find("</think>") {
            &content[pos + 8..]
        } else {
            content
        };

        tracing::info!("correct_moods_batch: response:\n{}", text);

        // Parse "PAGE N: mood" lines
        let re = regex::Regex::new(r"(?i)(?:PAGE|MOOD)\s+(\d+)\s*:\s*(?:mood\s*=\s*)?(\w+)")
            .map_err(|e| format!("Regex error: {}", e))?;

        let mut corrected: Vec<(u32, MoodCategory)> = Vec::new();
        for cap in re.captures_iter(text) {
            if let (Ok(num), Some(mood)) = (
                cap[1].parse::<u32>(),
                MoodCategory::from_str_opt(&cap[2].to_lowercase()),
            ) {
                corrected.push((num, mood));
            }
        }

        if corrected.is_empty() {
            return Err(format!(
                "No moods parsed from correction: '{}'",
                &text[..text.len().min(300)]
            ));
        }

        tracing::info!(
            "correct_moods_batch: parsed {}/{} pages",
            corrected.len(),
            pages.len()
        );

        Ok(corrected)
    }

    /// Layer 1 — Extract structured features from a manga page image.
    ///
    /// Returns universal features (emotion, intensity, narrative, atmosphere, content)
    /// that do NOT depend on any user-defined categories.
    pub async fn extract_structured(&self, image_base64: &str) -> Result<PageFeatures, String> {
        tracing::info!(
            "extract_structured: sending image ({} bytes b64) to port {}",
            image_base64.len(),
            self.port
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": format!("data:image/jpeg;base64,{}", image_base64) }
                    },
                    {
                        "type": "text",
                        "text": "Analyze this manga page. Fill in each field:\n\n\
                            EMOTION: What is the dominant emotion shown?\n  \
                            (joy, sadness, anger, fear, surprise, neutral)\n\
                            INTENSITY: How strong? (1-10)\n\
                            NARRATIVE: Is this page showing:\n  \
                            - present (events happening now)\n  \
                            - flashback (memory of past events)\n  \
                            - dream (imagination, fantasy, what-if)\n  \
                            - thought (internal monologue, character reflecting)\n\
                            ATMOSPHERE: Visual feel in 2-3 words (e.g. \"dark, tense\", \"bright, peaceful\")\n\
                            CONTENT: What is shown in 1 sentence\n\n\
                            Reply in this exact format:\n\
                            EMOTION: ...\n\
                            INTENSITY: ...\n\
                            NARRATIVE: ...\n\
                            ATMOSPHERE: ...\n\
                            CONTENT: ..."
                    }
                ]
            }],
            "max_tokens": 512,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("extract_structured: request failed: {}", e);
            format!("Failed to send to llama-server: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server returned HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        tracing::debug!("extract_structured: raw response: {}", content);

        let result = parse_structured_response(content);
        match &result {
            Ok(feat) => tracing::info!(
                "extract_structured: emotion={}, intensity={}, narrative={}",
                feat.emotion,
                feat.intensity,
                feat.narrative
            ),
            Err(e) => tracing::error!("extract_structured: parse failed: {}", e),
        }
        result
    }

    /// Extract structured features with 8 canonical manga emotions.
    ///
    /// Similar to `extract_structured()` but with an updated emotion list
    /// (joy, sadness, anger, fear, determination, shock, nostalgia, neutral)
    /// and applies `normalize_emotion()` on the parsed result.
    pub async fn extract_features_manga(&self, image_base64: &str) -> Result<PageFeatures, String> {
        tracing::info!(
            "extract_features_manga: sending image ({} bytes b64) to port {}",
            image_base64.len(),
            self.port
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": format!("data:image/jpeg;base64,{}", image_base64) }
                    },
                    {
                        "type": "text",
                        "text": "Analyze this manga page. Fill in each field:\n\n\
                            EMOTION: What is the dominant emotion shown?\n  \
                            (joy, sadness, anger, fear, determination, shock, nostalgia, neutral)\n\
                            INTENSITY: How strong? (1-10)\n\
                            NARRATIVE: Is this page showing:\n  \
                            - present (events happening now)\n  \
                            - flashback (memory of past events)\n  \
                            - dream (imagination, fantasy, what-if)\n  \
                            - thought (internal monologue, character reflecting)\n\
                            ATMOSPHERE: Visual feel in 2-3 words (e.g. \"dark, tense\", \"bright, peaceful\")\n\
                            CONTENT: What is shown in 1 sentence\n\n\
                            Reply in this exact format:\n\
                            EMOTION: ...\n\
                            INTENSITY: ...\n\
                            NARRATIVE: ...\n\
                            ATMOSPHERE: ...\n\
                            CONTENT: ..."
                    }
                ]
            }],
            "max_tokens": 512,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("extract_features_manga: request failed: {}", e);
            format!("Failed to send to llama-server: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server returned HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        tracing::debug!("extract_features_manga: raw response: {}", content);

        let mut features = parse_structured_response(content)?;
        // Normalize emotion to canonical 8 manga emotions
        features.emotion = normalize_emotion(&features.emotion);

        tracing::info!(
            "extract_features_manga: emotion={}, intensity={}, narrative={}",
            features.emotion,
            features.intensity,
            features.narrative
        );
        Ok(features)
    }

    /// Hybrid extraction — Single VLM inference that produces both a mood label
    /// and structured features (GUIDED_V3 prompt + feature extraction).
    pub async fn extract_hybrid(&self, image_base64: &str) -> Result<HybridResult, String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);

        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": format!("data:image/jpeg;base64,{}", image_base64) }
                    },
                    {
                        "type": "text",
                        "text": "Analyze this manga page step by step:\n\
                            1. What are the characters expressing? (faces, posture, gestures)\n\
                            2. What feeling does the author want the reader to experience?\n\
                            3. Classify as ONE mood category.\n\
                            \n\
                            Categories: epic, tension, sadness, comedy, romance, horror, peaceful (calm daily life), mystery\n\
                            \n\
                            Then extract these features:\n\
                            EMOTION: (joy, sadness, anger, fear, determination, shock, nostalgia, neutral)\n\
                            INTENSITY: (1-10)\n\
                            NARRATIVE: (present, flashback, dream, thought)\n\
                            ATMOSPHERE: 2-3 words\n\
                            CONTENT: 1 sentence\n\
                            \n\
                            Reply format:\n\
                            MOOD: [category]\n\
                            EMOTION: ...\n\
                            INTENSITY: ...\n\
                            NARRATIVE: ...\n\
                            ATMOSPHERE: ...\n\
                            CONTENT: ..."
                    }
                ]
            }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("extract_hybrid: request failed: {}", e);
            format!("Failed to send to llama-server: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!("llama-server returned HTTP {}: {}", status, text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        tracing::debug!("extract_hybrid: raw response: {}", content);

        let result = parse_hybrid_response(content);
        match &result {
            Ok(hr) => tracing::info!(
                "extract_hybrid: mood={:?}, emotion={}, intensity={}, narrative={}",
                hr.mood,
                hr.features.emotion,
                hr.features.intensity,
                hr.features.narrative
            ),
            Err(e) => tracing::error!("extract_hybrid: parse failed: {}", e),
        }
        result
    }

    /// Layer 2 — Classify a batch of pages from their structured features.
    ///
    /// Takes N PageFeatures + user-defined categories (name, description).
    /// Returns one mood category per page. Sees the full sequence for narrative reasoning.
    pub async fn classify_batch_from_features(
        &self,
        features: &[(u32, &PageFeatures)],
        categories: &[(&str, &str)],
    ) -> Result<Vec<(u32, MoodCategory)>, String> {
        if features.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!(
            "classify_batch_from_features: classifying {} pages with {} categories",
            features.len(),
            categories.len()
        );

        // Build CATEGORIES section
        let mut cat_lines = String::new();
        for (name, description) in categories {
            use std::fmt::Write;
            let _ = writeln!(cat_lines, "- {}: {}", name, description);
        }

        // Build PAGES section
        let mut page_lines = String::new();
        for (page_num, feat) in features {
            use std::fmt::Write;
            let _ = writeln!(
                page_lines,
                "Page {}:  EMOTION={}({}) NARRATIVE={} ATMOSPHERE={}\n         CONTENT=\"{}\"",
                page_num,
                feat.emotion,
                feat.intensity,
                feat.narrative,
                feat.atmosphere,
                feat.content
            );
        }

        let prompt = format!(
            "You are a manga soundtrack director. Assign a soundtrack category \
             to each page based on its emotional profile and narrative context.\n\
             \n\
             CATEGORIES:\n\
             {}\
             \n\
             PAGES:\n\
             {}\
             \n\
             RULES:\n\
             - Flashback/dream/thought pages: the MOOD comes from the emotional ARC, \
             not the literal visual content. A dream of victory within a sadness \
             arc = sadness, not triumph.\n\
             - High intensity should reinforce the base mood, not create a separate category.\n\
             - Pages in the same scene usually share a mood.\n\
             - Consider the narrative flow across the full sequence.\n\
             \n\
             For each page, output: PAGE N: category",
            cat_lines, page_lines
        );

        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = serde_json::json!({
            "model": ACTIVE_MOOD_MODEL_NAME,
            "messages": [{ "role": "user", "content": prompt }],
            "max_tokens": 8192,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;

        let response = client.post(&url).json(&body).send().await.map_err(|e| {
            tracing::error!("classify_batch_from_features: request failed: {}", e);
            format!("classify_batch_from_features request failed: {}", e)
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!(
                "classify_batch_from_features HTTP {}: {}",
                status, text
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "No content in response".to_string())?;

        // Strip </think> tags
        let text = if let Some(pos) = content.find("</think>") {
            &content[pos + 8..]
        } else {
            content
        };

        tracing::info!("classify_batch_from_features: response:\n{}", text);

        // Parse "PAGE N: category" lines
        let re = regex::Regex::new(r"(?i)(?:PAGE|MOOD)\s+(\d+)\s*:\s*(?:mood\s*=\s*)?(\w+)")
            .map_err(|e| format!("Regex error: {}", e))?;

        let mut classified: Vec<(u32, MoodCategory)> = Vec::new();
        for cap in re.captures_iter(text) {
            if let (Ok(num), Some(mood)) = (
                cap[1].parse::<u32>(),
                MoodCategory::from_str_opt(&cap[2].to_lowercase()),
            ) {
                classified.push((num, mood));
            }
        }

        if classified.is_empty() {
            return Err(format!(
                "No moods parsed from classify_batch_from_features: '{}'",
                &text[..text.len().min(300)]
            ));
        }

        tracing::info!(
            "classify_batch_from_features: parsed {}/{} pages",
            classified.len(),
            features.len()
        );

        Ok(classified)
    }

    /// Stop the server process.
    pub fn stop(&mut self) {
        let _ = self.process.start_kill();
        tracing::info!("llama-server stopped");
    }

    /// Check if the server process is still running.
    pub fn is_running(&mut self) -> bool {
        matches!(self.process.try_wait(), Ok(None))
    }
}

pub(crate) fn reasoning_format_from_env() -> Option<String> {
    match std::env::var("KEYTOMUSIC_LLAMA_REASONING_FORMAT") {
        Ok(value) if matches!(value.as_str(), "" | "omit" | "default") => None,
        Ok(value) => Some(value),
        Err(_) => Some("none".to_string()),
    }
}

impl Drop for LlamaServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn research_runtime_defaults_to_parallel_four() {
        assert_eq!(default_parallel_slots(LlamaRuntimeIntent::ResearchLarge), 4);
        assert_eq!(default_parallel_slots(LlamaRuntimeIntent::AppDefault), 1);
    }

    #[test]
    fn runtime_candidates_reduce_gpu_and_context_progressively() {
        let base = ResolvedRuntimeProfile {
            context_size: 32_768,
            parallel_slots: 4,
            gpu_layers: 99,
        };

        let candidates = build_runtime_candidates(base, false, false, false);

        assert_eq!(candidates.first().copied(), Some(base));
        assert!(candidates.contains(&ResolvedRuntimeProfile {
            context_size: 32_768,
            parallel_slots: 1,
            gpu_layers: 99,
        }));
        assert!(candidates.contains(&ResolvedRuntimeProfile {
            context_size: 12_288,
            parallel_slots: 1,
            gpu_layers: 48,
        }));
    }

    #[test]
    fn runtime_intent_prefers_explicit_value() {
        let options = LlamaServerStartOptions {
            reasoning_format: None,
            context_size: Some(8_192),
            parallel_slots: Some(1),
            gpu_layers: None,
            runtime_intent: Some(LlamaRuntimeIntent::BenchmarkPrimary),
        };

        assert_eq!(
            derive_runtime_intent(&options),
            LlamaRuntimeIntent::BenchmarkPrimary
        );
    }

    #[test]
    fn extract_content_supports_segment_arrays() {
        let json = json!({
            "choices": [{
                "message": {
                    "content": [
                        { "type": "text", "text": "epic 2" }
                    ]
                }
            }]
        });

        assert_eq!(extract_content(&json).as_deref(), Some("epic 2"));
    }

    #[test]
    fn parse_mood_intensity_response_supports_segment_arrays() {
        let json = json!({
            "choices": [{
                "message": {
                    "content": [
                        { "type": "text", "text": "tension 3" }
                    ]
                }
            }]
        });

        let parsed = parse_mood_intensity_response(&json).expect("segment array should parse");
        assert_eq!(parsed.mood, BaseMood::Tension);
        assert_eq!(parsed.intensity, MoodIntensity::High);
    }
}

/// Prepare an image for analysis: read file, resize to 672px max dimension, encode as base64 JPEG.
pub fn prepare_image(image_data: &[u8]) -> Result<String, String> {
    use base64::Engine;
    use image::GenericImageView;

    tracing::debug!("prepare_image: loading {} bytes", image_data.len());

    let img = image::load_from_memory(image_data).map_err(|e| {
        tracing::error!("prepare_image: failed to load image: {}", e);
        format!("Failed to load image: {}", e)
    })?;

    // Resize to max 672px on longest side
    let (w, h) = img.dimensions();
    let max_dim = 672u32;
    let resized = if w > max_dim || h > max_dim {
        let scale = max_dim as f64 / w.max(h) as f64;
        let new_w = (w as f64 * scale) as u32;
        let new_h = (h as f64 * scale) as u32;
        tracing::info!("prepare_image: resizing {}x{} -> {}x{}", w, h, new_w, new_h);
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    } else {
        tracing::info!(
            "prepare_image: image {}x{} already within 672px, no resize needed",
            w,
            h
        );
        img
    };

    // Encode as JPEG
    let mut buf = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .map_err(|e| {
            tracing::error!("prepare_image: failed to encode JPEG: {}", e);
            format!("Failed to encode image: {}", e)
        })?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    tracing::debug!("prepare_image: encoded to {} bytes base64", b64.len());
    Ok(b64)
}

/// Extract raw content text from a llama-server chat completion response.
pub(crate) fn extract_content(json: &serde_json::Value) -> Option<String> {
    fn extract_text(value: &serde_json::Value) -> Option<String> {
        match value {
            serde_json::Value::String(text) => {
                let trimmed = text.trim();
                (!trimmed.is_empty()).then(|| trimmed.to_string())
            }
            serde_json::Value::Array(parts) => {
                let joined = parts
                    .iter()
                    .filter_map(|part| match part {
                        serde_json::Value::String(text) => Some(text.trim().to_string()),
                        serde_json::Value::Object(map) => map
                            .get("text")
                            .and_then(|value| value.as_str())
                            .map(|text| text.trim().to_string()),
                        _ => None,
                    })
                    .filter(|text| !text.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n");
                (!joined.is_empty()).then_some(joined)
            }
            _ => None,
        }
    }

    if let Some(content) = json
        .pointer("/choices/0/message/content")
        .and_then(extract_text)
    {
        return Some(content);
    }

    json.pointer("/choices/0/message/reasoning_content")
        .and_then(extract_text)
}

/// Parse the mood from the LLM response.
/// Strategy: exact match on cleaned text → last mood keyword found (= conclusion, not reasoning).
/// Parse dimensional mood response: "mood intensity" (e.g. "sadness 3").
/// Strategy: find the LAST occurrence of "mood intensity" pattern in the post-think text.
/// Fallback: if only mood found without intensity → Medium (2).
/// Extract first sentence from a description (for cascade context).
pub(crate) fn first_sentence(text: &str) -> &str {
    text.find(|c: char| c == '.' || c == '!' || c == '?')
        .map(|i| &text[..=i])
        .unwrap_or(text)
}

pub(crate) fn parse_mood_intensity_response(json: &serde_json::Value) -> Result<MoodTag, String> {
    let content = extract_content(json).ok_or_else(|| "No content in response".to_string())?;

    let text = content.trim().to_lowercase();
    let cleaned = if let Some(pos) = text.find("</think>") {
        text[pos + 8..].trim()
    } else {
        &text
    };

    // 1. Direct compact answer: "mood 2"
    let re = regex::Regex::new(
        r"(?i)\b(epic|tension|sadness|comedy|romance|horror|peaceful|mystery)\s+([123])\b",
    )
    .unwrap();

    let mut best_match: Option<(BaseMood, MoodIntensity, usize)> = None;
    for cap in re.captures_iter(cleaned) {
        let mood_str = cap.get(1).unwrap().as_str();
        let intensity_str = cap.get(2).unwrap().as_str();
        let pos = cap.get(0).unwrap().start();
        if let Some(mood) = BaseMood::from_str_opt(mood_str) {
            let intensity = MoodIntensity::from_u8(intensity_str.parse::<u8>().unwrap_or(2));
            // Take the LAST match (conclusion, not reasoning)
            if best_match.is_none() || pos > best_match.unwrap().2 {
                best_match = Some((mood, intensity, pos));
            }
        }
    }

    if let Some((mood, intensity, _)) = best_match {
        return Ok(MoodTag { mood, intensity });
    }

    // 2. Field-oriented formats:
    //    "mood: tension\nintensity: 2" or JSON-ish {"mood":"tension","intensity":2}
    let field_patterns = [
        r#"(?is)mood\s*[:=]\s*["`']?(epic|tension|sadness|comedy|romance|horror|peaceful|mystery)["`']?[^a-z0-9]{0,80}intensity\s*[:=]\s*["`']?([123])["`']?"#,
        r#"(?is)intensity\s*[:=]\s*["`']?([123])["`']?[^a-z0-9]{0,80}mood\s*[:=]\s*["`']?(epic|tension|sadness|comedy|romance|horror|peaceful|mystery)["`']?"#,
        r#"(?im)^\s*(?:mood|answer|final answer)\s*[:=-]?\s*(epic|tension|sadness|comedy|romance|horror|peaceful|mystery)(?:\s*[,;:-]\s*|\s+)([123])\s*$"#,
    ];
    for pattern in field_patterns {
        let re = regex::Regex::new(pattern).unwrap();
        if let Some(cap) = re.captures_iter(cleaned).last() {
            let (mood_str, intensity_str) =
                if pattern.contains("intensity") && pattern.starts_with("(?is)intensity") {
                    (cap.get(2).unwrap().as_str(), cap.get(1).unwrap().as_str())
                } else {
                    (cap.get(1).unwrap().as_str(), cap.get(2).unwrap().as_str())
                };
            if let Some(mood) = BaseMood::from_str_opt(mood_str) {
                return Ok(MoodTag {
                    mood,
                    intensity: MoodIntensity::from_u8(intensity_str.parse::<u8>().unwrap_or(2)),
                });
            }
        }
    }

    // 3. Fallback: find the last mood keyword, and reuse an explicit intensity field if present.
    let mut best: Option<(BaseMood, usize)> = None;
    for &mood in BaseMood::ALL.iter() {
        if let Some(pos) = cleaned.rfind(mood.as_str()) {
            if best.is_none() || pos > best.unwrap().1 {
                best = Some((mood, pos));
            }
        }
    }

    if let Some((mood, _)) = best {
        let intensity = regex::Regex::new(r#"(?i)intensity\s*[:=]\s*["`']?([123])"#)
            .unwrap()
            .captures_iter(cleaned)
            .last()
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse::<u8>().ok())
            .map(MoodIntensity::from_u8)
            .unwrap_or(MoodIntensity::Medium);
        return Ok(MoodTag { mood, intensity });
    }

    Err(format!(
        "No mood found in response: '{}'",
        &cleaned[..cleaned.len().min(80)]
    ))
}

/// Parse the structured extraction response from the VLM.
///
/// Expects format:
/// ```text
/// EMOTION: sadness
/// INTENSITY: 7
/// NARRATIVE: flashback
/// ATMOSPHERE: dark, nostalgic
/// CONTENT: Character imagines idol player doing a powerful kick
/// ```
pub(crate) fn parse_structured_response(content: &str) -> Result<PageFeatures, String> {
    // Strip </think> tags if present
    let text = if let Some(pos) = content.find("</think>") {
        &content[pos + 8..]
    } else {
        content
    };

    let emotion_re =
        regex::Regex::new(r"(?i)EMOTION:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let intensity_re =
        regex::Regex::new(r"(?i)INTENSITY:\s*(\d+)").map_err(|e| format!("Regex error: {}", e))?;
    let narrative_re =
        regex::Regex::new(r"(?i)NARRATIVE:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let atmosphere_re =
        regex::Regex::new(r"(?i)ATMOSPHERE:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let content_re =
        regex::Regex::new(r"(?i)CONTENT:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;

    let emotion = emotion_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_lowercase())
        .ok_or_else(|| {
            format!(
                "EMOTION field not found in: '{}'",
                &text[..text.len().min(200)]
            )
        })?;

    let intensity = intensity_re
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().trim().parse::<u8>().ok())
        .unwrap_or(5);

    let narrative = narrative_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| {
            let raw = m.as_str().trim().to_lowercase();
            // Normalize to canonical values
            if raw.contains("flashback") {
                "flashback".to_string()
            } else if raw.contains("dream") {
                "dream".to_string()
            } else if raw.contains("thought") {
                "thought".to_string()
            } else {
                "present".to_string()
            }
        })
        .unwrap_or_else(|| "present".to_string());

    let atmosphere = atmosphere_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "neutral".to_string());

    let content_val = content_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown content".to_string());

    Ok(PageFeatures {
        emotion,
        intensity: intensity.clamp(1, 10),
        narrative,
        atmosphere,
        content: content_val,
    })
}

/// Normalize VLM emotion output to 8 canonical manga emotions.
///
/// Maps synonyms to canonical values: joy, sadness, anger, fear,
/// determination, shock, nostalgia, neutral.
pub(crate) fn normalize_emotion(raw: &str) -> String {
    let lower = raw.trim().to_lowercase();
    match lower.as_str() {
        // Direct canonical matches
        "joy" | "sadness" | "anger" | "fear" | "determination" | "shock" | "nostalgia"
        | "neutral" => lower,
        // surprise → shock (fixes surprise-spam problem)
        "surprise" | "surprised" | "stunned" | "astonished" | "amazed" | "disbelief" => {
            "shock".to_string()
        }
        // excitement/triumph/happiness → joy
        "excitement" | "excited" | "triumph" | "triumphant" | "happiness" | "happy" | "delight"
        | "elation" | "pride" | "proud" | "cheerful" => "joy".to_string(),
        // resolve/willpower/ambition → determination
        "resolve" | "resolved" | "willpower" | "ambition" | "ambitious" | "focused"
        | "motivated" | "driven" | "confident" | "confidence" | "intensity" | "intense" => {
            "determination".to_string()
        }
        // longing/wistful/bittersweet → nostalgia
        "longing" | "wistful" | "bittersweet" | "reminiscence" | "reminiscing" | "yearning"
        | "melancholy" | "regret" => "nostalgia".to_string(),
        // sorrow → sadness
        "sorrow" | "grief" | "mourning" | "despair" | "heartbreak" | "pain" | "anguish" => {
            "sadness".to_string()
        }
        // fear synonyms
        "terror" | "dread" | "horror" | "panic" | "scared" | "frightened" | "anxious"
        | "anxiety" => "fear".to_string(),
        // anger synonyms
        "rage" | "fury" | "furious" | "frustrated" | "frustration" | "aggression"
        | "aggressive" | "hostile" => "anger".to_string(),
        _ => "neutral".to_string(),
    }
}

/// Fallback mood from canonical emotion when MOOD field is missing or unparseable.
pub(crate) fn emotion_to_mood_fallback(emotion: &str) -> MoodCategory {
    match emotion {
        "joy" => MoodCategory::Comedy,
        "sadness" => MoodCategory::Sadness,
        "anger" => MoodCategory::Tension,
        "fear" => MoodCategory::Horror,
        "determination" => MoodCategory::Epic,
        "shock" => MoodCategory::Tension,
        "nostalgia" => MoodCategory::Sadness,
        "neutral" => MoodCategory::Peaceful,
        _ => MoodCategory::Peaceful,
    }
}

/// Parse a hybrid VLM response (MOOD + 5 feature fields) into a `HybridResult`.
///
/// Expects format:
/// ```text
/// MOOD: sadness
/// EMOTION: nostalgia
/// INTENSITY: 7
/// NARRATIVE: flashback
/// ATMOSPHERE: dark, nostalgic
/// CONTENT: Character recalls past events
/// ```
pub(crate) fn parse_hybrid_response(content: &str) -> Result<HybridResult, String> {
    // Strip </think> tags if present
    let text = if let Some(pos) = content.find("</think>") {
        &content[pos + 8..]
    } else {
        content
    };

    // Parse MOOD field
    let mood_re =
        regex::Regex::new(r"(?i)MOOD:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;

    // Parse features using same regexes as parse_structured_response
    let emotion_re =
        regex::Regex::new(r"(?i)EMOTION:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let intensity_re =
        regex::Regex::new(r"(?i)INTENSITY:\s*(\d+)").map_err(|e| format!("Regex error: {}", e))?;
    let narrative_re =
        regex::Regex::new(r"(?i)NARRATIVE:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let atmosphere_re =
        regex::Regex::new(r"(?i)ATMOSPHERE:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;
    let content_re =
        regex::Regex::new(r"(?i)CONTENT:\s*(.+)").map_err(|e| format!("Regex error: {}", e))?;

    // ── EMOTION (needed for fallback) ──
    let raw_emotion = emotion_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_lowercase())
        .unwrap_or_else(|| "neutral".to_string());
    let emotion = normalize_emotion(&raw_emotion);

    // ── MOOD ──
    let mood = if let Some(cap) = mood_re.captures(text) {
        let raw_mood = cap.get(1).unwrap().as_str().trim().to_lowercase();
        // 1. Try direct parse
        if let Some(m) = MoodCategory::from_str_opt(&raw_mood) {
            m
        } else {
            // 2. Last-keyword scan (same strategy as parse_mood_response)
            let mut best: Option<(MoodCategory, usize)> = None;
            for &mood_cat in MoodCategory::ALL.iter() {
                if let Some(pos) = raw_mood.rfind(mood_cat.as_str()) {
                    if best.is_none() || pos > best.unwrap().1 {
                        best = Some((mood_cat, pos));
                    }
                }
            }
            if let Some((m, _)) = best {
                m
            } else {
                // 3. Fallback to emotion
                emotion_to_mood_fallback(&emotion)
            }
        }
    } else {
        // No MOOD field at all → fallback to emotion
        emotion_to_mood_fallback(&emotion)
    };

    // ── INTENSITY ──
    let intensity = intensity_re
        .captures(text)
        .and_then(|c| c.get(1))
        .and_then(|m| m.as_str().trim().parse::<u8>().ok())
        .unwrap_or(5)
        .clamp(1, 10);

    // ── NARRATIVE ──
    let narrative = narrative_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| {
            let raw = m.as_str().trim().to_lowercase();
            if raw.contains("flashback") {
                "flashback".to_string()
            } else if raw.contains("dream") {
                "dream".to_string()
            } else if raw.contains("thought") {
                "thought".to_string()
            } else {
                "present".to_string()
            }
        })
        .unwrap_or_else(|| "present".to_string());

    // ── ATMOSPHERE ──
    let atmosphere = atmosphere_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "neutral".to_string());

    // ── CONTENT ──
    let content_val = content_re
        .captures(text)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_else(|| "Unknown content".to_string());

    Ok(HybridResult {
        mood,
        features: PageFeatures {
            emotion,
            intensity,
            narrative,
            atmosphere,
            content: content_val,
        },
    })
}

/// Default mood categories with descriptions (8 base moods).
pub fn default_mood_categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("epic", "Heroic moments, battles, triumph, power display"),
        ("tension", "Conflict, confrontation, suspense, buildup"),
        ("sadness", "Grief, loss, tears, regret, melancholy"),
        ("comedy", "Humor, funny situations, lighthearted"),
        ("romance", "Love, affection, tender connection"),
        ("horror", "Fear, dread, disturbing atmosphere"),
        ("peaceful", "Calm daily life, relaxation, serene"),
        ("mystery", "Unknown, intrigue, investigation"),
    ]
}

/// Find a free TCP port.
fn find_free_port() -> Result<u16, std::io::Error> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
