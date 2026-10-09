use super::*;

pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
pub(super) fn priority_badge(priority: &str) -> &'static str {
    match priority {
        "urgent" => "<span class=\"badge badge-danger\">Urgent</span>",
        "high" => "<span class=\"badge badge-warning\">High</span>",
        "medium" => "<span class=\"badge badge-info\">Medium</span>",
        "low" => "<span class=\"badge badge-secondary\">Low</span>",
        _ => "<span class=\"badge\">Unknown</span>",
    }
}
pub(super) fn status_badge(status: &str) -> &'static str {
    match status {
        "open" => "<span class=\"badge badge-primary\">Open</span>",
        "pending" => "<span class=\"badge badge-warning\">Pending</span>",
        "resolved" => "<span class=\"badge badge-success\">Resolved</span>",
        "closed" => "<span class=\"badge badge-secondary\">Closed</span>",
        _ => "<span class=\"badge\">Unknown</span>",
    }
}
pub(super) fn render_empty_state(icon: &str, title: &str, description: &str) -> String {
    format!(
        "<div class=\"empty-state\">\
        <div class=\"empty-icon\">{}</div>\
        <h3>{}</h3>\
        <p>{}</p>\
        </div>",
        icon, title, description
    )
}
pub(super) fn render_ticket_row(ticket: &SupportTicket) -> String {
    let requester = ticket
        .requester_name
        .as_deref()
        .or(ticket.requester_email.as_deref())
        .unwrap_or("Unknown");

    let assignee = ticket
        .assignee_id
        .map(|_| "Assigned")
        .unwrap_or("Unassigned");

    let created = ticket.created_at.format("%Y-%m-%d %H:%M").to_string();
    let hash = "#";

    format!(
        "<tr class=\"ticket-row\" data-id=\"{id}\">\
        <td class=\"ticket-number\">{number}</td>\
        <td class=\"ticket-subject\">\
        <a href=\"{hash}\" hx-get=\"/api/ui/tickets/{id}\" hx-target=\"{hash}ticket-detail\" hx-swap=\"innerHTML\">{subject}</a>\
        </td>\
        <td class=\"ticket-requester\">{requester}</td>\
        <td class=\"ticket-status\">{status}</td>\
        <td class=\"ticket-priority\">{priority}</td>\
        <td class=\"ticket-assignee\">{assignee}</td>\
        <td class=\"ticket-created\">{created}</td>\
        <td class=\"ticket-actions\">\
        <button class=\"btn-icon\" hx-put=\"/api/tickets/{id}/resolve\" hx-swap=\"none\" title=\"Resolve\">✓</button>\
        <button class=\"btn-icon\" hx-delete=\"/api/tickets/{id}\" hx-confirm=\"Delete this ticket?\" hx-swap=\"none\" title=\"Delete\">×</button>\
        </td>\
        </tr>",
        id = ticket.id,
        hash = hash,
        number = html_escape(&ticket.ticket_number),
        subject = html_escape(&ticket.subject),
        requester = html_escape(requester),
        status = status_badge(&ticket.status),
        priority = priority_badge(&ticket.priority),
        assignee = html_escape(assignee),
        created = created,
    )
}
pub(super) fn render_ticket_card(ticket: &SupportTicket) -> String {
    let requester = ticket
        .requester_name
        .as_deref()
        .or(ticket.requester_email.as_deref())
        .unwrap_or("Unknown");

    let hash = "#";

    format!(
        "<div class=\"ticket-card\" data-id=\"{id}\">\
        <div class=\"ticket-card-header\">\
        <span class=\"ticket-number\">{number}</span>\
        {status}\
        {priority}\
        </div>\
        <div class=\"ticket-card-body\">\
        <h4 class=\"ticket-subject\">{subject}</h4>\
        <p class=\"ticket-requester\">From: {requester}</p>\
        </div>\
        <div class=\"ticket-card-footer\">\
        <button class=\"btn-sm\" hx-get=\"/api/ui/tickets/{id}\" hx-target=\"{hash}ticket-detail\">View</button>\
        <button class=\"btn-sm btn-success\" hx-put=\"/api/tickets/{id}/resolve\" hx-swap=\"none\">Resolve</button>\
        </div>\
        </div>",
        id = ticket.id,
        hash = hash,
        number = html_escape(&ticket.ticket_number),
        subject = html_escape(&ticket.subject),
        requester = html_escape(requester),
        status = status_badge(&ticket.status),
        priority = priority_badge(&ticket.priority),
    )
}
pub fn configure_tickets_ui_routes() -> Router<Arc<TicketsState>> {
    Router::new()
        .route("/api/ui/tickets", get(handle_tickets_list))
        .route("/api/ui/tickets/count", get(handle_tickets_count))
        .route("/api/ui/tickets/open-count", get(handle_open_count))
        .route("/api/ui/tickets/overdue-count", get(handle_overdue_count))
        .route("/api/ui/tickets/cards", get(handle_tickets_cards))
        .route("/api/ui/tickets/search", get(handle_tickets_search))
        .route("/api/ui/tickets/:id", get(handle_ticket_detail))
        .route("/api/ui/tickets/:id/comments", get(handle_ticket_comments))
        .route("/api/ui/tickets/stats/by-status", get(handle_stats_by_status))
        .route("/api/ui/tickets/stats/by-priority", get(handle_stats_by_priority))
        .route("/api/ui/tickets/stats/avg-resolution", get(handle_avg_resolution))
        .route("/api/ui/tickets/stats/open", get(handle_stats_open))
        .route("/api/ui/tickets/stats/urgent", get(handle_stats_urgent))
        .route("/api/ui/tickets/stats/resolved-today", get(handle_stats_resolved_today))
        .route("/api/ui/tickets/stats/ai-resolved", get(handle_stats_ai_resolved))
        .route("/api/ui/tickets/ai-suggest", post(handle_ai_suggest))
}
pub(super) async fn handle_tickets_list(
    State(state): State<Arc<TicketsState>>,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(render_empty_state("🎫", "No tickets", "Unable to load tickets"));
    };

    let branch_id = get_bot_context(&state);

    let mut q = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(status) = query.status {
        if status != "all" {
            q = q.filter(support_tickets::status.eq(status));
        }
    }
    if let Some(priority) = query.priority {
        if priority != "all" {
            q = q.filter(support_tickets::priority.eq(priority));
        }
    }
    if let Some(category) = query.category {
        if category != "all" {
            q = q.filter(support_tickets::category.eq(category));
        }
    }
    if let Some(record_type) = query.record_type {
        if record_type != "all" {
            q = q.filter(support_tickets::record_type.eq(record_type));
        }
    }

    let tickets: Vec<SupportTicket> = q
        .order(support_tickets::created_at.desc())
        .limit(50)
        .load(&mut conn)
        .unwrap_or_default();

    if tickets.is_empty() {
        return Html(render_empty_state(
            "🎫",
            "No tickets yet",
            "Create your first ticket to get started",
        ));
    }

    let mut html = String::from(
        "<table class=\"tickets-table\">\
        <thead>\
        <tr>\
        <th>Number</th>\
        <th>Subject</th>\
        <th>Requester</th>\
        <th>Status</th>\
        <th>Priority</th>\
        <th>Assignee</th>\
        <th>Created</th>\
        <th>Actions</th>\
        </tr>\
        </thead>\
        <tbody>",
    );

    for ticket in &tickets {
        html.push_str(&render_ticket_row(ticket));
    }

    html.push_str("</tbody></table>");
    Html(html)
}
pub(super) async fn handle_tickets_count(
    State(state): State<Arc<TicketsState>>,
    Query(query): Query<StatusQuery>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };

    let branch_id = get_bot_context(&state);

    let count: i64 = if let Some(status) = query.status {
        if status == "all" {
            support_tickets::table
                .filter(support_tickets::branch_id.eq(branch_id))
                .count()
                .get_result(&mut conn)
                .unwrap_or(0)
        } else {
            support_tickets::table
                .filter(support_tickets::branch_id.eq(branch_id))
                .filter(support_tickets::status.eq(status))
                .count()
                .get_result(&mut conn)
                .unwrap_or(0)
        }
    } else {
        support_tickets::table
            .filter(support_tickets::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0)
    };

    Html(count.to_string())
}
pub(super) async fn handle_open_count(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };

    let branch_id = get_bot_context(&state);

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
pub(super) async fn handle_overdue_count(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("0".to_string());
    };

    let branch_id = get_bot_context(&state);
    let now = ChronoUtc::now();

    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.ne("closed"))
        .filter(support_tickets::status.ne("resolved"))
        .filter(support_tickets::due_date.lt(now))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    Html(count.to_string())
}
