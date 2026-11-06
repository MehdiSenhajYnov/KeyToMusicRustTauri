//! "Sign in with ChatGPT": Mood AI requests use the user's ChatGPT plan
//! instead of an API key (OpenAI's flow for open-source, locally run apps).
//!
//! The browser opens OpenAI's sign-in page; OpenAI redirects to a loopback
//! listener here with a code, exchanged (PKCE, no secret) for tokens. The
//! first sign-in registers KeyToMusic for that account and returns its own
//! `client_id`, kept for later sign-ins. Access tokens last an hour and are
//! refreshed on demand; everything is stored owner-only in `mood_chatgpt.json`.
//!
//! Reference: developers.openai.com/siwc/token-sharing-open-source

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const ISSUER: &str = "https://auth.openai.com";
/// Audience of the tokens: the API the plan pays for.
pub const RESOURCE: &str = "https://api.openai.com/v1";

/// Where the flow talks to (OpenAI; a local server in tests).
#[derive(Clone, Debug)]
pub struct Endpoints {
    pub authorize: String,
    pub token: String,
    pub revoke: String,
    pub jwks: String,
    /// Base of the Responses and models API.
    pub api: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            authorize: "https://auth.openai.com/api/accounts/authorize".into(),
            token: "https://auth.openai.com/api/accounts/oauth/token".into(),
            revoke: "https://auth.openai.com/api/accounts/oauth/revoke".into(),
            jwks: "https://auth.openai.com/.well-known/jwks.json".into(),
            api: RESOURCE.into(),
        }
    }
}
const REGISTRATION_CLIENT: &str = "dynamic_agent_client";
const APP_NAME: &str = "KeyToMusic";
const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
/// The scope that lets requests use the ChatGPT plan.
pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
const PREFERRED_PORT: u16 = 1455;
const CALLBACK_PATH: &str = "/auth/callback";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(600);
/// Refresh this long before the access token expires.
const REFRESH_MARGIN: u64 = 120;
pub const USAGE_URL: &str = "https://chatgpt.com/settings/usage";

/// What is stored on disk (0600).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Store {
    /// Stable id of this installation (`urn:uuid:…`), sent on every sign-in.
    ext_agent_host_id: Option<String>,
    /// Issued for this account at its first sign-in; reused afterwards.
    client_id: Option<String>,
    email: Option<String>,
    subject: Option<String>,
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
    /// Unix seconds.
    expires_at: Option<u64>,
}

/// The signed-in account, for the UIs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub email: Option<String>,
    /// The user agreed to let KeyToMusic use their ChatGPT plan.
    pub plan_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AuthError {
    pub message: String,
    /// Signing in again is needed.
    pub sign_in_needed: bool,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn err(message: impl Into<String>) -> AuthError {
    AuthError {
        message: message.into(),
        sign_in_needed: false,
    }
}

fn sign_in_again(message: impl Into<String>) -> AuthError {
    AuthError {
        message: message.into(),
        sign_in_needed: true,
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

fn random_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn code_challenge(verifier: &str) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()))
}

/// One sign-in in progress.
struct Attempt {
    state: String,
    nonce: String,
    verifier: String,
    redirect_uri: String,
    /// Saved issued id, or None for a first registration.
    client_id: Option<String>,
}

