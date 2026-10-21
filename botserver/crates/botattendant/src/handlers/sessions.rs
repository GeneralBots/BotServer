use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::schema::*;
use crate::handlers::{db_conn, generate_session_number, get_bot_context, org_and_bot};

pub async fn create_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<AttendantSession>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let session_number = generate_session_number(&mut conn, branch_id);

    let (org_id, bot_id) = org_and_bot(&config, branch_id);
    let session = AttendantSession {
        id,
        org_id,
        bot_id,
        session_number,
        channel: req.channel,
        customer_id: req.customer_id,
        customer_name: req.customer_name,
        customer_email: req.customer_email,
        customer_phone: req.customer_phone,
        status: "waiting".to_string(),
        priority: req.priority.unwrap_or(0),
        agent_id: None,
        queue_id: req.queue_id,
        subject: req.subject,
        initial_message: req.initial_message,
        started_at: now,
        assigned_at: None,
        first_response_at: None,
        ended_at: None,
        wait_time_seconds: None,
        handle_time_seconds: None,
        satisfaction_rating: None,
        satisfaction_comment: None,
        tags: vec![],
        metadata: serde_json::json!({}),
        notes: None,
        transfer_count: 0,
        created_at: now,
        branch_id: Some(branch_id),
    };

    diesel::insert_into(attendant_sessions::table)
        .values(&session)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(session))
}

pub async fn list_sessions(
    State(config): State<Arc<crate::AttendantConfig>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<AttendantSession>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = attendant_sessions::table
        .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
        .into_boxed();

    if let Some(status) = query.status {
        if status != "all" {
            q = q.filter(attendant_sessions::status.eq(status));
        }
    }

    if let Some(queue_id) = query.queue_id {
        q = q.filter(attendant_sessions::queue_id.eq(queue_id));
    }

    if let Some(agent_id) = query.agent_id {
        q = q.filter(attendant_sessions::agent_id.eq(agent_id));
    }

    if let Some(channel) = query.channel {
        q = q.filter(attendant_sessions::channel.eq(channel));
    }

    let sessions: Vec<AttendantSession> = q
        .order(attendant_sessions::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(sessions))
}

pub async fn get_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SessionWithMessages>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let messages: Vec<SessionMessage> = attendant_session_messages::table
        .filter(attendant_session_messages::session_id.eq(id))
        .order(attendant_session_messages::created_at.asc())
        .load(&mut conn)
        .unwrap_or_default();

    Ok(Json(SessionWithMessages { session, messages }))
}

pub async fn assign_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignSessionRequest>,
) -> Result<Json<AttendantSession>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let now = Utc::now();

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let wait_time = (now - session.started_at).num_seconds() as i32;

    diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(id)))
        .set((
            attendant_sessions::agent_id.eq(Some(req.agent_id)),
            attendant_sessions::status.eq("active"),
            attendant_sessions::assigned_at.eq(Some(now)),
            attendant_sessions::wait_time_seconds.eq(Some(wait_time)),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let updated: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    Ok(Json(updated))
}

pub async fn transfer_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransferSessionRequest>,
) -> Result<Json<AttendantSession>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let now = Utc::now();

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let transfer = SessionTransfer {
        id: Uuid::new_v4(),
        session_id: id,
        from_agent_id: session.agent_id,
        to_agent_id: req.to_agent_id,
        to_queue_id: req.to_queue_id,
        reason: req.reason,
        notes: req.notes,
        created_at: now,
    };

    diesel::insert_into(attendant_transfers::table)
        .values(&transfer)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    if let Some(to_agent_id) = req.to_agent_id {
        diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(id)))
            .set((
                attendant_sessions::agent_id.eq(Some(to_agent_id)),
                attendant_sessions::transfer_count.eq(session.transfer_count + 1),
            ))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    } else if let Some(to_queue_id) = req.to_queue_id {
        diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(id)))
            .set((
                attendant_sessions::agent_id.eq(None::<Uuid>),
                attendant_sessions::queue_id.eq(Some(to_queue_id)),
                attendant_sessions::status.eq("waiting"),
                attendant_sessions::transfer_count.eq(session.transfer_count + 1),
            ))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    let updated: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    Ok(Json(updated))
}

pub async fn end_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
    Json(_req): Json<EndSessionRequest>,
) -> Result<Json<AttendantSession>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let now = Utc::now();

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    let handle_time = session.assigned_at.map(|assigned| {
        (now - assigned).num_seconds() as i32
    });

    diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(id)))
        .set((
            attendant_sessions::status.eq("ended"),
            attendant_sessions::ended_at.eq(Some(now)),
            attendant_sessions::handle_time_seconds.eq(handle_time),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let updated: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    Ok(Json(updated))
}

pub async fn rate_session(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
    Json(req): Json<RateSessionRequest>,
) -> Result<Json<AttendantSession>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(id)))
        .set((
            attendant_sessions::satisfaction_rating.eq(Some(req.rating)),
            attendant_sessions::satisfaction_comment.eq(req.comment),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    Ok(Json(session))
}

pub async fn send_message(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(session_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<SessionMessage>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let id = Uuid::new_v4();
    let now = Utc::now();

    let message = SessionMessage {
        id,
        session_id,
        sender_type: "agent".to_string(),
        sender_id: None,
        sender_name: req.sender_name,
        content: req.content,
        content_type: req.content_type.unwrap_or_else(|| "text".to_string()),
        attachments: serde_json::to_value(req.attachments.clone().unwrap_or_default())
            .unwrap_or_else(|_| serde_json::json!([])),
        is_internal: req.is_internal.unwrap_or(false),
        created_at: now,
    };

    diesel::insert_into(attendant_session_messages::table)
        .values(&message)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    let session: AttendantSession = attendant_sessions::table
        .filter(attendant_sessions::id.eq(session_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Session not found".to_string()))?;

    if session.first_response_at.is_none() && !message.is_internal {
        diesel::update(attendant_sessions::table.filter(attendant_sessions::id.eq(session_id)))
            .set(attendant_sessions::first_response_at.eq(Some(now)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    Ok(Json(message))
}
