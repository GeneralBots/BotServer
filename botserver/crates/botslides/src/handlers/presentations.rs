use crate::storage::{
    create_new_presentation, delete_presentation_from_drive, get_current_user_id,
    list_presentations_from_drive, load_presentation_by_id, load_presentation_from_drive,
    save_presentation_to_drive, DriveOps,
};
use crate::types::{
    LoadQuery, Presentation, PresentationMetadata, SavePresentationRequest, SaveResponse,
    SearchQuery,
};
use crate::SlidesState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub async fn handle_new_presentation<D: DriveOps>(
    State(_state): State<Arc<SlidesState<D>>>,
) -> Result<Json<Presentation>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(create_new_presentation()))
}

pub async fn handle_list_presentations<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
) -> Result<Json<Vec<PresentationMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let presentations = match &state.drive {
        Some(drive) => match list_presentations_from_drive(drive, &user_id).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to list presentations: {e}");
                Vec::new()
            }
        },
        None => Vec::new(),
    };

    Ok(Json(presentations))
}

pub async fn handle_search_presentations<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<PresentationMetadata>>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

let presentations = match &state.drive {
    Some(drive) => list_presentations_from_drive(drive, &user_id)
        .await
        .unwrap_or_default(),
    None => Vec::new(),
};

    let filtered = if let Some(q) = query.q {
        let q_lower = q.to_lowercase();
        presentations
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&q_lower))
            .collect()
    } else {
        presentations
    };

    Ok(Json(filtered))
}

pub async fn handle_load_presentation<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Query(query): Query<LoadQuery>,
) -> Result<Json<Presentation>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    match load_presentation_from_drive(drive, &user_id, &query.id).await {
        Ok(presentation) => Ok(Json(presentation)),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e })),
        )),
    }
}

pub async fn handle_save_presentation<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<SavePresentationRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let presentation_id = req.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let presentation = Presentation {
        id: presentation_id.clone(),
        name: req.name,
        owner_id: user_id.clone(),
        slides: req.slides,
        theme: req.theme,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    if let Some(drive) = &state.drive {
        if let Err(e) = save_presentation_to_drive(drive, &user_id, &presentation).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::presentations", None)})),
            ));
        }
    }

    Ok(Json(SaveResponse {
        id: presentation_id,
        success: true,
        message: Some("Presentation saved successfully".to_string()),
    }))
}

pub async fn handle_delete_presentation<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<LoadQuery>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    if let Some(drive) = &state.drive {
        if let Err(e) = delete_presentation_from_drive(drive, &user_id, &req.id).await {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::presentations", None)})),
            ));
        }
    }

    Ok(Json(SaveResponse {
        id: req.id.unwrap_or_default(),
        success: true,
        message: Some("Presentation deleted".to_string()),
    }))
}

pub async fn handle_get_presentation_by_id<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Path(presentation_id): Path<String>,
) -> Result<Json<Presentation>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();

    let drive = state.drive.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": "Drive not configured" })),
        )
    })?;

    match load_presentation_by_id(drive, &user_id, &presentation_id).await {
        Ok(presentation) => Ok(Json(presentation)),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": e })),
        )),
    }
}
