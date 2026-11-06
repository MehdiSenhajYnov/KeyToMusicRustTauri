//! Client for any OpenAI-compatible chat API (OpenAI, OpenRouter, Ollama,
//! LM Studio, llama-server…): images in, JSON out.

use std::time::Duration;

use serde_json::{json, Value};

use crate::types::MoodImageDetail;

/// Ready-made endpoints offered by the UIs: (name, base URL, needs a key).
pub const PRESETS: &[(&str, &str, bool)] = &[
    ("OpenAI", "https://api.openai.com/v1", true),
    ("OpenRouter", "https://openrouter.ai/api/v1", true),
    ("Ollama (local)", "http://localhost:11434/v1", false),
    ("LM Studio (local)", "http://localhost:1234/v1", false),
];

/// One chapter batch can take a while on reasoning models.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

/// Everything needed to talk to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderSettings {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub image_detail: MoodImageDetail,
}

impl ProviderSettings {
    /// A key is required unless the endpoint is on this machine.
    pub fn is_local(&self) -> bool {
        let host = self
            .base_url
            .split("://")
            .nth(1)
            .unwrap_or_default()
            .split(['/', ':'])
            .next()
            .unwrap_or_default();
        matches!(host, "localhost" | "127.0.0.1" | "[::1]" | "0.0.0.0")
    }

    pub fn is_configured(&self) -> bool {
        !self.base_url.is_empty()
            && !self.model.is_empty()
            && (self.is_local() || self.api_key.as_deref().is_some_and(|k| !k.is_empty()))
    }

    fn is_openai(&self) -> bool {
        self.base_url.contains("api.openai.com")
    }
}

/// A part of the user message, in order.
#[derive(Clone, Debug)]
pub enum Part {
    Text(String),
    /// Base64 JPEG.
    Image(std::sync::Arc<String>),
}

#[derive(Debug, Clone)]
pub struct ProviderError {
    pub message: String,
    /// Retrying the same request later may work (network, rate limit, 5xx).
    pub retryable: bool,
    /// The settings are wrong (key, model, URL): retrying is pointless.
    pub settings: bool,
}

impl ProviderError {
    fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
            settings: false,
        }
    }

    fn settings(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            settings: true,
        }
    }

    fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            settings: false,
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn client() -> Result<reqwest::Client, ProviderError> {
    reqwest::Client::builder()
        .user_agent(concat!("KeyToMusic/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| ProviderError::fatal(format!("Client HTTP indisponible : {e}")))
}

fn authorized(
    request: reqwest::RequestBuilder,
    settings: &ProviderSettings,
) -> reqwest::RequestBuilder {
    match settings.api_key.as_deref().filter(|k| !k.is_empty()) {
        Some(key) => request.bearer_auth(key),
        None => request,
    }
}

/// Models whose names say they reason (they accept `reasoning_effort`).
fn reasons(model: &str) -> bool {
    let name = model
        .rsplit('/')
        .next()
        .unwrap_or(model)
        .to_ascii_lowercase();
    name.starts_with("gpt-5")
        || name.starts_with("o1")
        || name.starts_with("o3")
        || name.starts_with("o4")
}

/// Build the chat request. Optional parameters may be dropped on retry when
/// an endpoint rejects them.
fn request_body(
    settings: &ProviderSettings,
    system: &str,
    parts: &[Part],
    schema_name: &str,
    schema: &Value,
) -> Value {
    let detail = settings.image_detail.as_str();
    let content: Vec<Value> = parts
        .iter()
        .map(|part| match part {
            Part::Text(text) => json!({ "type": "text", "text": text }),
            Part::Image(b64) => json!({
                "type": "image_url",
                "image_url": { "url": format!("data:image/jpeg;base64,{b64}"), "detail": detail }
            }),
        })
        .collect();
    let mut body = json!({
        "model": settings.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": content }
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": { "name": schema_name, "strict": true, "schema": schema }
        }
    });
    if reasons(&settings.model) {
        body["reasoning_effort"] = json!("low");
    }
    body
}

/// The optional parameter an error names, if we may drop it and retry.
fn rejected_parameter(error: &Value, body: &Value) -> Option<&'static str> {
    let err = error.get("error").unwrap_or(error);
    let param = err
        .get("param")
        .and_then(|p| p.as_str())
        .unwrap_or_default();
    let message = err
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    ["reasoning_effort", "response_format"]
        .into_iter()
        .filter(|name| body.get(*name).is_some())
        .find(|name| {
            param.starts_with(name)
                || message.contains(name)
                || (*name == "response_format" && message.contains("json_schema"))
        })
}