pub struct ChatGptAuth {
    endpoints: Endpoints,
    path: Option<PathBuf>,
    store: Mutex<Store>,
    /// Refreshes are serialized: refresh tokens rotate.
    refresh: tokio::sync::Mutex<()>,
    sign_in: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

static GLOBAL: OnceLock<Arc<ChatGptAuth>> = OnceLock::new();

/// The account of this installation (stored in the app data folder).
pub fn global() -> Arc<ChatGptAuth> {
    GLOBAL
        .get_or_init(|| {
            Arc::new(ChatGptAuth::new(Some(
                crate::storage::get_app_data_dir().join("mood_chatgpt.json"),
            )))
        })
        .clone()
}

/// Replace the account used by Mood AI (tests, tools). Fails once in use.
pub fn install_global(auth: Arc<ChatGptAuth>) -> Result<(), Arc<ChatGptAuth>> {
    GLOBAL.set(auth)
}

fn form_body(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("KeyToMusic/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_default()
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<u64>,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct IdClaims {
    sub: String,
    email: Option<String>,
    nonce: Option<String>,
}

impl ChatGptAuth {
    pub fn new(path: Option<PathBuf>) -> Self {
        Self::with_endpoints(path, Endpoints::default())
    }

    pub fn with_endpoints(path: Option<PathBuf>, endpoints: Endpoints) -> Self {
        let store = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Self {
            endpoints,
            path,
            store: Mutex::new(store),
            refresh: tokio::sync::Mutex::new(()),
            sign_in: Mutex::new(None),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn save(&self, store: &Store) {
        let Some(path) = &self.path else { return };
        let Ok(bytes) = serde_json::to_vec_pretty(store) else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = path.with_extension("json.tmp");
        if write_private(&tmp, &bytes).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }

    /// Signed-in account, if any.
    pub fn account(&self) -> Option<Account> {
        let store = self.lock();
        store.refresh_token.as_ref().map(|_| Account {
            email: store.email.clone(),
            plan_enabled: store.scopes.iter().any(|s| s == PLAN_SCOPE),
        })
    }

    /// Base URL of the API requests go to.
    pub fn api_base(&self) -> &str {
        &self.endpoints.api
    }

    /// Ready to make requests on the user's plan.
    pub fn can_use_plan(&self) -> bool {
        self.account().is_some_and(|a| a.plan_enabled)
    }

    fn host_id(&self) -> String {
        let mut store = self.lock();
        if let Some(id) = &store.ext_agent_host_id {
            return id.clone();
        }
        let id = format!("urn:uuid:{}", uuid::Uuid::new_v4());
        store.ext_agent_host_id = Some(id.clone());
        let snapshot = store.clone();
        drop(store);
        self.save(&snapshot);
        id
    }

    fn authorize_url(&self, attempt: &Attempt, force_consent: bool) -> String {
        let host_id = self.host_id();
        let store = self.lock().clone();
        let mut params: Vec<(&str, String)> = vec![
            (
                "client_id",
                attempt
                    .client_id
                    .clone()
                    .unwrap_or_else(|| REGISTRATION_CLIENT.into()),
            ),
            ("ext_agent_host_id", host_id),
            ("response_type", "code".into()),
            ("redirect_uri", attempt.redirect_uri.clone()),
            ("scope", SCOPES.into()),
            ("resource", RESOURCE.into()),
            ("state", attempt.state.clone()),
            ("nonce", attempt.nonce.clone()),
            ("code_challenge_method", "S256".into()),
            ("code_challenge", code_challenge(&attempt.verifier)),
        ];
        if attempt.client_id.is_none() {
            params.push(("agent_name_hint", APP_NAME.into()));
        } else {
            if let Some(hint) = store.id_token.clone() {
                params.push(("id_token_hint", hint));
            }
            if let Some(email) = store.email.clone() {
                params.push(("login_hint", email));
            }
        }
        if force_consent {
            params.push(("prompt", "consent".into()));
        }
        let query = params
            .iter()
            .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        format!("{}?{query}", self.endpoints.authorize)
    }

    /// Start signing in: returns the page to open in the browser. `done`
    /// runs once with the result (the account, or why it failed).
    pub async fn begin_sign_in(
        self: &Arc<Self>,
        done: impl FnOnce(Result<Account, AuthError>) + Send + 'static,
    ) -> Result<String, AuthError> {
        if let Some(previous) = self
            .sign_in
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            previous.abort();
        }
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PREFERRED_PORT)).await {
            Ok(listener) => listener,
            Err(_) => tokio::net::TcpListener::bind(("127.0.0.1", 0))
                .await
                .map_err(|e| err(format!("Impossible d'écouter le retour de connexion : {e}")))?,
        };
        let port = listener
            .local_addr()
            .map_err(|e| err(e.to_string()))?
            .port();
        let (client_id, had_plan) = {
            let store = self.lock();
            (
                store.client_id.clone(),
                store.scopes.iter().any(|s| s == PLAN_SCOPE),
            )
        };
        let attempt = Attempt {
            state: random_token(),
            nonce: random_token(),
            verifier: random_token(),
            redirect_uri: format!("http://127.0.0.1:{port}{CALLBACK_PATH}"),
            // Ask again for the plan if it was declined last time.
            client_id,
        };
        let force_consent = attempt.client_id.is_some() && !had_plan && self.account().is_some();
        let url = self.authorize_url(&attempt, force_consent);
        let me = self.clone();
        let task = tokio::spawn(async move {
            let result =
                tokio::time::timeout(SIGN_IN_TIMEOUT, me.finish_sign_in(listener, attempt))
                    .await
                    .unwrap_or_else(|_| Err(err("Connexion abandonnée (délai dépassé)")));
            done(result);
        });
        *self.sign_in.lock().unwrap_or_else(|e| e.into_inner()) = Some(task);
        Ok(url)
    }

    async fn finish_sign_in(
        &self,
        listener: tokio::net::TcpListener,
        attempt: Attempt,
    ) -> Result<Account, AuthError> {
        let query = wait_for_callback(&listener, &attempt.state).await?;
        let param = |name: &str| {
            query
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        };
        if let Some(error) = param("error") {
            return Err(err(if error == "access_denied" {
                "Connexion refusée dans le navigateur".to_string()
            } else {
                format!(
                    "Connexion impossible : {error} {}",
                    param("error_description").unwrap_or_default()
                )
            }));
        }
        let code = param("code").ok_or_else(|| err("Retour de connexion sans code"))?;
        let client_id = match (&attempt.client_id, param("client_id")) {
            (Some(saved), Some(returned)) if *saved != returned => {
                return Err(err(
                    "Le compte renvoyé ne correspond pas à l'inscription enregistrée",
                ))
            }
            (Some(saved), _) => saved.clone(),
            (None, Some(issued)) if issued != REGISTRATION_CLIENT => issued,
            (None, _) => {
                return Err(err(
                    "Inscription incomplète : OpenAI n'a pas renvoyé d'identifiant d'app",
                ))
            }
        };

        let response = client()
            .post(&self.endpoints.token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body(&[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", &code),
                ("code_verifier", &attempt.verifier),
                ("redirect_uri", &attempt.redirect_uri),
                ("resource", RESOURCE),
            ]))
            .send()
            .await
            .map_err(|e| err(format!("Échange du code impossible : {e}")))?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(err(format!(
                "Échange du code refusé (HTTP {status}) : {body}"
            )));
        }
        let tokens: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| err(format!("Réponse de jeton illisible : {e}")))?;
        let id_token = tokens
            .id_token
            .clone()
            .ok_or_else(|| err("Réponse sans jeton d'identité"))?;
        let claims =
            validate_id_token(&self.endpoints.jwks, &id_token, &client_id, &attempt.nonce).await?;

        let scopes: Vec<String> = tokens
            .scope
            .clone()
            .or_else(|| param("scope"))
            .unwrap_or_default()
            .split([' ', '+'])
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        let mut store = self.lock();
        if store.subject.as_ref().is_some_and(|s| *s != claims.sub) && attempt.client_id.is_some() {
            return Err(err("Ce n'est pas le compte ChatGPT enregistré"));
        }
        store.client_id = Some(client_id);
        store.subject = Some(claims.sub);
        store.email = claims.email.or(store.email.take());
        store.id_token = Some(id_token);
        store.access_token = Some(tokens.access_token);
        store.refresh_token = tokens.refresh_token.or(store.refresh_token.take());
        store.expires_at = Some(now() + tokens.expires_in.unwrap_or(3600));
        store.scopes = scopes;
        let snapshot = store.clone();
        drop(store);
        self.save(&snapshot);
        self.account()
            .ok_or_else(|| err("Connexion sans jeton de renouvellement (offline_access refusé)"))
    }

