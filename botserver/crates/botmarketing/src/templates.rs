use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::marketing_templates;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = marketing_templates)]
pub struct MarketingTemplate {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub name: String,
    pub channel: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub media_url: Option<String>,
    pub ai_prompt: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub approved: Option<bool>,
    pub meta_template_id: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub channel: String,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub media_url: Option<String>,
    pub ai_prompt: Option<String>,
    pub variables: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub channel: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub media_url: Option<String>,
    pub ai_prompt: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub approved: Option<bool>,
}

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MarketingTemplate>>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (_, branch_id, _) = state.get_scope();

    let templates: Vec<MarketingTemplate> = marketing_templates::table
        .filter(marketing_templates::branch_id.eq(branch_id))
        .order(marketing_templates::created_at.desc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(templates))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<MarketingTemplate>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let template: MarketingTemplate = marketing_templates::table
        .filter(marketing_templates::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Template not found".to_string()))?;

    Ok(Json(template))
}

pub async fn create_template(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<Json<MarketingTemplate>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (org_id, default_branch_id, bot_id) = state.get_scope();
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or(default_branch_id);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let template = MarketingTemplate {
        id,
        org_id,
        bot_id,
        branch_id,
        name: req.name,
        channel: req.channel,
        subject: req.subject,
        body: req.body,
        media_url: req.media_url,
        ai_prompt: req.ai_prompt,
        variables: req.variables,
        approved: Some(false),
        meta_template_id: None,
        created_at: Some(now),
        updated_at: Some(now),
    };

    diesel::insert_into(marketing_templates::table)
        .values(&template)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(template))
}

pub async fn update_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTemplateRequest>,
) -> Result<Json<MarketingTemplate>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    if let Some(name) = req.name {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::name.eq(name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(channel) = req.channel {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::channel.eq(channel))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(subject) = req.subject {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::subject.eq(Some(subject)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(body) = req.body {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::body.eq(Some(body)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(media_url) = req.media_url {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::media_url.eq(Some(media_url)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(ai_prompt) = req.ai_prompt {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::ai_prompt.eq(Some(ai_prompt)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(variables) = req.variables {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::variables.eq(variables))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(approved) = req.approved {
        diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
            .set(marketing_templates::approved.eq(Some(approved)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    diesel::update(marketing_templates::table.filter(marketing_templates::id.eq(id)))
        .set(marketing_templates::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    get_template(State(state), Path(id)).await
}

pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(marketing_templates::table.filter(marketing_templates::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}