fn error_message(status: reqwest::StatusCode, body: &Value, raw: &str) -> String {
    let err = body.get("error").unwrap_or(body);
    let detail = err
        .get("message")
        .and_then(|m| m.as_str())
        .map(str::to_string)
        .or_else(|| err.as_str().map(str::to_string))
        .unwrap_or_else(|| raw.chars().take(300).collect());
    format!("HTTP {} : {}", status.as_u16(), detail.trim())
}

fn classify(status: reqwest::StatusCode, body: &Value, raw: &str) -> ProviderError {
    let message = error_message(status, body, raw);
    let code = body
        .pointer("/error/code")
        .and_then(|c| c.as_str())
        .unwrap_or_default();
    match status.as_u16() {
        401 | 403 => ProviderError::settings(format!("Clé API refusée ({message})")),
        404 => ProviderError::settings(format!("Modèle ou adresse introuvable ({message})")),
        429 if code == "insufficient_quota" => {
            ProviderError::settings(format!("Crédit épuisé sur ce compte ({message})"))
        }
        408 | 409 | 429 | 500..=599 => ProviderError::retryable(message),
        400 if message.to_ascii_lowercase().contains("image") => ProviderError::settings(format!(
            "Ce modèle ne lit pas les images, choisis un modèle de vision ({message})"
        )),
        _ => ProviderError::fatal(message),
    }
}

/// The first JSON object in a model answer (tolerates ``` fences and prose).
pub fn extract_json(text: &str) -> Option<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(text.trim()) {
        return Some(value);
    }
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start)
        .then(|| serde_json::from_str(&text[start..=end]).ok())
        .flatten()
}

/// Send images and instructions, get back the JSON object the schema describes.
pub async fn chat_json(
    settings: &ProviderSettings,
    system: &str,
    parts: &[Part],
    schema_name: &str,
    schema: &Value,
) -> Result<Value, ProviderError> {
    let client = client()?;
    let url = format!("{}/chat/completions", settings.base_url);
    let mut body = request_body(settings, system, parts, schema_name, schema);

    // Up to three downgrades: effort, schema → JSON mode, JSON mode.
    for _ in 0..4 {
        let response = authorized(client.post(&url), settings)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::retryable("Le modèle a mis trop de temps à répondre")
                } else if e.is_connect() {
                    ProviderError::retryable(format!(
                        "Impossible de joindre {} ({e})",
                        settings.base_url
                    ))
                } else {
                    ProviderError::retryable(format!("Requête impossible : {e}"))
                }
            })?;
        let status = response.status();
        let raw = response
            .text()
            .await
            .map_err(|e| ProviderError::retryable(format!("Réponse incomplète : {e}")))?;
        let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);

        if !status.is_success() {
            if status.as_u16() == 400 || status.as_u16() == 422 {
                if let Some(param) = rejected_parameter(&parsed, &body) {
                    tracing::info!("Mood AI: endpoint rejected `{param}`, retrying without it");
                    let schema_mode =
                        body.pointer("/response_format/type") == Some(&json!("json_schema"));
                    if param == "response_format" && schema_mode {
                        // Plain JSON mode; the prompt already describes the shape.
                        body["response_format"] = json!({ "type": "json_object" });
                    } else if let Some(fields) = body.as_object_mut() {
                        fields.remove(param);
                    }
                    continue;
                }
            }
            return Err(classify(status, &parsed, &raw));
        }

        let message = parsed.pointer("/choices/0/message").ok_or_else(|| {
            ProviderError::retryable(format!(
                "Réponse inattendue du modèle : {}",
                raw.chars().take(200).collect::<String>()
            ))
        })?;
        if let Some(refusal) = message.get("refusal").and_then(|r| r.as_str()) {
            return Err(ProviderError::fatal(format!(
                "Le modèle a refusé : {refusal}"
            )));
        }
        let text = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or_default();
        return extract_json(text).ok_or_else(|| {
            ProviderError::retryable(format!(
                "Le modèle n'a pas répondu en JSON : {}",
                text.chars().take(200).collect::<String>()
            ))
        });
    }
    Err(ProviderError::settings(
        "Ce point d'accès refuse les réponses structurées",
    ))
}

