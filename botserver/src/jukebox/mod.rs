use axum::{
    body::Body,
    extract::{Path, Query},
    http::{header, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
struct AudioQuery {
    path: String,
}

fn valid_audio_path(path: &str) -> bool {
    if path.is_empty() || path.len() > 4096 || path.contains('\0') {
        return false;
    }
    if path
        .split(['/', '\\'])
        .any(|segment| segment == "..")
    {
        return false;
    }
    let bytes = path.as_bytes();
    let windows_absolute = bytes.len() > 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'/' || bytes[2] == b'\\');
    path.starts_with('/') || windows_absolute
}

fn botmodels_url() -> String {
    std::env::var("BOTMODELS_HOST")
        .or_else(|_| std::env::var("BOTMODELS_URL"))
        .unwrap_or_else(|_| "http://localhost:8085".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn botmodels_key() -> String {
    std::env::var("BOTMODELS_API_KEY").unwrap_or_else(|_| "starter".to_string())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(35))
        .build()
        .map_err(|error| format!("Could not create BotModels client: {error}"))
}

fn status_from_upstream(status: reqwest::StatusCode) -> StatusCode {
    StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY)
}

fn proxy_error(context: &str, error: &dyn std::fmt::Display) -> axum::response::Response {
    tracing::error!(%error, "{context}");
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "error": "Music generation service is unavailable",
            "context": context,
        })),
    )
        .into_response()
}

async fn json_response(request: reqwest::RequestBuilder) -> axum::response::Response {
    let upstream = match request.send().await {
        Ok(response) => response,
        Err(error) => return proxy_error("BotModels request failed", &error),
    };
    let status = status_from_upstream(upstream.status());
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| header::HeaderValue::from_static("application/json"));
    let bytes = match upstream.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return proxy_error("BotModels response could not be read", &error),
    };

    let mut builder = Response::builder().status(status);
    builder = builder.header(header::CONTENT_TYPE, content_type);
    match builder.body(Body::from(bytes)) {
        Ok(response) => response,
        Err(error) => proxy_error("BotModels response could not be built", &error),
    }
}

async fn generate(Json(mut payload): Json<Value>) -> axum::response::Response {
    // JukeBox is an instrumental radio. Enforce this at the trusted API boundary so
    // a modified client cannot accidentally request vocals or submit lyrics.
    let object = match payload.as_object_mut() {
        Some(object) => object,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid music generation request" })),
            )
                .into_response();
        }
    };
    object.insert("instrumental".to_string(), Value::Bool(true));
    object.insert("lyrics".to_string(), Value::String(String::new()));

    let client = match client() {
        Ok(client) => client,
        Err(error) => return proxy_error("BotModels client configuration failed", &error),
    };
    json_response(
        client
            .post(format!("{}/api/music/generate", botmodels_url()))
            .header("X-API-Key", botmodels_key())
            .json(&payload),
    )
    .await
}

async fn format_input(Json(payload): Json<Value>) -> axum::response::Response {
    let client = match client() {
        Ok(client) => client,
        Err(error) => return proxy_error("BotModels client configuration failed", &error),
    };
    json_response(
        client
            .post(format!("{}/api/music/format", botmodels_url()))
            .header("X-API-Key", botmodels_key())
            .json(&payload),
    )
    .await
}

async fn job_status(Path(job_id): Path<String>) -> axum::response::Response {
    if job_id.is_empty() || job_id.len() > 160 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid job identifier" })),
        )
            .into_response();
    }
    let client = match client() {
        Ok(client) => client,
        Err(error) => return proxy_error("BotModels client configuration failed", &error),
    };
    json_response(
        client
            .get(format!(
                "{}/api/music/jobs/{}",
                botmodels_url(),
                urlencoding::encode(&job_id)
            ))
            .header("X-API-Key", botmodels_key()),
    )
    .await
}

async fn models() -> axum::response::Response {
    proxy_get("/api/music/models").await
}

async fn health() -> axum::response::Response {
    proxy_get("/api/music/health").await
}

async fn proxy_get(path: &str) -> axum::response::Response {
    let client = match client() {
        Ok(client) => client,
        Err(error) => return proxy_error("BotModels client configuration failed", &error),
    };
    json_response(
        client
            .get(format!("{}{}", botmodels_url(), path))
            .header("X-API-Key", botmodels_key()),
    )
    .await
}

async fn audio(Query(query): Query<AudioQuery>) -> axum::response::Response {
    if !valid_audio_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Invalid audio path" })),
        )
            .into_response();
    }
    let client = match client() {
        Ok(client) => client,
        Err(error) => return proxy_error("BotModels client configuration failed", &error),
    };
    let upstream = match client
        .get(format!("{}/api/music/audio", botmodels_url()))
        .header("X-API-Key", botmodels_key())
        .query(&[("path", &query.path)])
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return proxy_error("BotModels audio request failed", &error),
    };
    let status = status_from_upstream(upstream.status());
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| header::HeaderValue::from_static("audio/mpeg"));
    // Buffer the locally generated track before handing it to BotUI. Passing the
    // reqwest stream through two reverse-proxy hops occasionally caused Chromium
    // to reject an otherwise complete WAV transfer with `Failed to fetch`.
    let bytes = match upstream.bytes().await {
        Ok(bytes) => bytes,
        Err(error) => return proxy_error("BotModels audio response could not be read", &error),
    };
    let builder = Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "private, max-age=3600")
        .header(header::CONTENT_LENGTH, bytes.len().to_string());
    match builder.body(Body::from(bytes)) {
        Ok(response) => response,
        Err(error) => proxy_error("BotModels audio response could not be built", &error),
    }
}

pub fn configure_jukebox_routes() -> Router {
    Router::new()
        .route("/api/jukebox/generate", post(generate))
        .route("/api/jukebox/format", post(format_input))
        .route("/api/jukebox/jobs/:job_id", get(job_status))
        .route("/api/jukebox/models", get(models))
        .route("/api/jukebox/health", get(health))
        .route("/api/jukebox/audio", get(audio))
}
