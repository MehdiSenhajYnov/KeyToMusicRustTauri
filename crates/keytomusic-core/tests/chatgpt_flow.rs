//! "Sign in with ChatGPT" end to end against a local stand-in for OpenAI:
//! browser sign-in → loopback callback → code exchange → signed ID token
//! check → model list → streamed Responses request → token refresh → sign-out.
//! (Test-only RSA key in tests/data; it signs nothing real.)

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::extract::{Form, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use keytomusic_core::mood::analyzer::{self, BatchContext};
use keytomusic_core::mood::chatgpt::{self, ChatGptAuth, Endpoints};
use keytomusic_core::mood::provider::{self, AuthMode, ProviderSettings};
use keytomusic_core::types::{BaseMood, MoodImageDetail};
use serde_json::{json, Value};

#[derive(Default)]
struct Fake {
    nonce: String,
    access_token: String,
    refreshes: u32,
    revoked: bool,
    responses_body: Option<Value>,
}

type Shared = Arc<Mutex<Fake>>;

fn id_token(nonce: &str) -> String {
    let key = jsonwebtoken::EncodingKey::from_rsa_pem(include_bytes!("data/test_rsa.pem")).unwrap();
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("test-key".into());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    jsonwebtoken::encode(
        &header,
        &json!({
            "iss": "https://auth.openai.com",
            "aud": "oaiapp_test",
            "sub": "user-123",
            "email": "reader@example.com",
            "nonce": nonce,
            "iat": now,
            "exp": now + 3600
        }),
        &key,
    )
    .unwrap()
}

async fn token(
    State(fake): State<Shared>,
    Form(form): Form<HashMap<String, String>>,
) -> impl IntoResponse {
    let mut fake = fake.lock().unwrap();
    assert_eq!(
        form["client_id"], "oaiapp_test",
        "the issued client, never the registration placeholder"
    );
    assert_eq!(form["resource"], "https://api.openai.com/v1");
    match form["grant_type"].as_str() {
        "authorization_code" => {
            assert_eq!(form["code"], "c0de");
            assert!(form["code_verifier"].len() >= 43);
            assert!(form["redirect_uri"].starts_with("http://127.0.0.1:"));
            fake.access_token = "access-1".into();
            (
                StatusCode::OK,
                Json(json!({
                    "access_token": "access-1",
                    "refresh_token": "refresh-1",
                    "id_token": id_token(&fake.nonce),
                    "token_type": "Bearer",
                    "expires_in": 3600,
                    "scope": "chatgpt.tokens.use.direct email offline_access openid profile resource.invoke"
                })),
            )
        }
        "refresh_token" => {
            fake.refreshes += 1;
            assert_eq!(form["refresh_token"], format!("refresh-{}", fake.refreshes));
            fake.access_token = format!("access-{}", fake.refreshes + 1);
            (
                StatusCode::OK,
                Json(json!({
                    "access_token": fake.access_token,
                    "refresh_token": format!("refresh-{}", fake.refreshes + 1),
                    "expires_in": 3600
                })),
            )
        }
        other => panic!("unexpected grant {other}"),
    }
}

fn authorized(fake: &Shared, headers: &HeaderMap) -> bool {
    let expected = format!("Bearer {}", fake.lock().unwrap().access_token);
    headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(expected.as_str())
}

async fn models(State(fake): State<Shared>, headers: HeaderMap) -> impl IntoResponse {
    if !authorized(&fake, &headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "detail": "bad token" })),
        );
    }
    (
        StatusCode::OK,
        Json(json!({ "models": [
            { "slug": "gpt-6.1-sol", "display_name": "GPT-6.1 Sol", "visibility": "list" },
            { "slug": "gpt-6.1-terra", "display_name": "GPT-6.1 Terra", "visibility": "list" },
            { "slug": "gpt-6-luna", "display_name": "GPT-6 Luna", "visibility": "list" },
            { "slug": "codex-internal", "display_name": "Internal", "visibility": "hide" }
        ]})),
    )
}

async fn responses(
    State(fake): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if !authorized(&fake, &headers) {
        return (StatusCode::UNAUTHORIZED, String::new());
    }
    // Like the plan route: no reasoning effort for this model.
    if body.get("reasoning").is_some() {
        return (
            StatusCode::BAD_REQUEST,
            json!({ "error": {
                "code": "subscription_sharing_unsupported_capability",
                "param": "reasoning.effort",
                "message": "unsupported"
            }})
            .to_string(),
        );
    }
    fake.lock().unwrap().responses_body = Some(body);
    let answer = json!({ "scenes": [
        { "first_page": 1, "last_page": 2, "mood": "tension", "intensity": 2, "summary": "ambush" },
        { "first_page": 3, "last_page": 3, "mood": "comedy", "intensity": 1, "summary": "gag" }
    ]})
    .to_string();
    let (a, b) = answer.split_at(answer.len() / 2);
    let event = |v: Value| format!("event: {}\ndata: {}\n\n", v["type"].as_str().unwrap(), v);
    let stream = [
        event(json!({ "type": "response.created" })),
        event(json!({ "type": "response.output_text.delta", "delta": a })),
        event(json!({ "type": "response.output_text.delta", "delta": b })),
        event(json!({ "type": "response.completed", "response": { "status": "completed" } })),
    ]
    .concat();
    (StatusCode::OK, stream)
}

