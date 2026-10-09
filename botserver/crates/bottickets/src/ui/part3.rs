use super::*;

pub(super) async fn handle_stats_by_status(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("<p>Unable to load stats</p>".to_string());
    };

    let branch_id = get_bot_context(&state);

    let open: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let pending: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("pending"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let resolved: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("resolved"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let closed: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let html = format!(
        "<div class=\"stats-grid\">\
        <div class=\"stat-card stat-open\">\
        <div class=\"stat-value\">{}</div>\
        <div class=\"stat-label\">Open</div>\
        </div>\
        <div class=\"stat-card stat-pending\">\
        <div class=\"stat-value\">{}</div>\
        <div class=\"stat-label\">Pending</div>\
        </div>\
        <div class=\"stat-card stat-resolved\">\
        <div class=\"stat-value\">{}</div>\
        <div class=\"stat-label\">Resolved</div>\
        </div>\
        <div class=\"stat-card stat-closed\">\
        <div class=\"stat-value\">{}</div>\
        <div class=\"stat-label\">Closed</div>\
        </div>\
        </div>",
        open, pending, resolved, closed
    );

    Html(html)
}
pub(super) async fn handle_stats_by_priority(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("<p>Unable to load stats</p>".to_string());
    };

    let branch_id = get_bot_context(&state);

    let urgent: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::priority.eq("urgent"))
        .filter(support_tickets::status.ne("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let high: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::priority.eq("high"))
        .filter(support_tickets::status.ne("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let medium: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::priority.eq("medium"))
        .filter(support_tickets::status.ne("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let low: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::priority.eq("low"))
        .filter(support_tickets::status.ne("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let html = format!(
        "<div class=\"priority-stats\">\
        <div class=\"priority-bar\">\
        <div class=\"priority-segment urgent\" style=\"flex: {};\" title=\"Urgent: {}\"></div>\
        <div class=\"priority-segment high\" style=\"flex: {};\" title=\"High: {}\"></div>\
        <div class=\"priority-segment medium\" style=\"flex: {};\" title=\"Medium: {}\"></div>\
        <div class=\"priority-segment low\" style=\"flex: {};\" title=\"Low: {}\"></div>\
        </div>\
        <div class=\"priority-legend\">\
        <span class=\"legend-item\"><span class=\"dot urgent\"></span>Urgent ({})</span>\
        <span class=\"legend-item\"><span class=\"dot high\"></span>High ({})</span>\
        <span class=\"legend-item\"><span class=\"dot medium\"></span>Medium ({})</span>\
        <span class=\"legend-item\"><span class=\"dot low\"></span>Low ({})</span>\
        </div>\
        </div>",
        urgent, urgent, high, high, medium, medium, low, low,
        urgent, high, medium, low
    );

    Html(html)
}
pub(super) async fn handle_avg_resolution(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("-".to_string());
    };

    let branch_id = get_bot_context(&state);

    let resolved_tickets: Vec<SupportTicket> = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::resolved_at.is_not_null())
        .limit(100)
        .load(&mut conn)
        .unwrap_or_default();

    if resolved_tickets.is_empty() {
        return Html("-".to_string());
    }

    let total_hours: f64 = resolved_tickets
        .iter()
        .filter_map(|t| {
            t.resolved_at.map(|resolved| {
                let duration = resolved - t.created_at;
                duration.num_hours() as f64
            })
        })
        .sum();

    let avg_hours = total_hours / resolved_tickets.len() as f64;

    if avg_hours < 24.0 {
        Html(format!("{:.1}h", avg_hours))
    } else {
        Html(format!("{:.1}d", avg_hours / 24.0))
    }
}
pub(super) async fn handle_stats_open(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };
    let branch_id = get_bot_context(&state);

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.ne("closed"))
        .filter(support_tickets::status.ne("resolved"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
pub(super) async fn handle_stats_urgent(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };
    let branch_id = get_bot_context(&state);

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.ne("closed"))
        .filter(support_tickets::status.ne("resolved"))
        .filter(
            support_tickets::priority
                .eq("urgent")
                .or(support_tickets::priority.eq("critical"))
                .or(support_tickets::priority.eq("high")),
        )
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
pub(super) async fn handle_stats_resolved_today(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };
    let branch_id = get_bot_context(&state);
    let today = ChronoUtc::now().date_naive();
    let start_of_day = today.and_time(chrono::NaiveTime::MIN);

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::resolved_at.is_not_null())
        .filter(support_tickets::resolved_at.ge(start_of_day))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
pub(super) async fn handle_stats_ai_resolved(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };
    let branch_id = get_bot_context(&state);

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::resolved_at.is_not_null())
        .filter(support_tickets::source.eq("ai"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
