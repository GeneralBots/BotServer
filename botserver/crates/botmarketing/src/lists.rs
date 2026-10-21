use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::marketing_lists;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = marketing_lists)]
pub struct MarketingList {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub bot_id: Option<Uuid>,
    pub name: String,
    pub list_type: String,
    pub query_text: Option<String>,
    pub contact_count: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
    pub description: Option<String>,
    pub member_count: Option<i32>,
    pub is_dynamic: Option<bool>,
    pub criteria: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateListRequest {
    pub name: String,
    pub list_type: String,
    pub query_text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateListRequest {
    pub name: Option<String>,
    pub list_type: Option<String>,
    pub query_text: Option<String>,
}

pub async fn list_lists(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MarketingList>>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (_, branch_id, _) = state.get_scope();

    let lists: Vec<MarketingList> = marketing_lists::table
        .filter(marketing_lists::branch_id.eq(branch_id))
        .order(marketing_lists::created_at.desc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(lists))
}

pub async fn get_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<MarketingList>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let list: MarketingList = marketing_lists::table
        .filter(marketing_lists::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "List not found".to_string()))?;

    Ok(Json(list))
}

pub async fn create_list(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateListRequest>,
) -> Result<Json<MarketingList>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (_, branch_id, _) = state.get_scope();
    let id = Uuid::new_v4();
    let now = Utc::now();

    let list = MarketingList {
        id,
        branch_id,
        name: req.name,
        list_type: req.list_type,
        query_text: req.query_text,
        member_count: None,
        is_dynamic: None,
        criteria: None,
        description: None,
        contact_count: Some(0),
        created_at: Some(now),
        updated_at: Some(now),
        org_id: None,
        bot_id: None,
    };

    diesel::insert_into(marketing_lists::table)
        .values(&list)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(list))
}

pub async fn update_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateListRequest>,
) -> Result<Json<MarketingList>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    if let Some(name) = req.name {
        diesel::update(marketing_lists::table.filter(marketing_lists::id.eq(id)))
            .set(marketing_lists::name.eq(name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(list_type) = req.list_type {
        diesel::update(marketing_lists::table.filter(marketing_lists::id.eq(id)))
            .set(marketing_lists::name.eq(list_type))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(query_text) = req.query_text {
        diesel::update(marketing_lists::table.filter(marketing_lists::id.eq(id)))
            .set(marketing_lists::query_text.eq(Some(query_text)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    diesel::update(marketing_lists::table.filter(marketing_lists::id.eq(id)))
        .set(marketing_lists::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    get_list(State(state), Path(id)).await
}

pub async fn delete_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(marketing_lists::table.filter(marketing_lists::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

pub async fn refresh_marketing_list(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    use crate::schema::crm_contacts;

    let list: MarketingList = marketing_lists::table
        .filter(marketing_lists::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "List not found".to_string()))?;

    let (_, branch_id, _) = state.get_scope();

    let query_text = list.query_text.as_deref().unwrap_or("");
    let list_type = list.list_type.as_str();

    let contact_count: i64 = if list_type == "dynamic" && !query_text.is_empty() {
        let query_lower = query_text.to_lowercase();

        if query_lower.contains("status=") {
            let status = query_lower
                .split("status=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("active");

            crm_contacts::table
                .filter(crm_contacts::branch_id.eq(branch_id))
                .filter(crm_contacts::status.eq(status))
                .count()
                .get_result(&mut conn)
                .unwrap_or(0)
        } else if query_lower.contains("company=") {
            let company = query_lower
                .split("company=")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or("");

            if !company.is_empty() {
                crm_contacts::table
                    .filter(crm_contacts::branch_id.eq(branch_id))
                    .filter(crm_contacts::company.ilike(format!("%{company}%")))
                    .count()
                    .get_result(&mut conn)
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            let pattern = format!("%{query_text}%");
            crm_contacts::table
                .filter(crm_contacts::branch_id.eq(branch_id))
                .filter(
                    crm_contacts::first_name.ilike(pattern.clone())
                        .or(crm_contacts::last_name.ilike(pattern.clone()))
                        .or(crm_contacts::email.ilike(pattern.clone()))
                        .or(crm_contacts::company.ilike(pattern)),
                )
                .count()
                .get_result(&mut conn)
                .unwrap_or(0)
        }
    } else {
        crm_contacts::table
            .filter(crm_contacts::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0)
    };

    diesel::update(marketing_lists::table.filter(marketing_lists::id.eq(id)))
        .set((
            marketing_lists::contact_count.eq(Some(contact_count as i32)),
            marketing_lists::updated_at.eq(Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    Ok(Json(serde_json::json!({
        "status": "refreshed",
        "list_id": id,
        "contact_count": contact_count
    })))
}