/// Models the endpoint offers, vision-capable chat models first when the
/// endpoint is OpenAI (its list also holds audio, embedding and image models).
pub async fn list_models(settings: &ProviderSettings) -> Result<Vec<String>, ProviderError> {
    let client = client()?;
    let url = format!("{}/models", settings.base_url);
    let response = authorized(client.get(&url), settings)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| {
            ProviderError::retryable(format!("Impossible de joindre {} ({e})", settings.base_url))
        })?;
    let status = response.status();
    let raw = response.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if !status.is_success() {
        return Err(classify(status, &parsed, &raw));
    }
    let mut models: Vec<String> = parsed
        .get("data")
        .or_else(|| parsed.get("models"))
        .and_then(|d| d.as_array())
        .map(|list| {
            list.iter()
                .filter_map(|m| {
                    m.get("id")
                        .or_else(|| m.get("name"))
                        .and_then(|id| id.as_str())
                        .map(str::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    if settings.is_openai() {
        models.retain(|id| openai_chat_model(id));
    }
    models.sort();
    models.dedup();
    Ok(models)
}

/// OpenAI's own vision chat models, without audio/realtime/image/embedding ones.
fn openai_chat_model(id: &str) -> bool {
    let chat = id.starts_with("gpt-") || id.starts_with("o3") || id.starts_with("o4");
    let other = [
        "audio",
        "realtime",
        "tts",
        "transcribe",
        "image",
        "search",
        "embedding",
        "instruct",
        "codex",
        "3.5",
    ];
    chat && !other.iter().any(|word| id.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(url: &str) -> ProviderSettings {
        ProviderSettings {
            base_url: url.into(),
            model: "gpt-5-mini".into(),
            api_key: None,
            image_detail: MoodImageDetail::High,
        }
    }

    #[test]
    fn local_endpoints_need_no_key() {
        assert!(settings("http://localhost:11434/v1").is_configured());
        assert!(settings("http://127.0.0.1:1234/v1").is_configured());
        assert!(!settings("https://api.openai.com/v1").is_configured());
        let mut keyed = settings("https://api.openai.com/v1");
        keyed.api_key = Some("sk-test".into());
        assert!(keyed.is_configured());
    }

    #[test]
    fn json_is_found_inside_prose_and_fences() {
        let text = "Here you go:\n```json\n{\"scenes\": []}\n```";
        assert_eq!(extract_json(text), Some(json!({ "scenes": [] })));
        assert_eq!(extract_json("nothing"), None);
    }

    #[test]
    fn rejected_parameters_are_recognized() {
        let body = json!({ "reasoning_effort": "low", "response_format": {} });
        let error = json!({ "error": {
            "message": "Unsupported parameter: 'reasoning_effort' is not supported with this model.",
            "param": "reasoning_effort"
        }});
        assert_eq!(rejected_parameter(&error, &body), Some("reasoning_effort"));
        let error = json!({ "error": { "message": "Invalid parameter: 'response_format' of type 'json_schema' is not supported with this model." }});
        assert_eq!(rejected_parameter(&error, &body), Some("response_format"));
        let error = json!({ "error": { "message": "You exceeded your quota" }});
        assert_eq!(rejected_parameter(&error, &body), None);
    }

    #[test]
    fn only_reasoning_models_get_an_effort() {
        assert!(reasons("gpt-5-mini"));
        assert!(reasons("openai/gpt-5"));
        assert!(reasons("o4-mini"));
        assert!(!reasons("gpt-4.1-mini"));
        assert!(!reasons("qwen2.5vl:7b"));
    }

    #[test]
    fn openai_list_keeps_chat_models() {
        assert!(openai_chat_model("gpt-5-mini"));
        assert!(openai_chat_model("gpt-4.1"));
        assert!(!openai_chat_model("gpt-4o-mini-transcribe"));
        assert!(!openai_chat_model("text-embedding-3-small"));
        assert!(!openai_chat_model("gpt-image-1"));
    }
}
