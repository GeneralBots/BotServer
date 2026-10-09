//! Google Vertex AI and Gemini Native API Integration
//! Support for both OpenAI-compatible endpoints and native Google AI Studio / Vertex AI endpoints.

pub(crate) use log::{info, trace};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform";

#[derive(Debug, Deserialize)]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    token_uri: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: std::time::Instant,
}

/// Manages OAuth2 access tokens for Google Vertex AI
#[derive(Debug, Clone)]
pub struct VertexTokenManager {
    credentials_path: String,
    cached_token: Arc<RwLock<Option<CachedToken>>>,
}

#[derive(Debug, Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: u64,
    exp: u64,
}

use crate::LLMProvider;
use serde_json::Value;
use tokio::sync::mpsc;
use async_trait::async_trait;
use futures::StreamExt;

#[derive(Debug)]
pub struct VertexClient {
    client: reqwest::Client,
    base_url: String,
    endpoint_path: String,
    token_manager: Arc<tokio::sync::RwLock<Option<Arc<VertexTokenManager>>>>,
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part2::*;
