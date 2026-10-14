use crate::collaboration::broadcast_slide_change;
use crate::storage::{
    create_slide_with_layout, get_current_user_id, load_presentation_by_id,
    save_presentation_to_drive, DriveOps,
};
use crate::types::{
    AddElementRequest, AddSlideRequest, ApplyThemeRequest, ApplyTransitionToAllRequest,
    DeleteElementRequest, DeleteSlideRequest, DuplicateSlideRequest, ReorderSlidesRequest,
    RemoveTransitionRequest, SaveResponse, SetTransitionRequest, UpdateElementRequest,
    UpdateSlideNotesRequest,
};
use crate::SlidesState;
use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

macro_rules! load_pres {
    ($state:expr, $user_id:expr, $pres_id:expr) => {
        match $state.drive.as_ref() {
            Some(drive) => match load_presentation_by_id(drive, $user_id, $pres_id).await {
                Ok(p) => p,
                Err(e) => {
                    return Err((
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({ "error": e })),
                    ))
                }
            },
            None => {
                return Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({ "error": "Drive not configured" })),
                ))
            }
        }
    };
}

macro_rules! save_pres {
    ($state:expr, $user_id:expr, $presentation:expr) => {
        if let Some(drive) = &$state.drive {
            if let Err(e) = save_presentation_to_drive(drive, $user_id, $presentation).await {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botslides::handlers::slides", None)})),
                ));
            }
        }
    };
}

pub async fn handle_add_slide<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<AddSlideRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    let new_slide = create_slide_with_layout(&req.layout, &presentation.theme);

    if let Some(position) = req.position {
        if position <= presentation.slides.len() {
            presentation.slides.insert(position, new_slide);
        } else {
            presentation.slides.push(new_slide);
        }
    } else {
        presentation.slides.push(new_slide);
    }

    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Slide added".to_string()),
    }))
}

pub async fn handle_delete_slide<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<DeleteSlideRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides.remove(req.slide_index);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Slide deleted".to_string()),
    }))
}

pub async fn handle_duplicate_slide<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<DuplicateSlideRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    let mut duplicated = presentation.slides[req.slide_index].clone();
    duplicated.id = Uuid::new_v4().to_string();
    presentation.slides.insert(req.slide_index + 1, duplicated);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Slide duplicated".to_string()),
    }))
}

pub async fn handle_reorder_slides<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<ReorderSlidesRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_order.len() != presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide order" })),
        ));
    }

    let old_slides = presentation.slides.clone();
    presentation.slides = req
        .slide_order
        .iter()
        .filter_map(|&idx| old_slides.get(idx).cloned())
        .collect();

    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Slides reordered".to_string()),
    }))
}

pub async fn handle_update_slide_notes<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<UpdateSlideNotesRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides[req.slide_index].notes = Some(req.notes);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Slide notes updated".to_string()),
    }))
}

pub async fn handle_add_element<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<AddElementRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides[req.slide_index].elements.push(req.element);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    broadcast_slide_change(
        &req.presentation_id,
        &user_id,
        "User",
        "element_added",
        Some(req.slide_index),
        None,
        None,
    )
    .await;

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Element added".to_string()),
    }))
}

pub async fn handle_update_element<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<UpdateElementRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    let slide = &mut presentation.slides[req.slide_index];
    if let Some(pos) = slide.elements.iter().position(|e| e.id == req.element.id) {
        slide.elements[pos] = req.element.clone();
    } else {
        slide.elements.push(req.element.clone());
    }

    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    broadcast_slide_change(
        &req.presentation_id,
        &user_id,
        "User",
        "element_updated",
        Some(req.slide_index),
        Some(&req.element.id),
        None,
    )
    .await;

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Element updated".to_string()),
    }))
}

pub async fn handle_delete_element<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<DeleteElementRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides[req.slide_index]
        .elements
        .retain(|e| e.id != req.element_id);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Element deleted".to_string()),
    }))
}

pub async fn handle_apply_theme<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<ApplyThemeRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    presentation.theme = req.theme;
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Theme applied".to_string()),
    }))
}

pub async fn handle_set_transition<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<SetTransitionRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides[req.slide_index].transition_config = Some(req.transition);
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Transition set".to_string()),
    }))
}

pub async fn handle_apply_transition_to_all<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<ApplyTransitionToAllRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    for slide in presentation.slides.iter_mut() {
        slide.transition_config = Some(req.transition.clone());
    }
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Transition applied to all slides".to_string()),
    }))
}

pub async fn handle_remove_transition<D: DriveOps>(
    State(state): State<Arc<SlidesState<D>>>,
    Json(req): Json<RemoveTransitionRequest>,
) -> Result<Json<SaveResponse>, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_current_user_id();
    let mut presentation = load_pres!(state, &user_id, &req.presentation_id);

    if req.slide_index >= presentation.slides.len() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Invalid slide index" })),
        ));
    }

    presentation.slides[req.slide_index].transition_config = None;
    presentation.slides[req.slide_index].transition = None;
    presentation.updated_at = Utc::now();
    save_pres!(state, &user_id, &presentation);

    Ok(Json(SaveResponse {
        id: req.presentation_id,
        success: true,
        message: Some("Transition removed".to_string()),
    }))
}
