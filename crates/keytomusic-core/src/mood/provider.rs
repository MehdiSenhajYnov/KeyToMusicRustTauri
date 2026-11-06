//! Model client, images in, JSON out. Two routes:
//! - a ChatGPT plan (Sign in with ChatGPT): OpenAI's Responses API, streamed;
//! - an API key on any OpenAI-compatible chat API (OpenAI, OpenRouter,
//!   Ollama, LM Studio, llama-server…).

use std::time::Duration;

use serde_json::{json, Value};

use super::chatgpt;
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

/// How requests are paid for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMode {
    /// The user's ChatGPT plan; `plan`: signed in with plan usage allowed.
    ChatGpt { plan: bool },
    /// An API key (or none for a local server) on `base_url`.
    ApiKey,
}

/// Everything needed to talk to the model.
#[derive(Clone, Debug, PartialEq)]
pub struct ProviderSettings {
    pub auth: AuthMode,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub image_detail: MoodImageDetail,
}

/// A model the account can use: `id` goes in requests, `name` in the UI.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
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
        match self.auth {
            AuthMode::ChatGpt { plan } => plan && !self.model.is_empty(),
            AuthMode::ApiKey => {
                !self.base_url.is_empty()
                    && !self.model.is_empty()
                    && (self.is_local() || self.api_key.as_deref().is_some_and(|k| !k.is_empty()))
            }
        }
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
    /// Wait this long before trying again (usage limit reached).
    pub pause: Option<Duration>,
}

impl ProviderError {
    pub fn retryable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: true,
            settings: false,
            pause: None,
        }
    }

    pub fn settings(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            settings: true,
            pause: None,
        }
    }

    pub fn fatal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            retryable: false,
            settings: false,
            pause: None,
        }
    }
}

