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

pub async fn update_agent_status(
    State(config): State<Arc<crate::AttendantConfig>>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<UpdateAgentStatusRequest>,
) -> Result<Json<AgentStatus>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let now = Utc::now();

    let existing: Option<AgentStatus> = attendant_agent_status::table
        .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
        .filter(attendant_agent_status::agent_id.eq(agent_id))
        .first(&mut conn)
        .ok();

    let break_started = if req.status == "break" {
        Some(now)
    } else {
        None
    };

    let available_since = if req.status == "online" {
        Some(now)
    } else {
        None
    };

    if existing.is_some() {
        diesel::update(
            attendant_agent_status::table
                .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
                .filter(attendant_agent_status::agent_id.eq(agent_id)),
        )
        .set((
            attendant_agent_status::status.eq(&req.status),
            attendant_agent_status::status_message.eq(&req.status_message),
            attendant_agent_status::break_started_at.eq(break_started),
            attendant_agent_status::break_reason.eq(&req.break_reason),
            attendant_agent_status::available_since.eq(available_since),
            attendant_agent_status::last_activity_at.eq(now),
            attendant_agent_status::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    } else {
        let (org_id, bot_id) = org_and_bot(&config, branch_id);
        let status = AgentStatus {
            id: Uuid::new_v4(),
            org_id,
            bot_id,
            agent_id,
            status: req.status.clone(),
            status_message: req.status_message.clone(),
            current_sessions: 0,
            max_sessions: 5,
            last_activity_at: now,
            break_started_at: break_started,
            break_reason: req.break_reason.clone(),
            available_since,
            created_at: now,
            updated_at: now,
            branch_id: Some(branch_id),
        };

        diesel::insert_into(attendant_agent_status::table)
            .values(&status)
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;
    }

    let agent_status: AgentStatus = attendant_agent_status::table
        .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
        .filter(attendant_agent_status::agent_id.eq(agent_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Agent status not found".to_string()))?;

    Ok(Json(agent_status))
}

pub async fn list_agent_statuses(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<Vec<AgentStatus>>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);

    let statuses: Vec<AgentStatus> = attendant_agent_status::table
        .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
        .order(attendant_agent_status::status.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(statuses))
}

pub async fn get_attendant_stats(
    State(config): State<Arc<crate::AttendantConfig>>,
) -> Result<Json<AttendantStats>, (StatusCode, String)> {
    let mut conn = db_conn!(config);

    let branch_id = get_bot_context(&config);
    let epoch = chrono::DateTime::<chrono::Utc>::UNIX_EPOCH.naive_utc();
    let today = Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap_or(epoch);
    let today_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(today, chrono::Utc);

    let total_sessions_today: i64 = attendant_sessions::table
        .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
        .filter(attendant_sessions::created_at.ge(today_utc))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_sessions: i64 = attendant_sessions::table
        .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
        .filter(attendant_sessions::status.eq("active"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let waiting_sessions: i64 = attendant_sessions::table
        .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
        .filter(attendant_sessions::status.eq("waiting"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let agents_online: i64 = attendant_agent_status::table
        .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
        .filter(attendant_agent_status::status.eq("online"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let agents_on_break: i64 = attendant_agent_status::table
        .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
        .filter(attendant_agent_status::status.eq("break"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let stats = AttendantStats {
        total_sessions_today,
        active_sessions,
        waiting_sessions,
        avg_wait_time_seconds: 0,
        avg_handle_time_seconds: 0,
        agents_online,
        agents_on_break,
        satisfaction_avg: 0.0,
    };

    Ok(Json(stats))
}
