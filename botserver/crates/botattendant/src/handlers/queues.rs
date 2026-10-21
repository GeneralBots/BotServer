use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::*;
use crate::schema::*;
use crate::handlers::{db_conn, get_bot_context, org_and_bot};

pub async fn create_queue(
    State(config): State<Arc<crate::AttendantConfig>>,
    Json(req): Json<CreateQueueRequest>,
) -> Result<Json<AttendantQueue>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let (org_id, bot_id) = org_and_bot(&config, branch_id);
    let queue = AttendantQueue {
        id,
        org_id,
        bot_id,
        name: req.name,
        description: req.description,
        priority: req.priority.unwrap_or(0),
        max_wait_minutes: req.max_wait_minutes.unwrap_or(30),
        auto_assign: req.auto_assign.unwrap_or(true),
        working_hours: serde_json::json!({}),
        is_active: true,
        created_at: now,
        updated_at: now,
        branch_id: Some(branch_id),
    };

    diesel::insert_into(attendant_queues::table)
        .values(&queue)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(queue))
}

pub async fn list_queues(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<Vec<AttendantQueue>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);

    let queues: Vec<AttendantQueue> = attendant_queues::table
        .filter(attendant_queues::branch_id.eq(Some(branch_id)))
        .filter(attendant_queues::is_active.eq(true))
        .order(attendant_queues::priority.desc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(queues))
}

pub async fn get_queue(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
) -> Result<Json<QueueWithStats>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let queue: AttendantQueue = attendant_queues::table
        .filter(attendant_queues::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Queue not found".to_string()))?;

    let waiting_count: i64 = attendant_sessions::table
        .filter(attendant_sessions::queue_id.eq(id))
        .filter(attendant_sessions::status.eq("waiting"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_count: i64 = attendant_sessions::table
        .filter(attendant_sessions::queue_id.eq(id))
        .filter(attendant_sessions::status.eq("active"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let agents_count: i64 = attendant_queue_agents::table
        .filter(attendant_queue_agents::queue_id.eq(id))
        .filter(attendant_queue_agents::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Ok(Json(QueueWithStats {
        queue,
        waiting_count,
        active_count,
        agents_count,
    }))
}

pub async fn delete_queue(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    diesel::delete(attendant_queues::table.filter(attendant_queues::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn add_queue_agent(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(queue_id): Path<Uuid>,
    Json(req): Json<AddQueueAgentRequest>,
) -> Result<Json<QueueAgent>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let id = Uuid::new_v4();
    let now = Utc::now();

    let agent = QueueAgent {
        id,
        queue_id,
        agent_id: req.agent_id,
        max_concurrent: req.max_concurrent.unwrap_or(3),
        priority: req.priority.unwrap_or(0),
        skills: req.skills.unwrap_or_default(),
        is_active: true,
        created_at: now,
    };

    diesel::insert_into(attendant_queue_agents::table)
        .values(&agent)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(agent))
}

pub async fn remove_queue_agent(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path((queue_id, agent_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    diesel::delete(
        attendant_queue_agents::table
            .filter(attendant_queue_agents::queue_id.eq(queue_id))
            .filter(attendant_queue_agents::agent_id.eq(agent_id)),
    )
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_canned_responses(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<Vec<CannedResponse>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);

    let responses: Vec<CannedResponse> = attendant_canned_responses::table
        .filter(attendant_canned_responses::branch_id.eq(Some(branch_id)))
        .filter(attendant_canned_responses::is_active.eq(true))
        .order(attendant_canned_responses::title.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(responses))
}

pub async fn create_canned_response(
    State(config): State<Arc<crate::AttendantConfig>>,
    Json(req): Json<CreateCannedResponseRequest>,
) -> Result<Json<CannedResponse>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let (org_id, bot_id) = org_and_bot(&config, branch_id);
    let response = CannedResponse {
        id,
        org_id,
        bot_id,
        title: req.title,
        content: req.content,
        shortcut: req.shortcut,
        category: req.category,
        queue_id: req.queue_id,
        is_active: true,
        usage_count: 0,
        created_by: None,
        created_at: now,
        updated_at: now,
        branch_id: Some(branch_id),
    };

    diesel::insert_into(attendant_canned_responses::table)
        .values(&response)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(response))
}

pub async fn list_tags(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<Vec<AttendantTag>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);

    let tags: Vec<AttendantTag> = attendant_tags::table
        .filter(attendant_tags::branch_id.eq(Some(branch_id)))
        .filter(attendant_tags::is_active.eq(true))
        .order(attendant_tags::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(tags))
}

pub async fn list_wrap_up_codes(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<Vec<WrapUpCode>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);

    let codes: Vec<WrapUpCode> = attendant_wrap_up_codes::table
        .filter(attendant_wrap_up_codes::branch_id.eq(Some(branch_id)))
        .filter(attendant_wrap_up_codes::is_active.eq(true))
        .order(attendant_wrap_up_codes::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(codes))
}