    /// A valid access token, refreshed when about to expire.
    pub async fn access_token(&self) -> Result<String, AuthError> {
        self.token(false).await
    }

    /// After a 401: refresh even if the token looks valid.
    pub async fn refreshed_token(&self) -> Result<String, AuthError> {
        self.token(true).await
    }

    async fn token(&self, force: bool) -> Result<String, AuthError> {
        let _serial = self.refresh.lock().await;
        let store = self.lock().clone();
        let (Some(client_id), Some(refresh_token)) = (store.client_id, store.refresh_token) else {
            return Err(sign_in_again("Connecte-toi avec ChatGPT"));
        };
        if !force {
            if let (Some(token), Some(expires_at)) = (store.access_token, store.expires_at) {
                if expires_at > now() + REFRESH_MARGIN {
                    return Ok(token);
                }
            }
        }
        let response = client()
            .post(&self.endpoints.token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body(&[
                ("grant_type", "refresh_token"),
                ("client_id", &client_id),
                ("refresh_token", &refresh_token),
                ("resource", RESOURCE),
            ]))
            .send()
            .await
            .map_err(|e| {
                err(format!(
                    "Renouvellement de la session ChatGPT impossible : {e}"
                ))
            })?;
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            let value: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let code = value
                .get("error")
                .and_then(|e| {
                    e.as_str()
                        .or_else(|| e.get("code").and_then(|c| c.as_str()))
                })
                .unwrap_or_default();
            let unusable = [
                "invalid_grant",
                "invalid_refresh_token",
                "token_expired",
                "refresh_token_expired",
                "refresh_token_invalidated",
                "refresh_token_reused",
            ];
            if unusable.contains(&code) || status.as_u16() == 401 {
                self.clear_tokens();
                return Err(sign_in_again(
                    "Session ChatGPT expirée ou déconnectée : reconnecte-toi",
                ));
            }
            if code == "invalid_client" {
                return Err(sign_in_again(
                    "Inscription de l'app refusée par OpenAI : reconnecte-toi",
                ));
            }
            return Err(err(format!(
                "Renouvellement refusé (HTTP {status}) : {body}"
            )));
        }
        let tokens: TokenResponse = serde_json::from_str(&body)
            .map_err(|e| err(format!("Réponse de jeton illisible : {e}")))?;
        let mut store = self.lock();
        store.access_token = Some(tokens.access_token.clone());
        if let Some(refresh) = tokens.refresh_token {
            store.refresh_token = Some(refresh);
        }
        if let Some(id_token) = tokens.id_token {
            store.id_token = Some(id_token);
        }
        if let Some(scope) = tokens.scope {
            store.scopes = scope
                .split(' ')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        store.expires_at = Some(now() + tokens.expires_in.unwrap_or(3600));
        let snapshot = store.clone();
        drop(store);
        self.save(&snapshot);
        Ok(tokens.access_token)
    }