impl From<chatgpt::AuthError> for ProviderError {
    fn from(error: chatgpt::AuthError) -> Self {
        if error.sign_in_needed {
            ProviderError::settings(error.message)
        } else {
            ProviderError::retryable(error.message)
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
        || name.starts_with("gpt-6")
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
    if let AuthMode::ChatGpt { .. } = settings.auth {
        return responses_json(settings, system, parts, schema_name, schema).await;
    }
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
pub async fn list_models(settings: &ProviderSettings) -> Result<Vec<ModelInfo>, ProviderError> {
    if let AuthMode::ChatGpt { .. } = settings.auth {
        return plan_models().await;
    }
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
    Ok(models
        .into_iter()
        .map(|id| ModelInfo {
            name: id.clone(),
            id,
        })
        .collect())
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

// ── ChatGPT plan: Responses API ──

/// Usage limit reached: wait before asking again.
const LIMIT_PAUSE: Duration = Duration::from_secs(15 * 60);

fn plan_request_body(
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
            Part::Text(text) => json!({ "type": "input_text", "text": text }),
            Part::Image(b64) => json!({
                "type": "input_image",
                "image_url": format!("data:image/jpeg;base64,{b64}"),
                "detail": detail
            }),
        })
        .collect();
    json!({
        "model": settings.model,
        "instructions": system,
        "input": [{ "role": "user", "content": content }],
        "text": { "format": {
            "type": "json_schema", "name": schema_name, "strict": true, "schema": schema
        }},
        "reasoning": { "effort": "low" },
        "store": false,
        "stream": true
    })
}

/// Drop or relax the optional field an error names; false if none applies.
fn relax(body: &mut Value, param: &str) -> bool {
    let Some(fields) = body.as_object_mut() else {
        return false;
    };
    if param.starts_with("reasoning") && fields.remove("reasoning").is_some() {
        return true;
    }
    if param.starts_with("text") || param.contains("format") || param.contains("json_schema") {
        let schema_mode = fields
            .get("text")
            .and_then(|t| t.pointer("/format/type"))
            .and_then(|t| t.as_str())
            == Some("json_schema");
        if schema_mode {
            fields.insert(
                "text".into(),
                json!({ "format": { "type": "json_object" } }),
            );
            return true;
        }
        return fields.remove("text").is_some();
    }
    false
}

/// What a plan-usage error means for the app.
fn plan_error(status: u16, code: &str, message: &str) -> ProviderError {
    let detail = if message.is_empty() {
        format!("HTTP {status}")
    } else {
        format!("HTTP {status} : {message}")
    };
    match code {
        "subscription_sharing_usage_limit_exceeded" => ProviderError {
            message: format!(
                "Limite d'utilisation de ton forfait ChatGPT atteinte (ou celle fixée pour KeyToMusic). Gère-la sur {}",
                chatgpt::USAGE_URL
            ),
            retryable: true,
            settings: false,
            pause: Some(LIMIT_PAUSE),
        },
        "subscription_sharing_user_not_eligible" => ProviderError::settings(
            "Ton compte ChatGPT ne permet pas d'utiliser ton forfait dans une app (forfait Plus ou Pro requis)",
        ),
        "subscription_sharing_invalid_user"
        | "chatpass_v2_scope_not_authorized"
        | "chatpass_v2_invalid_authorization_context" => {
            ProviderError::settings(format!("Autorisation ChatGPT refusée, reconnecte-toi ({detail})"))
        }
        "subscription_sharing_usage_unavailable" | "subscription_sharing_user_unavailable" => {
            ProviderError::retryable(format!("ChatGPT momentanément indisponible ({detail})"))
        }
        "model_not_found" => ProviderError::settings(format!(
            "Ce modèle n'est pas disponible pour ton compte ChatGPT ({detail})"
        )),
        _ => match status {
            401 => ProviderError::settings(format!("Session ChatGPT refusée, reconnecte-toi ({detail})")),
            403 => ProviderError::settings(format!("Accès refusé par OpenAI ({detail})")),
            408 | 409 | 429 | 500..=599 => ProviderError::retryable(detail),
            _ => ProviderError::fatal(detail),
        },
    }
}

/// An error body: `{error: {code, message, param}}` or `{detail: "..."}`.
fn error_parts(body: &Value, raw: &str) -> (String, String, String) {
    let error = body.get("error").unwrap_or(body);
    let text = |v: Option<&Value>| v.and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let message = match text(error.get("message")) {
        m if !m.is_empty() => m,
        _ => match text(body.get("detail")) {
            d if !d.is_empty() => d,
            _ => raw.chars().take(300).collect(),
        },
    };
    (text(error.get("code")), message, text(error.get("param")))
}

/// Outcome of a streamed response.
#[derive(Debug, PartialEq)]
enum Streamed {
    Text(String),
    /// `response.failed`, `error`, or an interrupted stream: (code, message, param).
    Failed(String, String, String),
}

/// Read the server-sent events of a streamed Responses request.
fn parse_stream(raw: &str) -> Streamed {
    let mut deltas = String::new();
    let mut done_text: Option<String> = None;
    for line in raw.lines() {
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else {
            continue;
        };
        let text = |v: Option<&Value>| v.and_then(|v| v.as_str()).unwrap_or_default().to_string();
        match event
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or_default()
        {
            "response.output_text.delta" => deltas.push_str(&text(event.get("delta"))),
            "response.output_text.done" => done_text = Some(text(event.get("text"))),
            "response.completed" => {
                let from_output = event
                    .pointer("/response/output")
                    .and_then(|o| o.as_array())
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|item| item.get("content").and_then(|c| c.as_array()))
                            .flatten()
                            .filter(|c| {
                                c.get("type").and_then(|t| t.as_str()) == Some("output_text")
                            })
                            .filter_map(|c| c.get("text").and_then(|t| t.as_str()))
                            .collect::<String>()
                    })
                    .filter(|t| !t.is_empty());
                return Streamed::Text(
                    done_text
                        .or(from_output)
                        .unwrap_or_else(|| std::mem::take(&mut deltas)),
                );
            }
            "response.failed" => {
                let error = event
                    .pointer("/response/error")
                    .cloned()
                    .unwrap_or_default();
                return Streamed::Failed(
                    text(error.get("code")),
                    text(error.get("message")),
                    text(error.get("param")),
                );
            }
            "response.incomplete" => {
                let reason = text(event.pointer("/response/incomplete_details/reason"));
                return Streamed::Failed(
                    "incomplete".into(),
                    format!("Réponse incomplète ({reason})"),
                    String::new(),
                );
            }
            "error" => {
                let error = event.get("error").unwrap_or(&event);
                return Streamed::Failed(
                    text(error.get("code")),
                    text(error.get("message")),
                    text(error.get("param")),
                );
            }
            _ => {}
        }
    }
    Streamed::Failed(
        "interrupted".into(),
        "Le flux de réponse s'est arrêté avant la fin".into(),
        String::new(),
    )
}

