pub(crate) use crate::renderers::{get_prompts_data, get_templates_data, html_escape};
pub(crate) use crate::state::{AppState, get_work_path_or_default, get_keywords_or_default, make_mcp_loader};
pub(crate) use crate::types::{ApiResponse, BotQuery, SearchQuery, RepositoryInfo, AppInfo};
pub(crate) use axum::{ extract::{Form, Path, Query, State}, response::{Html, IntoResponse}, Json, };
pub(crate) use std::fmt::Write;
pub(crate) use std::sync::Arc;

use serde::Deserialize as SerdeDeserialize;

#[derive(SerdeDeserialize)]
pub struct SavePromptRequest {
    pub prompt_id: Option<String>,
    pub collection: Option<String>,
    pub prompt: Option<String>,
}pub async fn handle_prompts_save(
    State(state): State<Arc<AppState>>,
    Form(form): Form<SavePromptRequest>,
) -> impl IntoResponse {
    let _ = state;
    let prompt_id = form.prompt_id.unwrap_or_default();
    let collection = form.collection.unwrap_or_else(|| "default".to_string());
    let prompt = form.prompt.unwrap_or_default();

    Json(serde_json::json!({
        "ok": true,
        "prompt_id": prompt_id,
        "collection": collection,
        "prompt": prompt,
        "message": "Prompt saved to collection",
    }))
}

#[derive(SerdeDeserialize)]
pub struct InstallSkillRequest {
    pub name: Option<String>,
    pub bot_id: Option<String>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
