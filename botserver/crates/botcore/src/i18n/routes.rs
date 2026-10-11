//! Axum routes exposing the available locales and translation catalogues.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

use super::translations::get_translations_json;
use super::{available_locales, Locale};
use crate::shared::state::AppState;

pub fn configure_i18n_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/i18n/locales", get(handle_get_locales))
        .route("/api/i18n/:locale", get(handle_get_translations))
}

async fn handle_get_locales(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let locales = available_locales();
    Json(serde_json::json!({
        "locales": locales,
        "default": "en"
    }))
}

async fn handle_get_translations(
    State(_state): State<Arc<AppState>>,
    Path(locale_str): Path<String>,
) -> impl IntoResponse {
    let locale = Locale::new(&locale_str).unwrap_or_default();

    let translations = get_translations_json(&locale);

    Json(serde_json::json!({
        "locale": locale.to_bcp47(),
        "translations": translations
    }))
}

