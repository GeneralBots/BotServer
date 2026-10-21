use axum::{extract::State, response::Html};
use chrono::Utc;
use diesel::prelude::*;
use std::sync::Arc;

use crate::schema::*;
use crate::AttendantConfig;

pub async fn dashboard_stats(State(config): State<Arc<AttendantConfig>>) -> Html<String> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot;

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        let today = Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0)?;

        let total_today: i64 = attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .filter(attendant_sessions::created_at.ge(today_start))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let waiting: i64 = attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .filter(attendant_sessions::status.eq("waiting"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let active: i64 = attendant_sessions::table
            .filter(attendant_sessions::branch_id.eq(Some(branch_id)))
            .filter(attendant_sessions::status.eq("active"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let agents_online: i64 = attendant_agent_status::table
            .filter(attendant_agent_status::branch_id.eq(Some(branch_id)))
            .filter(attendant_agent_status::status.eq("online"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        Some((total_today, waiting, active, agents_online))
    })
    .await
    .ok()
    .flatten();

    match result {
        Some((total, waiting, active, agents)) => Html(format!(
            r##"<div class="dashboard-stats">
<div class="stat-card">
<span class="stat-value">{}</span>
<span class="stat-label">Sessions Today</span>
</div>
<div class="stat-card stat-warning">
<span class="stat-value">{}</span>
<span class="stat-label">Waiting</span>
</div>
<div class="stat-card stat-success">
<span class="stat-value">{}</span>
<span class="stat-label">Active</span>
</div>
<div class="stat-card stat-info">
<span class="stat-value">{}</span>
<span class="stat-label">Agents Online</span>
</div>
</div>"##,
            total, waiting, active, agents
        )),
        None => Html(
            r##"<div class="dashboard-stats">
<div class="stat-card"><span class="stat-value">-</span></div>
</div>"##
                .to_string(),
        ),
    }
}
