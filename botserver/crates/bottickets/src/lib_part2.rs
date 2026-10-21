use super::*;

pub async fn change_status(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ChangeStatusRequest>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
        .set((
            support_tickets::status.eq(&req.status),
            support_tickets::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if req.status == "resolved" {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::resolved_at.eq(Some(now)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if req.status == "closed" {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::closed_at.eq(Some(now)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    get_ticket(State(state), Path(id)).await
}
pub async fn resolve_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    change_status(
        State(state),
        Path(id),
        Json(ChangeStatusRequest {
            status: "resolved".to_string(),
            resolution: None,
        }),
    )
    .await
}
pub async fn close_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    change_status(
        State(state),
        Path(id),
        Json(ChangeStatusRequest {
            status: "closed".to_string(),
            resolution: None,
        }),
    )
    .await
}
pub async fn reopen_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
        .set((
            support_tickets::status.eq("open"),
            support_tickets::resolved_at.eq(None::<DateTime<Utc>>),
            support_tickets::closed_at.eq(None::<DateTime<Utc>>),
            support_tickets::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    get_ticket(State(state), Path(id)).await
}
pub async fn delete_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(support_tickets::table.filter(support_tickets::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn add_comment(
    State(state): State<Arc<TicketsState>>,
    Path(ticket_id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<TicketComment>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    let comment = TicketComment {
        id,
        ticket_id,
        author_id: None,
        author_name: req.author_name,
        author_email: req.author_email,
        content: req.content,
        is_internal: req.is_internal.unwrap_or(false),
        attachments: serde_json::json!([]),
        created_at: now,
    };

    diesel::insert_into(ticket_comments::table)
        .values(&comment)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    let ticket: SupportTicket = support_tickets::table
        .filter(support_tickets::id.eq(ticket_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".to_string()))?;

    if ticket.first_response_at.is_none() && !comment.is_internal {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(ticket_id)))
            .set(support_tickets::first_response_at.eq(Some(now)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(ticket_id)))
        .set(support_tickets::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    Ok(Json(comment))
}
pub async fn list_comments(
    State(state): State<Arc<TicketsState>>,
    Path(ticket_id): Path<Uuid>,
) -> Result<Json<Vec<TicketComment>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let comments: Vec<TicketComment> = ticket_comments::table
        .filter(ticket_comments::ticket_id.eq(ticket_id))
        .order(ticket_comments::created_at.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(comments))
}
pub async fn get_ticket_stats(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<TicketStats>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);

    let total_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let open_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("open"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let pending_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("pending"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let resolved_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("resolved"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let closed_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.eq("closed"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let now = Utc::now();
    let overdue_tickets: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.ne("closed"))
        .filter(support_tickets::status.ne("resolved"))
        .filter(support_tickets::due_date.lt(now))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let stats = TicketStats {
        total_tickets,
        open_tickets,
        pending_tickets,
        resolved_tickets,
        closed_tickets,
        avg_resolution_hours: 0.0,
        overdue_tickets,
    };

    Ok(Json(stats))
}
pub async fn list_overdue_tickets(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<Vec<SupportTicket>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);
    let now = Utc::now();

    let tickets: Vec<SupportTicket> = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .filter(support_tickets::status.ne("closed"))
        .filter(support_tickets::status.ne("resolved"))
        .filter(support_tickets::due_date.lt(now))
        .order(support_tickets::due_date.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(tickets))
}
pub async fn list_canned_responses(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<Vec<TicketCannedResponse>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let responses: Vec<TicketCannedResponse> = ticket_canned_responses::table
        .filter(ticket_canned_responses::is_active.eq(true))
        .order(ticket_canned_responses::title.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(responses))
}
pub async fn create_canned_response(
    State(state): State<Arc<TicketsState>>,
    Json(req): Json<CreateCannedResponseRequest>,
) -> Result<Json<TicketCannedResponse>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
    let response = TicketCannedResponse {
        id,
        org_id,
        bot_id,
        title: req.title,
        content: req.content,
        category: req.category,
        shortcut: req.shortcut,
        created_by: None,
        is_active: true,
        created_at: now,
        updated_at: now,
        branch_id: Some(branch_id),
    };

    diesel::insert_into(ticket_canned_responses::table)
        .values(&response)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(response))
}
