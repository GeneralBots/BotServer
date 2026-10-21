use super::*;

pub(super) fn get_bot_context(state: &Arc<TicketsState>) -> Uuid {
    let Ok(mut conn) = state.pool.get() else {
        return Uuid::nil();
    };
    let bid: Uuid = bots::table
        .filter(bots::is_default_for_branch.eq(true))
        .order(bots::created_at.asc())
        .select(bots::branch_id)
        .first(&mut conn)
        .unwrap_or(Uuid::nil());
    bid
}
pub(super) fn org_and_bot(conn: &mut diesel::PgConnection, branch_id: Uuid) -> (Uuid, Uuid) {
    bots::table
        .filter(bots::branch_id.eq(branch_id))
        .select((bots::org_id, bots::id))
        .first(conn)
        .unwrap_or((Uuid::nil(), Uuid::nil()))
}
pub(super) fn generate_ticket_number(conn: &mut diesel::PgConnection, branch_id: Uuid) -> String {
    let count: i64 = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    format!("TKT-{:06}", count + 1)
}
/// Accepts both JSON (API clients) and urlencoded payloads (plain HTML forms
/// and HTMX's default encoding). Requiring JSON alone made every HTMX form
/// submission fail with 415 Unsupported Media Type.
pub(super) fn decode_create_ticket(
    headers: &axum::http::HeaderMap,
    body: &[u8],
) -> Result<CreateTicketRequest, (StatusCode, String)> {
    let is_json = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.to_ascii_lowercase().contains("json"));
    if is_json {
        serde_json::from_slice(body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid JSON body: {e}")))
    } else {
        serde_urlencoded::from_bytes(body)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid form body: {e}")))
    }
}
pub async fn create_ticket(
    State(state): State<Arc<TicketsState>>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let req = decode_create_ticket(&headers, &body)?;
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let ticket_number = generate_ticket_number(&mut conn, branch_id);
    let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);

    let due_date = req
        .due_date
        .and_then(|d| DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.with_timezone(&Utc));

    let ticket = SupportTicket {
        id,
        org_id,
        bot_id,
        ticket_number,
        subject: req.subject,
        description: req.description,
        status: "open".to_string(),
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        category: req.category,
        source: req.source.unwrap_or_else(|| "web".to_string()),
        requester_id: None,
        requester_email: req.requester_email,
        requester_name: req.requester_name,
        assignee_id: req.assignee_id,
        team_id: None,
        due_date,
        first_response_at: None,
        resolved_at: None,
        closed_at: None,
        satisfaction_rating: None,
        record_type: req.record_type.clone().unwrap_or_else(|| "ticket".to_string()),
        tags: req.tags.unwrap_or_default().into_iter().map(Some).collect(),
        custom_fields: serde_json::json!({}),
        created_at: now,
        updated_at: now,
        branch_id: Some(branch_id),
    };

    diesel::insert_into(support_tickets::table)
        .values(&ticket)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(ticket))
}
pub async fn list_tickets(
    State(state): State<Arc<TicketsState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<SupportTicket>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = support_tickets::table
        .filter(support_tickets::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(status) = query.status {
        q = q.filter(support_tickets::status.eq(status));
    }

    if let Some(priority) = query.priority {
        q = q.filter(support_tickets::priority.eq(priority));
    }

    if let Some(category) = query.category {
        q = q.filter(support_tickets::category.eq(category));
    }

    if let Some(assignee_id) = query.assignee_id {
        q = q.filter(support_tickets::assignee_id.eq(assignee_id));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            support_tickets::subject
                .ilike(pattern.clone())
                .or(support_tickets::description.ilike(pattern.clone()))
                .or(support_tickets::ticket_number.ilike(pattern)),
        );
    }

    let tickets: Vec<SupportTicket> = q
        .order(support_tickets::created_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(tickets))
}
pub async fn get_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let ticket: SupportTicket = support_tickets::table
        .filter(support_tickets::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".to_string()))?;

    Ok(Json(ticket))
}
pub async fn get_ticket_with_comments(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<TicketWithComments>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let ticket: SupportTicket = support_tickets::table
        .filter(support_tickets::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".to_string()))?;

    let comments: Vec<TicketComment> = ticket_comments::table
        .filter(ticket_comments::ticket_id.eq(id))
        .order(ticket_comments::created_at.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(TicketWithComments { ticket, comments }))
}
pub async fn update_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateTicketRequest>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
        .set(support_tickets::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if let Some(subject) = req.subject {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::subject.eq(subject))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(description) = req.description {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::description.eq(description))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(priority) = req.priority {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::priority.eq(priority))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(category) = req.category {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::category.eq(category))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(assignee_id) = req.assignee_id {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::assignee_id.eq(Some(assignee_id)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(tags) = req.tags {
        diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
            .set(support_tickets::tags.eq(tags))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    get_ticket(State(state), Path(id)).await
}
pub async fn assign_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<AssignTicketRequest>,
) -> Result<Json<SupportTicket>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let assignee_id = if let Some(assignee_id) = req.assignee_id {
        assignee_id
    } else if let Some(assignee) = req.assignee {
        let value = assignee.trim();
        if value.is_empty() {
            return Err((StatusCode::BAD_REQUEST, "assignee cannot be empty".to_string()));
        }
        resolve_user_id(&mut conn, value)?
    } else {
        return Err((StatusCode::BAD_REQUEST, "assignee_id or assignee required".to_string()));
    };

    let now = Utc::now();

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(id)))
        .set((
            support_tickets::assignee_id.eq(Some(assignee_id)),
            support_tickets::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    get_ticket(State(state), Path(id)).await
}
pub(super) fn resolve_user_id(conn: &mut diesel::PgConnection, value: &str) -> Result<Uuid, (StatusCode, String)> {
    use diesel::prelude::*;

    #[derive(diesel::QueryableByName)]
    struct UserIdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }

    let value_like = format!("%{value}%");
    let row: Result<UserIdRow, diesel::result::Error> = diesel::sql_query(
        "SELECT id FROM users WHERE LOWER(email) = LOWER($1) OR LOWER(username) = LOWER($1) OR LOWER(username) LIKE LOWER($2) LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(value)
    .bind::<diesel::sql_types::Text, _>(&value_like)
    .get_result(conn);

    match row {
        Ok(row) => Ok(row.id),
        Err(diesel::result::Error::NotFound) => Err((
            StatusCode::NOT_FOUND,
            format!("No user found matching '{value}'. Use an existing user email or username."),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}"))),
    }
}
