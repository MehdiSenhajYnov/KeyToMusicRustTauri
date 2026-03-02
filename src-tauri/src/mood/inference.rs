use std::process::Stdio;
use tokio::process::{Child, Command};

use crate::types::MoodCategory;

use super::llama_manager;

/// A running llama-server instance.
pub struct LlamaServer {
    process: Child,
    pub port: u16,
}

impl LlamaServer {
    /// Start llama-server with the given model and mmproj paths.
    pub async fn start(model_path: &str, mmproj_path: &str) -> Result<Self, String> {
        let server_path = llama_manager::find_llama_server()
            .ok_or_else(|| "llama-server not installed".to_string())?;

        // Find a free port
        let port = find_free_port().map_err(|e| format!("Failed to find free port: {}", e))?;

        // Log llama-server stderr to a file for debugging
        let log_dir = crate::storage::config::get_app_data_dir().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let stderr_file = std::fs::File::create(log_dir.join("llama-server.log"))
            .map_err(|e| format!("Failed to create llama-server log: {}", e))?;

        tracing::info!(
            "Starting llama-server: {:?} -m {} --mmproj {} --port {} -c 2048 -ngl 99",
            server_path,
            model_path,
            mmproj_path,
            port
        );

        let mut cmd = Command::new(&server_path);
        cmd.args([
            "-m",
            model_path,
            "--mmproj",
            mmproj_path,
            "--port",
            &port.to_string(),
            "-c",
            "2048",
            "--flash-attn", "auto",
            "--cache-type-k",
            "q8_0",
            "--cache-type-v",
            "q8_0",
            "-ngl",
            "99",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(stderr_file));

        #[cfg(target_os = "windows")]
        {
            // CREATE_NO_WINDOW (0x08000000) | BELOW_NORMAL_PRIORITY_CLASS (0x00004000)
            cmd.creation_flags(0x08004000);
        }

        let process = cmd
            .spawn()
            .map_err(|e| format!("Failed to start llama-server: {}", e))?;

        let mut server = Self { process, port };

        // Wait for server to become ready (VLM model loading can take a while)
        server.wait_for_ready(120).await?;

        tracing::info!("llama-server started on port {}", port);
        Ok(server)
    }

    /// Poll GET /health until the server responds 200.
    async fn wait_for_ready(&mut self, timeout_secs: u64) -> Result<(), String> {
        let url = format!("http://127.0.0.1:{}/health", self.port);
        let client = reqwest::Client::new();
        let deadline =
            tokio::time::Instant::now() + tokio::time::Duration::from_secs(timeout_secs);

        tracing::info!("Waiting for llama-server to be ready (timeout {}s)...", timeout_secs);

        loop {
            // Check if process crashed
            if let Ok(Some(status)) = self.process.try_wait() {
                let log_path = crate::storage::config::get_app_data_dir()
                    .join("logs")
                    .join("llama-server.log");
                let stderr = std::fs::read_to_string(&log_path).unwrap_or_default();
                let last_lines: String = stderr.lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
                tracing::error!("llama-server process exited with {}\nLast log lines:\n{}", status, last_lines);
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
                    tracing::debug!("llama-server health check returned {}, retrying...", resp.status());
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Err(e) => {
                    tracing::debug!("llama-server health check failed: {}, retrying...", e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Analyze a manga page image and detect the mood.
    pub async fn analyze_mood(&self, image_base64: &str) -> Result<MoodCategory, String> {
        tracing::info!("analyze_mood: sending image ({} bytes b64) to llama-server port {}", image_base64.len(), self.port);

        let url = format!(
            "http://127.0.0.1:{}/v1/chat/completions",
            self.port
        );

        let body = serde_json::json!({
            "model": "qwen3-vl-2b",
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
                            "text": "Analyze this manga page step by step:\n1. What are the characters expressing? (faces, posture, gestures)\n2. What feeling does the author want the reader to experience?\n3. Classify as ONE of the categories below.\n\nKey distinctions:\n- sadness vs emotional_climax: sorrow/regret/nostalgia = sadness, triumph/determination = emotional_climax\n- tension vs epic_battle: anxious anticipation = tension, active combat = epic_battle\n- chase_action: ONLY for active pursuit/escape, not flashbacks with movement\n\nCategories: epic_battle, tension, sadness, comedy, romance, horror, peaceful (calm daily life), emotional_climax, mystery, chase_action\n\nReply with ONLY the category name after your reasoning."
                        }
                    ]
                }
            ],
            "max_tokens": 2048,
            "temperature": 0.0
        });

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("HTTP client error: {}", e))?;
        tracing::debug!("analyze_mood: POST {}", url);
        let response: reqwest::Response = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("analyze_mood: failed to send request to llama-server: {}", e);
                format!("Failed to send to llama-server: {}", e)
            })?;

        tracing::debug!("analyze_mood: llama-server responded with HTTP {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let text: String = response.text().await.unwrap_or_default();
            tracing::error!("analyze_mood: llama-server error HTTP {}: {}", status, text);
            return Err(format!(
                "llama-server returned HTTP {}: {}",
                status, text
            ));
        }

        let json: serde_json::Value = response
            .json::<serde_json::Value>()
            .await
            .map_err(|e| {
                tracing::error!("analyze_mood: failed to parse JSON response: {}", e);
                format!("Failed to parse response: {}", e)
            })?;

        tracing::debug!("analyze_mood: raw response: {}", serde_json::to_string(&json).unwrap_or_default());

        let result = parse_mood_response(&json);
        match &result {
            Ok(mood) => tracing::info!("analyze_mood: detected {:?}", mood),
            Err(e) => tracing::error!("analyze_mood: failed to parse mood from response: {}", e),
        }
        result
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

impl Drop for LlamaServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Prepare an image for analysis: read file, resize to 672px max dimension, encode as base64 JPEG.
pub fn prepare_image(image_data: &[u8]) -> Result<String, String> {
    use base64::Engine;
    use image::GenericImageView;

    tracing::debug!("prepare_image: loading {} bytes", image_data.len());

    let img = image::load_from_memory(image_data)
        .map_err(|e| {
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
        tracing::info!("prepare_image: image {}x{} already within 672px, no resize needed", w, h);
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

/// Parse the mood from the LLM response.
fn parse_mood_response(json: &serde_json::Value) -> Result<MoodCategory, String> {
    let content = json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "No content in response".to_string())?;

    let text = content.trim().to_lowercase();
    // Remove any thinking tags if present
    let cleaned = if let Some(pos) = text.find("</think>") {
        text[pos + 8..].trim()
    } else {
        &text
    };

    match cleaned {
        "epic_battle" => Ok(MoodCategory::EpicBattle),
        "tension" => Ok(MoodCategory::Tension),
        "sadness" => Ok(MoodCategory::Sadness),
        "comedy" => Ok(MoodCategory::Comedy),
        "romance" => Ok(MoodCategory::Romance),
        "horror" => Ok(MoodCategory::Horror),
        "peaceful" => Ok(MoodCategory::Peaceful),
        "emotional_climax" => Ok(MoodCategory::EmotionalClimax),
        "mystery" => Ok(MoodCategory::Mystery),
        "chase_action" => Ok(MoodCategory::ChaseAction),
        other => Err(format!("Unknown mood: '{}'", other)),
    }
}

/// Find a free TCP port.
fn find_free_port() -> Result<u16, std::io::Error> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