    fn clear_tokens(&self) {
        let mut store = self.lock();
        store.access_token = None;
        store.refresh_token = None;
        store.expires_at = None;
        store.scopes.clear();
        let snapshot = store.clone();
        drop(store);
        self.save(&snapshot);
    }

    /// Revoke the session at OpenAI (best effort) and forget the tokens. The
    /// app registration and the host id stay, for the next sign-in.
    pub async fn sign_out(&self) -> Result<(), AuthError> {
        let (client_id, refresh_token) = {
            let store = self.lock();
            (store.client_id.clone(), store.refresh_token.clone())
        };
        let mut confirmed = true;
        if let (Some(client_id), Some(token)) = (client_id, refresh_token) {
            confirmed = client()
                .post(&self.endpoints.revoke)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(form_body(&[
                    ("token", &token),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", &client_id),
                ]))
                .send()
                .await
                .is_ok_and(|r| r.status().is_success());
        }
        self.clear_tokens();
        if confirmed {
            Ok(())
        } else {
            Err(err(
                "Déconnecté ici, mais OpenAI n'a pas confirmé : retire l'app dans les réglages ChatGPT si besoin",
            ))
        }
    }
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, bytes)
}

/// Check the ID token's signature (OpenAI's JWKS), issuer, audience, expiry and nonce.
async fn validate_id_token(
    jwks_url: &str,
    token: &str,
    client_id: &str,
    nonce: &str,
) -> Result<IdClaims, AuthError> {
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| err(format!("Jeton d'identité illisible : {e}")))?;
    let jwks: jsonwebtoken::jwk::JwkSet = client()
        .get(jwks_url)
        .send()
        .await
        .map_err(|e| err(format!("Clés de signature OpenAI inaccessibles : {e}")))?
        .json()
        .await
        .map_err(|e| err(format!("Clés de signature OpenAI illisibles : {e}")))?;
    let jwk = match header.kid.as_deref() {
        Some(kid) => jwks.find(kid),
        None => jwks.keys.first(),
    }
    .ok_or_else(|| err("Clé de signature inconnue"))?;
    let key = jsonwebtoken::DecodingKey::from_jwk(jwk)
        .map_err(|e| err(format!("Clé de signature inutilisable : {e}")))?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_issuer(&[ISSUER]);
    validation.set_audience(&[client_id]);
    let data = jsonwebtoken::decode::<IdClaims>(token, &key, &validation)
        .map_err(|e| err(format!("Jeton d'identité refusé : {e}")))?;
    if data.claims.nonce.as_deref() != Some(nonce) {
        return Err(err("Jeton d'identité rejoué (nonce différent)"));
    }
    Ok(data.claims)
}

/// Serve the loopback callback until the browser comes back with our `state`.
async fn wait_for_callback(
    listener: &tokio::net::TcpListener,
    state: &str,
) -> Result<Vec<(String, String)>, AuthError> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|e| err(format!("Retour de connexion impossible : {e}")))?;
        let mut buffer = vec![0u8; 16 * 1024];
        let mut read = 0;
        // The request line is all we need.
        while read < buffer.len() {
            match socket.read(&mut buffer[read..]).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    read += n;
                    if buffer[..read].windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
            }
        }
        let request = String::from_utf8_lossy(&buffer[..read]).to_string();
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or_default()
            .to_string();
        let (path, query) = target.split_once('?').unwrap_or((target.as_str(), ""));
        let params = parse_query(query);
        let matches =
            path == CALLBACK_PATH && params.iter().any(|(k, v)| k == "state" && v == state);
        let (status, page) = if matches {
            let failed = params.iter().any(|(k, _)| k == "error");
            (
                "200 OK",
                if failed {
                    "Connexion annulée. Tu peux fermer cet onglet et revenir à KeyToMusic."
                } else {
                    "Connexion réussie. Tu peux fermer cet onglet et revenir à KeyToMusic."
                },
            )
        } else {
            ("404 Not Found", "Lien inattendu pour KeyToMusic.")
        };
        let html = format!(
            "<!doctype html><html lang=\"fr\"><meta charset=\"utf-8\"><title>KeyToMusic</title>\
             <body style=\"font:16px system-ui;display:grid;place-items:center;height:90vh\"><p>{page}</p></body></html>"
        );
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
            html.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.shutdown().await;
        if matches {
            return Ok(params);
        }
    }
}

