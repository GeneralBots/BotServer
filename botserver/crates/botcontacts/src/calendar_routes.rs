use axum::{
    http::HeaderMap,

    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use super::calendar_types::*;
use super::calendar_service::CalendarIntegrationService;

impl axum::response::IntoResponse for CalendarIntegrationError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let status = match &self {
            CalendarIntegrationError::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            CalendarIntegrationError::ContactNotFound | CalendarIntegrationError::EventNotFound => StatusCode::NOT_FOUND,
            CalendarIntegrationError::AlreadyLinked | CalendarIntegrationError::NotLinked => StatusCode::CONFLICT,
            CalendarIntegrationError::Unauthorized => StatusCode::UNAUTHORIZED,
            CalendarIntegrationError::InvalidInput(_) => StatusCode::BAD_REQUEST,
        };
        (status, Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&self, "botcontacts::calendar_routes", None) }))).into_response()
    }
}

pub fn calendar_integration_routes() -> Router<Arc<crate::CrateState>> {
    Router::new()
        .route("/events/:event_id/contacts", get(get_event_contacts_handler).post(link_contact_handler))
        .route("/events/:event_id/contacts/bulk", post(bulk_link_contacts_handler))
        .route("/events/:event_id/contacts/:contact_id", delete(unlink_contact_handler).post(update_event_contact_handler))
        .route("/events/:event_id/contacts/suggestions", get(get_suggestions_handler))
        .route("/contacts/:contact_id/events", get(get_contact_events_handler))
        .route("/events/:event_id/find-contacts", post(find_contacts_handler))
        .route("/events/:event_id/create-contacts", post(create_contacts_from_attendees_handler))
}

async fn get_event_contacts_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Query(query): Query<EventContactsQuery>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.get_event_contacts(branch_id, event_id, &query).await {
        Ok(contacts) => Json(contacts).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn link_contact_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(request): Json<LinkContactRequest>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.link_contact_to_event(branch_id, event_id, &request).await {
        Ok(ec) => Json(ec).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn bulk_link_contacts_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(request): Json<BulkLinkContactsRequest>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.bulk_link_contacts(branch_id, event_id, &request).await {
        Ok(ecs) => Json(ecs).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn unlink_contact_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path((event_id, contact_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.unlink_contact_from_event(branch_id, event_id, contact_id).await {
        Ok(()) => Json(serde_json::json!({"success": true})).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn update_event_contact_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path((event_id, contact_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateEventContactRequest>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.update_event_contact(branch_id, event_id, contact_id, &request).await {
        Ok(ec) => Json(ec).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_suggestions_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.get_suggested_contacts(branch_id, event_id, None).await {
        Ok(suggestions) => Json(suggestions).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_contact_events_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(contact_id): Path<Uuid>,
    Query(query): Query<ContactEventsQuery>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    match service.get_contact_events(branch_id, contact_id, &query).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn find_contacts_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(emails): Json<Vec<String>>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    let _ = event_id;
    match service.find_contacts_for_event(branch_id, &emails).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_contacts_from_attendees_handler(
    State(state): State<Arc<crate::CrateState>>,
    headers: HeaderMap,
    Path(event_id): Path<Uuid>,
    Json(attendees): Json<Vec<AttendeeInfo>>,
) -> impl IntoResponse {
    let service = CalendarIntegrationService::new(Arc::new(state.db_pool.clone()));
    let branch_id = crate::scope::branch_from_jwt_pool(&headers, &state.db_pool).unwrap_or_else(|| state.get_bot_context());
    let _ = event_id;
    match service.create_contacts_from_attendees(branch_id, &attendees).await {
        Ok(created) => Json(created).into_response(),
        Err(e) => e.into_response(),
    }
}