async fn responses_json(
    settings: &ProviderSettings,
    system: &str,
    parts: &[Part],
    schema_name: &str,
    schema: &Value,
) -> Result<Value, ProviderError> {
    let auth = chatgpt::global();
    let mut token = auth.access_token().await?;
    let client = client()?;
    let url = format!("{}/responses", auth.api_base());
    let mut body = plan_request_body(settings, system, parts, schema_name, schema);
    let mut refreshed = false;

    for _ in 0..5 {
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ProviderError::retryable("Le modèle a mis trop de temps à répondre")
                } else {
                    ProviderError::retryable(format!("Impossible de joindre OpenAI ({e})"))
                }
            })?;
        let status = response.status().as_u16();
        let raw = response
            .text()
            .await
            .map_err(|e| ProviderError::retryable(format!("Réponse incomplète : {e}")))?;

        let (code, message, param) = if (200..300).contains(&status) {
            match parse_stream(&raw) {
                Streamed::Text(text) => {
                    return extract_json(&text).ok_or_else(|| {
                        ProviderError::retryable(format!(
                            "Le modèle n'a pas répondu en JSON : {}",
                            text.chars().take(200).collect::<String>()
                        ))
                    })
                }
                Streamed::Failed(code, message, param) => (code, message, param),
            }
        } else {
            error_parts(&serde_json::from_str(&raw).unwrap_or(Value::Null), &raw)
        };

        if status == 401 && !refreshed {
            token = auth.refreshed_token().await?;
            refreshed = true;
            continue;
        }
        let unsupported = code == "subscription_sharing_unsupported_capability"
            || code == "unsupported_parameter"
            || status == 400;
        if unsupported && relax(&mut body, &param) {
            tracing::info!("Mood AI: `{param}` refused on the ChatGPT plan, retrying without it");
            continue;
        }
        if code == "interrupted" {
            return Err(ProviderError::retryable(message));
        }
        return Err(plan_error(status, &code, &message));
    }
    Err(ProviderError::fatal(
        "OpenAI refuse cette requête même simplifiée",
    ))
}

/// Models the signed-in ChatGPT account may use, in OpenAI's order.
async fn plan_models() -> Result<Vec<ModelInfo>, ProviderError> {
    let auth = chatgpt::global();
    let token = auth.access_token().await?;
    let response = client()?
        .get(format!("{}/models", auth.api_base()))
        .bearer_auth(&token)
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| ProviderError::retryable(format!("Impossible de joindre OpenAI ({e})")))?;
    let status = response.status().as_u16();
    let raw = response.text().await.unwrap_or_default();
    let parsed: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if !(200..300).contains(&status) {
        let (code, message, _) = error_parts(&parsed, &raw);
        return Err(plan_error(status, &code, &message));
    }
    Ok(parse_plan_models(&parsed))
}

