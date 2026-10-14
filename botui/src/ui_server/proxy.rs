use axum::{
    body::Body,
    extract::{OriginalUri, State},
    http::{HeaderMap, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use log::{debug, error, warn};
use serde::Deserialize;

use crate::shared::AppState;

fn extract_app_context(headers: &HeaderMap, path: &str) -> Option<String> {
    if let Some(referer) = headers.get("referer") {
        if let Ok(referer_str) = referer.to_str() {
            if let Some(start) = referer_str.find("/apps/") {
                let after_apps = &referer_str[start + 6..];
                if let Some(end) = after_apps.find('/') {
                    return Some(after_apps[..end].to_string());
                } else if !after_apps.is_empty() {
                    return Some(after_apps.to_string());
                }
            }
            if let Some(_start) = referer_str.find(".gb.solutions") {
                if let Some(host_start) = referer_str.find("://") {
                    let host_part = &referer_str[host_start + 3..];
                    if let Some(dot) = host_part.find('.') {
                        return Some(host_part[..dot].to_string());
                    }
                }
            }
        }
    }

    if let Some(after_apps) = path.strip_prefix("/apps/") {
        if let Some(end) = after_apps.find('/') {
            return Some(after_apps[..end].to_string());
        }
    }

    None
}

pub async fn proxy_api(
    State(state): State<AppState>,
    original_uri: OriginalUri,
    req: Request<Body>,
) -> Response<Body> {
    let path = original_uri.path();
    let query = original_uri
        .query()
        .map_or_else(String::new, |q| format!("?{q}"));
    let method = req.method().clone();
    let headers = req.headers().clone();

    let app_context = extract_app_context(&headers, path);

    let target_url = format!("{}{path}{query}", state.client.base_url());
    debug!("Proxying {method} {path} to {target_url} (app: {app_context:?})");

    let client = crate::ui_server::tls_policy::backend_http_client(state.client.base_url());
    let mut proxy_req = client.request(method.clone(), &target_url);

    if let Some(host) = headers.get("host").and_then(|value| value.to_str().ok()) {
        proxy_req = proxy_req.header("x-forwarded-host", host);
    }
    if !headers.contains_key("x-forwarded-proto") {
        proxy_req = proxy_req.header("x-forwarded-proto", "http");
    }

    for (name, value) in &headers {
        if name != "host" {
            if let Ok(v) = value.to_str() {
                proxy_req = proxy_req.header(name.as_str(), v);
            }
        }
    }

    if let Some(app) = app_context {
        proxy_req = proxy_req.header("X-App-Context", app);
    }

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {e}");
            return build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read request body",
            );
        }
    };

    if !body_bytes.is_empty() {
        proxy_req = proxy_req.body(body_bytes.to_vec());
    }

    match proxy_req.send().await {
        Ok(resp) => build_proxy_response(resp).await,
        Err(e) => {
            error!("Proxy request failed: {e}");
            build_error_response(StatusCode::BAD_GATEWAY, &format!("Proxy error: {e}"))
        }
    }
}

fn build_error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(message.to_string()))
        .unwrap_or_else(|_| {
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to build error response"))
                .unwrap_or_default()
        })
}

async fn build_proxy_response(resp: reqwest::Response) -> Response<Body> {
    let status = resp.status();
    let headers = resp.headers().clone();

    match resp.bytes().await {
        Ok(body) => {
            let mut response = Response::builder().status(status);

            for (name, value) in &headers {
                response = response.header(name, value);
            }

            response.body(Body::from(body)).unwrap_or_else(|_| {
                build_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to build response",
                )
            })
        }
        Err(e) => {
            error!("Failed to read response body: {e}");
            build_error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to read response: {e}"),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ClientError {
    message: String,
    stack: Option<String>,
    source: String,
    url: String,
    user_agent: String,
    timestamp: String,
}

pub async fn handle_client_error(Json(error): Json<ClientError>) -> impl IntoResponse {
    warn!(
        "CLIENT:{}: {} at {} ({}) - {}",
        error.source.to_uppercase(),
        error.message,
        error.url,
        error.timestamp,
        error.user_agent
    );

    if let Some(stack) = &error.stack {
        if !stack.is_empty() {
            warn!("CLIENT:STACK: {}", stack);
        }
    }

    StatusCode::OK
}
