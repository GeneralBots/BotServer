use axum::{
    extract::{Path, Query, State},
    response::Html,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::*;
use crate::ui::SessionListQuery;
use crate::AttendantConfig;

pub async fn sessions_table(
    State(config): State<Arc<AttendantConfig>>,
    Query(query): Query<SessionListQuery>,
) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        let mut db_query = attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .into_boxed();

        if let Some(status) = query.status {
            db_query = db_query.filter(attendant_sessions::status.eq(status));
        }
        if let Some(queue_id) = query.queue_id {
            db_query = db_query.filter(attendant_sessions::queue_id.eq(queue_id));
        }
        if let Some(agent_id) = query.agent_id {
            db_query = db_query.filter(attendant_sessions::agent_id.eq(agent_id));
        }

        db_query = db_query.order(attendant_sessions::created_at.desc());

        if let Some(limit) = query.limit {
            db_query = db_query.limit(limit);
        } else {
            db_query = db_query.limit(50);
        }

        db_query
            .select((
                attendant_sessions::id,
                attendant_sessions::session_number,
                attendant_sessions::customer_name,
                attendant_sessions::customer_email,
                attendant_sessions::channel,
                attendant_sessions::status,
                attendant_sessions::priority,
                attendant_sessions::subject,
                attendant_sessions::created_at,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>, String, String, i32, Option<String>, DateTime<Utc>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(sessions) if !sessions.is_empty() => {
            let rows: String = sessions
                .iter()
                .map(|(id, number, name, email, channel, status, priority, subject, created)| {
                    let customer = name.clone().unwrap_or_else(|| email.clone().unwrap_or_else(|| "Unknown".to_string()));
                    let subj = subject.clone().unwrap_or_else(|| "No subject".to_string());
                    let status_class = match status.as_str() {
                        "waiting" => "status-waiting",
                        "active" => "status-active",
                        "ended" => "status-ended",
                        _ => "status-default",
                    };
                    let priority_badge = match priority {
                        p if *p >= 2 => r##"<span class="badge badge-high">High</span>"##,
                        p if *p == 1 => r##"<span class="badge badge-medium">Medium</span>"##,
                        _ => r##"<span class="badge badge-low">Low</span>"##,
                    };
                    let time = created.format("%Y-%m-%d %H:%M").to_string();

                    format!(
                        r##"<tr class="session-row" data-id="{}" hx-get="/api/ui/attendant/sessions/{}" hx-target="#session-detail" hx-swap="innerHTML">
<td class="session-number">{}</td>
<td class="session-customer">{}</td>
<td class="session-channel"><span class="channel-badge channel-{}">{}</span></td>
<td class="session-subject">{}</td>
<td class="session-priority">{}</td>
<td class="session-status"><span class="{}">{}</span></td>
<td class="session-time">{}</td>
</tr>"##,
                        id, id, number, customer, channel, channel, subj, priority_badge, status_class, status, time
                    )
                })
                .collect();

            Html(format!(
                r##"<table class="sessions-table">
<thead>
<tr>
<th>Session #</th>
<th>Customer</th>
<th>Channel</th>
<th>Subject</th>
<th>Priority</th>
<th>Status</th>
<th>Created</th>
</tr>
</thead>
<tbody>{}</tbody>
</table>"##,
                rows
            ))
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No sessions found</p>
</div>"##
                .to_string(),
        ),
    }
}

pub async fn session_detail(
    State(config): State<Arc<AttendantConfig>>,
    Path(id): Path<Uuid>,
) -> Html<String> {
    let pool = config.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        attendant_sessions::table
            .find(id)
            .select((
                attendant_sessions::id,
                attendant_sessions::session_number,
                attendant_sessions::customer_name,
                attendant_sessions::customer_email,
                attendant_sessions::customer_phone,
                attendant_sessions::channel,
                attendant_sessions::status,
                attendant_sessions::priority,
                attendant_sessions::subject,
                attendant_sessions::initial_message,
                attendant_sessions::notes,
                attendant_sessions::created_at,
                attendant_sessions::assigned_at,
                attendant_sessions::ended_at,
            ))
            .first::<(
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                String,
                String,
                i32,
                Option<String>,
                Option<String>,
                Option<String>,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
                Option<DateTime<Utc>>,
            )>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some((id, number, name, email, phone, channel, status, priority, subject, message, notes, created, assigned, ended)) => {
            let customer_name = name.unwrap_or_else(|| "Unknown".to_string());
            let customer_email = email.unwrap_or_else(|| "-".to_string());
            let customer_phone = phone.unwrap_or_else(|| "-".to_string());
            let subject_text = subject.unwrap_or_else(|| "No subject".to_string());
            let message_text = message.unwrap_or_else(|| "-".to_string());
            let notes_text = notes.unwrap_or_else(|| "-".to_string());
            let created_time = created.format("%Y-%m-%d %H:%M:%S").to_string();
            let assigned_time = assigned.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "-".to_string());
            let ended_time = ended.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "-".to_string());

            let priority_text = match priority {
                2.. => "High",
                1 => "Medium",
                _ => "Low",
            };

            Html(format!(
                r##"<div class="session-detail-card">
<div class="detail-header">
<h3>Session #{}</h3>
<span class="status-badge status-{}">{}</span>
</div>
<div class="detail-section">
<h4>Customer Information</h4>
<div class="detail-grid">
<div class="detail-item"><label>Name</label><span>{}</span></div>
<div class="detail-item"><label>Email</label><span>{}</span></div>
<div class="detail-item"><label>Phone</label><span>{}</span></div>
<div class="detail-item"><label>Channel</label><span class="channel-badge channel-{}">{}</span></div>
</div>
</div>
<div class="detail-section">
<h4>Session Details</h4>
<div class="detail-grid">
<div class="detail-item"><label>Subject</label><span>{}</span></div>
<div class="detail-item"><label>Priority</label><span>{}</span></div>
<div class="detail-item"><label>Created</label><span>{}</span></div>
<div class="detail-item"><label>Assigned</label><span>{}</span></div>
<div class="detail-item"><label>Ended</label><span>{}</span></div>
</div>
</div>
<div class="detail-section">
<h4>Initial Message</h4>
<p class="message-content">{}</p>
</div>
<div class="detail-section">
<h4>Notes</h4>
<p class="notes-content">{}</p>
</div>
<div class="detail-actions">
<button class="btn btn-primary" hx-put="/api/attendant/sessions/{}/assign" hx-swap="none">Assign to Me</button>
<button class="btn btn-secondary" hx-get="/api/ui/attendant/sessions/{}/messages" hx-target="#messages-panel">View Messages</button>
<button class="btn btn-danger" hx-put="/api/attendant/sessions/{}/end" hx-swap="none">End Session</button>
</div>
</div>"##,
                number, status, status,
                customer_name, customer_email, customer_phone,
                channel, channel,
                subject_text, priority_text,
                created_time, assigned_time, ended_time,
                message_text, notes_text,
                id, id, id
            ))
        }
        None => Html(
            r##"<div class="empty-state">
<p>Session not found</p>
</div>"##
                .to_string(),
        ),
    }
}