fn parse_plan_models(body: &Value) -> Vec<ModelInfo> {
    body.get("models")
        .and_then(|m| m.as_array())
        .map(|models| {
            models
                .iter()
                .filter(|m| {
                    m.get("visibility")
                        .and_then(|v| v.as_str())
                        .unwrap_or("list")
                        == "list"
                })
                .filter_map(|m| {
                    let id = m.get("slug").or_else(|| m.get("id"))?.as_str()?.to_string();
                    let name = m
                        .get("display_name")
                        .and_then(|n| n.as_str())
                        .unwrap_or(&id)
                        .to_string();
                    Some(ModelInfo { id, name })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(url: &str) -> ProviderSettings {
        ProviderSettings {
            auth: AuthMode::ApiKey,
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

    #[test]
    fn the_plan_needs_a_signed_in_account() {
        let mut plan = settings("https://api.openai.com/v1");
        plan.auth = AuthMode::ChatGpt { plan: false };
        assert!(!plan.is_configured());
        plan.auth = AuthMode::ChatGpt { plan: true };
        assert!(plan.is_configured());
    }

    #[test]
    fn plan_requests_follow_the_responses_rules() {
        let mut plan = settings("https://api.openai.com/v1");
        plan.auth = AuthMode::ChatGpt { plan: true };
        plan.model = "gpt-6.1-sol".into();
        let parts = [
            Part::Text("Page 1".into()),
            Part::Image(std::sync::Arc::new("AAAA".into())),
        ];
        let body = plan_request_body(&plan, "system", &parts, "s", &json!({}));
        assert_eq!(body["store"], false);
        assert_eq!(body["stream"], true);
        assert_eq!(body["instructions"], "system");
        assert_eq!(body["input"][0]["content"][1]["type"], "input_image");
        assert_eq!(
            body["input"][0]["content"][1]["image_url"],
            "data:image/jpeg;base64,AAAA"
        );
        for unsupported in [
            "temperature",
            "max_output_tokens",
            "top_p",
            "metadata",
            "user",
        ] {
            assert!(
                body.get(unsupported).is_none(),
                "{unsupported} is refused on this route"
            );
        }
        let mut body = body;
        assert!(relax(&mut body, "reasoning.effort"));
        assert!(body.get("reasoning").is_none());
        assert!(relax(&mut body, "text.format"));
        assert_eq!(body["text"]["format"]["type"], "json_object");
        assert!(relax(&mut body, "text"));
        assert!(body.get("text").is_none());
        assert!(!relax(&mut body, "input"));
    }

    #[test]
    fn streams_are_read_to_completion() {
        let raw = "event: response.created\ndata: {\"type\":\"response.created\"}\n\n\
                   data: {\"type\":\"response.output_text.delta\",\"delta\":\"{\\\"scenes\\\":\"}\n\n\
                   data: {\"type\":\"response.output_text.delta\",\"delta\":\"[]}\"}\n\n\
                   data: {\"type\":\"response.completed\",\"response\":{}}\n\n";
        assert_eq!(parse_stream(raw), Streamed::Text("{\"scenes\":[]}".into()));
        let failed = "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"subscription_sharing_usage_limit_exceeded\",\"message\":\"limit\"}}}\n";
        let Streamed::Failed(code, ..) = parse_stream(failed) else {
            panic!()
        };
        assert_eq!(code, "subscription_sharing_usage_limit_exceeded");
        assert!(plan_error(429, &code, "limit").pause.is_some());
        assert!(matches!(
            parse_stream("data: {\"type\":\"response.output_text.delta\",\"delta\":\"{\"}\n"),
            Streamed::Failed(ref c, ..) if c == "interrupted"
        ));
    }

    #[test]
    fn plan_models_keep_listed_ones() {
        let body = json!({ "models": [
            { "slug": "gpt-6.1-sol", "display_name": "GPT-6.1 Sol", "visibility": "list" },
            { "slug": "internal", "display_name": "Hidden", "visibility": "hide" },
            { "slug": "gpt-6-luna", "display_name": "GPT-6 Luna", "visibility": "list" }
        ]});
        let models = parse_plan_models(&body);
        assert_eq!(models.len(), 2);
        assert_eq!(
            models[0],
            ModelInfo {
                id: "gpt-6.1-sol".into(),
                name: "GPT-6.1 Sol".into()
            }
        );
        assert!(plan_error(403, "subscription_sharing_user_not_eligible", "").settings);
        assert!(plan_error(503, "", "busy").retryable);
        assert_eq!(
            error_parts(&json!({ "detail": "not enabled" }), "").1,
            "not enabled"
        );
    }
}