fn parse_query(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            let decode = |s: &str| {
                urlencoding::decode(&s.replace('+', " "))
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| s.to_string())
            };
            (decode(k), decode(v))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_unpadded_base64url_sha256() {
        // Expected value computed with Python's hashlib + urlsafe_b64encode.
        assert_eq!(
            code_challenge("keytomusic-verifier"),
            "_losNl9ewMOi3EynENmCTaRVLwVWZFjxRRVumMIPqGU"
        );
        assert_eq!(random_token().len(), 43);
    }

    #[test]
    fn first_sign_in_registers_and_later_ones_reuse_the_client() {
        let dir = tempfile::tempdir().unwrap();
        let auth = ChatGptAuth::new(Some(dir.path().join("a.json")));
        let attempt = Attempt {
            state: "S".into(),
            nonce: "N".into(),
            verifier: "V".into(),
            redirect_uri: "http://127.0.0.1:1455/auth/callback".into(),
            client_id: None,
        };
        let url = auth.authorize_url(&attempt, false);
        assert!(url.starts_with(
            "https://auth.openai.com/api/accounts/authorize?client_id=dynamic_agent_client&"
        ));
        assert!(url.contains("agent_name_hint=KeyToMusic"));
        assert!(url.contains("ext_agent_host_id=urn%3Auuid%3A"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A1455%2Fauth%2Fcallback"));
        assert!(url.contains("scope=openid%20profile%20email%20offline_access%20resource.invoke%20chatgpt.tokens.use.direct"));
        assert!(url.contains("resource=https%3A%2F%2Fapi.openai.com%2Fv1"));
        assert!(url.contains("code_challenge_method=S256"));
        let host = auth.host_id();

        // The host id survives a restart; the issued client is reused.
        {
            let mut store = auth.lock();
            store.client_id = Some("oaiapp_123".into());
            store.email = Some("me@example.com".into());
            let snapshot = store.clone();
            drop(store);
            auth.save(&snapshot);
        }
        let again = ChatGptAuth::new(Some(dir.path().join("a.json")));
        assert_eq!(again.host_id(), host);
        let attempt = Attempt {
            client_id: Some("oaiapp_123".into()),
            ..attempt
        };
        let url = again.authorize_url(&attempt, false);
        assert!(url.contains("client_id=oaiapp_123"));
        assert!(!url.contains("agent_name_hint"));
        assert!(url.contains("login_hint=me%40example.com"));
    }

    #[test]
    fn callback_queries_are_decoded() {
        let q = parse_query("code=a%2Fb&scope=openid+email&state=S");
        assert_eq!(q[0], ("code".into(), "a/b".into()));
        assert_eq!(q[1], ("scope".into(), "openid email".into()));
    }

    #[tokio::test]
    async fn the_loopback_listener_ignores_other_requests() {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();
        let waiting = tokio::spawn(async move { wait_for_callback(&listener, "good").await });
        let http = reqwest::Client::new();
        let stray = http
            .get(format!("http://127.0.0.1:{port}/favicon.ico"))
            .send()
            .await
            .unwrap();
        assert_eq!(stray.status(), 404);
        let wrong = http
            .get(format!(
                "http://127.0.0.1:{port}/auth/callback?code=x&state=bad"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(wrong.status(), 404);
        let ok = http
            .get(format!(
                "http://127.0.0.1:{port}/auth/callback?code=c0de&state=good&client_id=oaiapp_9"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(ok.status(), 200);
        let params = waiting.await.unwrap().unwrap();
        assert!(params.contains(&("client_id".into(), "oaiapp_9".into())));
    }

    #[tokio::test]
    async fn no_account_means_signing_in() {
        let auth = ChatGptAuth::new(None);
        assert!(auth.account().is_none());
        assert!(auth.access_token().await.unwrap_err().sign_in_needed);
    }
}