pub async fn queues_list(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_queues::table
            .filter(attendant_queues::branch_id.eq(Some(branch_id)))
            .filter(attendant_queues::is_active.eq(true))
            .order(attendant_queues::priority.desc())
            .select((
                attendant_queues::id,
                attendant_queues::name,
                attendant_queues::description,
                attendant_queues::priority,
            ))
            .load::<(Uuid, String, Option<String>, i32)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(queues) if !queues.is_empty() => {
            let items: String = queues
                .iter()
                .map(|(id, name, desc, priority)| {
                    let description = desc.clone().unwrap_or_default();
                    format!(
                        r##"<div class="queue-item" data-id="{}">
<div class="queue-header">
<span class="queue-name">{}</span>
<span class="queue-priority">Priority: {}</span>
</div>
<p class="queue-description">{}</p>
<div class="queue-stats" hx-get="/api/ui/attendant/queues/{}/stats" hx-trigger="load" hx-swap="innerHTML"></div>
</div>"##,
                        id, name, priority, description, id
                    )
                })
                .collect();

            Html(format!(r##"<div class="queues-list">{}</div>"##, items))
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No queues configured</p>
</div>"##
                .to_string(),
        ),
    }
}

pub async fn queue_stats(
    State(config): State<Arc<AttendantConfig>>,
    Path(queue_id): Path<Uuid>,
) -> Html<String> {
    let pool = config.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;

        let waiting: i64 = attendant_sessions::table
            .filter(attendant_sessions::queue_id.eq(queue_id))
            .filter(attendant_sessions::status.eq("waiting"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let active: i64 = attendant_sessions::table
            .filter(attendant_sessions::queue_id.eq(queue_id))
            .filter(attendant_sessions::status.eq("active"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        Some((waiting, active))
    })
    .await
    .ok()
    .flatten();

    match result {
        Some((waiting, active)) => Html(format!(
            r##"<span class="stat">Waiting: {}</span>
<span class="stat">Active: {}</span>"##,
            waiting, active
        )),
        None => Html(r##"<span class="stat">-</span>"##.to_string()),
    }
}

pub async fn agent_status_list(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        attendant_agent_status::table
            .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
            .order(attendant_agent_status::status.asc())
            .select((
                attendant_agent_status::id,
                attendant_agent_status::agent_id,
                attendant_agent_status::status,
                attendant_agent_status::current_sessions,
                attendant_agent_status::max_sessions,
                attendant_agent_status::last_activity_at,
            ))
            .load::<(Uuid, Uuid, String, i32, i32, DateTime<Utc>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(agents) if !agents.is_empty() => {
            let items: String = agents
                .iter()
                .map(|(id, agent_id, status, current, max, last_activity)| {
                    let status_class = match status.as_str() {
                        "online" => "status-online",
                        "busy" => "status-busy",
                        "break" => "status-break",
                        _ => "status-offline",
                    };
                    let last_time = last_activity.format("%H:%M").to_string();

                    format!(
                        r##"<div class="agent-item" data-id="{}">
<div class="agent-avatar">
<span class="status-indicator {}"></span>
</div>
<div class="agent-info">
<span class="agent-name">Agent {}</span>
<span class="agent-status">{}</span>
</div>
<div class="agent-load">
<span>{}/{} sessions</span>
<span class="last-activity">Last: {}</span>
</div>
</div>"##,
                        id, status_class, &agent_id.to_string()[..8], status, current, max, last_time
                    )
                })
                .collect();

            Html(format!(r##"<div class="agents-list">{}</div>"##, items))
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No agents found</p>
</div>"##
                .to_string(),
        ),
    }
}
