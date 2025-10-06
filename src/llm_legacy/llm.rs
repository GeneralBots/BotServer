use actix_web::{web, HttpResponse, Result};
use serde_json::json;
use crate::shared::state::AppState;
use crate::shared::utils::azure_from_config;

pub async fn health() -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(json!({"status": "healthy"})))
}

pub async fn chat_completions_local(
    _data: web::Data<AppState>,
    _payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::NotImplemented().json(json!({"error": "Local LLM not implemented"})))
}

pub async fn embeddings_local(
    _data: web::Data<AppState>,
    _payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::NotImplemented().json(json!({"error": "Local embeddings not implemented"})))
}

pub async fn generic_chat_completions(
    _data: web::Data<AppState>,
    _payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    Ok(HttpResponse::NotImplemented().json(json!({"error": "Generic chat not implemented"})))
}