#[tokio::test(flavor = "multi_thread")]
async fn signing_in_with_chatgpt_pays_requests_with_the_plan() {
    let fake: Shared = Arc::default();
    let app = Router::new()
        .route("/token", post(token))
        .route(
            "/revoke",
            post(
                |State(fake): State<Shared>, Form(form): Form<HashMap<String, String>>| async move {
                    assert_eq!(form["token_type_hint"], "refresh_token");
                    fake.lock().unwrap().revoked = true;
                    StatusCode::OK
                },
            ),
        )
        .route(
            "/jwks",
            get(|| async {
                Json(serde_json::from_str::<Value>(include_str!("data/test_jwks.json")).unwrap())
            }),
        )
        .route("/v1/models", get(models))
        .route("/v1/responses", post(responses))
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let auth = Arc::new(ChatGptAuth::with_endpoints(
        Some(dir.path().join("mood_chatgpt.json")),
        Endpoints {
            authorize: format!("{base}/authorize"),
            token: format!("{base}/token"),
            revoke: format!("{base}/revoke"),
            jwks: format!("{base}/jwks"),
            api: format!("{base}/v1"),
        },
    ));
    assert!(chatgpt::install_global(auth.clone()).is_ok());

    // 1. The app opens this page in the browser.
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let url = auth
        .begin_sign_in(move |result| {
            let _ = sender.send(result);
        })
        .await
        .unwrap();
    let query: HashMap<String, String> = url
        .split_once('?')
        .unwrap()
        .1
        .split('&')
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap();
            (k.to_string(), urlencoding::decode(v).unwrap().into_owned())
        })
        .collect();
    assert_eq!(query["client_id"], "dynamic_agent_client");
    assert_eq!(query["agent_name_hint"], "KeyToMusic");
    assert!(query["scope"].contains("chatgpt.tokens.use.direct"));
    assert!(query["redirect_uri"].ends_with("/auth/callback"));
    fake.lock().unwrap().nonce = query["nonce"].clone();

    // 2. OpenAI sends the browser back with a code and the issued client id.
    let callback = format!(
        "{}?code=c0de&state={}&client_id=oaiapp_test&scope=chatgpt.tokens.use.direct+openid",
        query["redirect_uri"], query["state"]
    );
    let page = reqwest::get(&callback).await.unwrap();
    assert_eq!(page.status(), 200);
    assert!(page.text().await.unwrap().contains("Connexion réussie"));
    let account = receiver.await.unwrap().unwrap();
    assert_eq!(account.email.as_deref(), Some("reader@example.com"));
    assert!(account.plan_enabled);
    let saved = std::fs::read_to_string(dir.path().join("mood_chatgpt.json")).unwrap();
    assert!(saved.contains("oaiapp_test") && saved.contains("refresh-1"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("mood_chatgpt.json"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    // 3. The model picker lists the account's models.
    let settings = ProviderSettings {
        auth: AuthMode::ChatGpt { plan: true },
        base_url: String::new(),
        model: "gpt-6.1-sol".into(),
        api_key: None,
        image_detail: MoodImageDetail::High,
    };
    let models = provider::list_models(&settings).await.unwrap();
    assert_eq!(
        models.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(),
        ["GPT-6.1 Sol", "GPT-6.1 Terra", "GPT-6 Luna"]
    );

    // 4. A chapter batch goes through the Responses API, streamed.
    let image = keytomusic_core::mood::image::page(vec!["AAAA".to_string()]);
    let pages = vec![(0, image.clone()), (1, image.clone()), (2, image)];
    let labels = analyzer::analyze(&settings, &pages, &BatchContext::default())
        .await
        .unwrap();
    assert_eq!(labels[&0].mood, BaseMood::Tension);
    assert_eq!(labels[&2].mood, BaseMood::Comedy);
    let body = fake.lock().unwrap().responses_body.clone().unwrap();
    assert_eq!(body["model"], "gpt-6.1-sol");
    assert_eq!(body["store"], false);
    assert_eq!(body["stream"], true);
    assert_eq!(body["text"]["format"]["type"], "json_schema");

    // 5. An expired token is refreshed transparently (refresh tokens rotate).
    let refreshed = auth.refreshed_token().await.unwrap();
    assert_eq!(refreshed, "access-2");
    assert!(provider::list_models(&settings).await.is_ok());
    assert_eq!(auth.refreshed_token().await.unwrap(), "access-3");

    // 6. Signing out revokes the session and keeps the registration.
    auth.sign_out().await.unwrap();
    assert!(fake.lock().unwrap().revoked);
    assert!(auth.account().is_none());
    let saved = std::fs::read_to_string(dir.path().join("mood_chatgpt.json")).unwrap();
    assert!(saved.contains("oaiapp_test") && !saved.contains("refresh-"));
}
