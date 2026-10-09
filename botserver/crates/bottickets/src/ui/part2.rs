use super::*;

pub(super) async fn handle_tickets_cards(
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

    let tickets: Vec<SupportTicket> = q
        .order(support_tickets::created_at.desc())
        .limit(20)
        .load(&mut conn)
        .unwrap_or_default();

    if tickets.is_empty() {
        return Html(render_empty_state(
            "🎫",
            "No tickets",
            "No tickets match your criteria",
        ));
    }

    let mut html = String::from("<div class=\"tickets-grid\">");
    for ticket in &tickets {
        html.push_str(&render_ticket_card(ticket));
    }
    html.push_str("</div>");

    Html(html)
}
pub(super) async fn handle_tickets_search(
    State(state): State<Arc<TicketsState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(render_empty_state("🔍", "Search error", "Unable to search tickets"));
    };

    let branch_id = get_bot_context(&state);

    let search_term = query.q.unwrap_or_default();
    if search_term.is_empty() {
        return Html(render_empty_state(
            "🔍",
            "Enter search term",
            "Type to search tickets",
        ));
    }

    let pattern = format!("%{search_term}%");

    let tickets: Vec<SupportTicket> = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(
            support_tickets::subject
                .ilike(pattern.clone())
                .or(support_tickets::description.ilike(pattern.clone()))
                .or(support_tickets::ticket_number.ilike(pattern)),
        )
        .order(support_tickets::created_at.desc())
        .limit(20)
        .load(&mut conn)
        .unwrap_or_default();

    if tickets.is_empty() {
        return Html(render_empty_state(
            "🔍",
            "No results",
            "No tickets match your search",
        ));
    }

    let mut html = String::from("<div class=\"search-results\">");
    for ticket in &tickets {
        html.push_str(&render_ticket_card(ticket));
    }
    html.push_str("</div>");

    Html(html)
}
pub(super) async fn handle_ticket_detail(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html(render_empty_state("❌", "Error", "Unable to load ticket"));
    };

    let ticket: Result<SupportTicket, _> = support_tickets::table
        .filter(support_tickets::id.eq(id))
        .first(&mut conn);

    let Ok(ticket) = ticket else {
        return Html(render_empty_state("❌", "Not found", "Ticket not found"));
    };

    let comments: Vec<TicketComment> = ticket_comments::table
        .filter(ticket_comments::ticket_id.eq(id))
        .order(ticket_comments::created_at.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let requester = ticket
        .requester_name
        .as_deref()
        .or(ticket.requester_email.as_deref())
        .unwrap_or("Unknown");

    let description = ticket
        .description
        .as_deref()
        .unwrap_or("No description provided");

    let created = ticket.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let updated = ticket.updated_at.format("%Y-%m-%d %H:%M:%S").to_string();
    let category = ticket.category.as_deref().unwrap_or("-");

    let mut comments_html = String::new();
    for comment in &comments {
        let author = comment
            .author_name
            .as_deref()
            .or(comment.author_email.as_deref())
            .unwrap_or("Unknown");
        let comment_time = comment.created_at.format("%Y-%m-%d %H:%M").to_string();
        let internal_class = if comment.is_internal { " internal" } else { "" };
        let internal_badge = if comment.is_internal {
            "<span class=\"badge badge-warning\">Internal</span>"
        } else {
            ""
        };

        comments_html.push_str(&format!(
            "<div class=\"comment{}\">\
            <div class=\"comment-header\">\
            <span class=\"comment-author\">{}</span>\
            <span class=\"comment-time\">{}</span>\
            {}\
            </div>\
            <div class=\"comment-body\">{}</div>\
            </div>",
            internal_class,
            html_escape(author),
            comment_time,
            internal_badge,
            html_escape(&comment.content),
        ));
    }

    let html = format!(
        "<div class=\"ticket-detail\">\
        <div class=\"ticket-detail-header\">\
        <h2>{}: {}</h2>\
        <div class=\"ticket-badges\">\
        {}\
        {}\
        </div>\
        </div>\
        <div class=\"ticket-detail-meta\">\
        <div class=\"meta-item\">\
        <label>Requester</label>\
        <span>{}</span>\
        </div>\
        <div class=\"meta-item\">\
        <label>Created</label>\
        <span>{}</span>\
        </div>\
        <div class=\"meta-item\">\
        <label>Updated</label>\
        <span>{}</span>\
        </div>\
        <div class=\"meta-item\">\
        <label>Category</label>\
        <span>{}</span>\
        </div>\
        </div>\
        <div class=\"ticket-detail-description\">\
        <h3>Description</h3>\
        <p>{}</p>\
        </div>\
        <div class=\"ticket-detail-actions\">\
        <button class=\"btn btn-success\" hx-put=\"/api/tickets/{}/resolve\" hx-swap=\"none\">Resolve</button>\
        <button class=\"btn btn-secondary\" hx-put=\"/api/tickets/{}/close\" hx-swap=\"none\">Close</button>\
        <button class=\"btn btn-warning\" hx-put=\"/api/tickets/{}/reopen\" hx-swap=\"none\">Reopen</button>\
        </div>\
        <div class=\"ticket-comments\">\
        <h3>Comments ({})</h3>\
        {}\
        <form class=\"comment-form\" hx-post=\"/api/tickets/{}/comments\" hx-target=\"#ticket-detail\" hx-swap=\"innerHTML\">\
        <textarea name=\"content\" placeholder=\"Add a comment...\" required></textarea>\
        <div class=\"comment-form-actions\">\
        <label>\
        <input type=\"checkbox\" name=\"is_internal\" value=\"true\">\
        Internal note\
        </label>\
        <button type=\"submit\" class=\"btn btn-primary\">Add Comment</button>\
        </div>\
        </form>\
        </div>\
        </div>",
        html_escape(&ticket.ticket_number),
        html_escape(&ticket.subject),
        status_badge(&ticket.status),
        priority_badge(&ticket.priority),
        html_escape(requester),
        created,
        updated,
        html_escape(category),
        html_escape(description),
        ticket.id,
        ticket.id,
        ticket.id,
        comments.len(),
        comments_html,
        ticket.id,
    );

    Html(html)
}
pub(super) async fn handle_ticket_comments(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Html("<p>Unable to load comments</p>".to_string());
    };

    let comments: Vec<TicketComment> = ticket_comments::table
        .filter(ticket_comments::ticket_id.eq(id))
        .order(ticket_comments::created_at.asc())
        .load(&mut conn)
        .unwrap_or_default();

    if comments.is_empty() {
        return Html("<p class=\"no-comments\">No comments yet</p>".to_string());
    }

    let mut html = String::new();
    for comment in &comments {
        let author = comment
            .author_name
            .as_deref()
            .or(comment.author_email.as_deref())
            .unwrap_or("Unknown");
        let comment_time = comment.created_at.format("%Y-%m-%d %H:%M").to_string();
        let internal_class = if comment.is_internal { " internal" } else { "" };

        html.push_str(&format!(
            "<div class=\"comment{}\">\
            <div class=\"comment-header\">\
            <span class=\"comment-author\">{}</span>\
            <span class=\"comment-time\">{}</span>\
            </div>\
            <div class=\"comment-body\">{}</div>\
            </div>",
            internal_class,
            html_escape(author),
            comment_time,
            html_escape(&comment.content),
        ));
    }

    Html(html)
}
