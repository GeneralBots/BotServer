use super::*;

pub async fn list_categories(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<Vec<TicketCategory>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let categories: Vec<TicketCategory> = ticket_categories::table
        .filter(ticket_categories::is_active.eq(true))
        .order(ticket_categories::sort_order.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(categories))
}
pub async fn create_category(
    State(state): State<Arc<TicketsState>>,
    Json(req): Json<CreateCategoryRequest>,
) -> Result<Json<TicketCategory>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = get_bot_context(&state);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let max_order: Option<i32> = ticket_categories::table
        .select(diesel::dsl::max(ticket_categories::sort_order))
        .first(&mut conn)
        .unwrap_or(None);

    let category = TicketCategory {
        id,
        branch_id,
        name: req.name,
        description: req.description,
        parent_id: req.parent_id,
        color: req.color,
        icon: req.icon,
        sort_order: max_order.unwrap_or(0) + 1,
        is_active: true,
        created_at: now,
    };

    diesel::insert_into(ticket_categories::table)
        .values(&category)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(category))
}
pub async fn list_sla_policies(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<Vec<TicketSlaPolicy>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let policies: Vec<TicketSlaPolicy> = ticket_sla_policies::table
        .filter(ticket_sla_policies::is_active.eq(true))
        .order(ticket_sla_policies::priority.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(policies))
}
pub async fn list_tags(
    State(state): State<Arc<TicketsState>>,
) -> Result<Json<Vec<TicketTag>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let tags: Vec<TicketTag> = ticket_tags::table
        .order(ticket_tags::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(tags))
}
pub fn configure_tickets_routes() -> Router<Arc<TicketsState>> {
    Router::new()
        .route("/api/tickets", get(list_tickets).post(create_ticket))
        .route("/api/tickets/stats", get(get_ticket_stats))
        .route("/api/tickets/overdue", get(list_overdue_tickets))
        .route("/api/tickets/:id", get(get_ticket).put(update_ticket).delete(delete_ticket))
        .route("/api/tickets/:id/full", get(get_ticket_with_comments))
        .route("/api/tickets/:id/assign", put(assign_ticket))
        .route("/api/tickets/:id/status", put(change_status))
        .route("/api/tickets/:id/resolve", put(resolve_ticket))
        .route("/api/tickets/:id/close", put(close_ticket))
        .route("/api/tickets/:id/reopen", put(reopen_ticket))
        .route("/api/tickets/:id/comments", get(list_comments).post(add_comment))
        .route("/api/tickets/:id/activities", get(list_activities).post(add_activity))
        .route("/api/tickets/:id/ai-suggest", get(ai_suggest_for_ticket).post(ai_suggest_for_ticket))
        .route("/api/tickets/canned", get(list_canned_responses).post(create_canned_response))
        .route("/api/tickets/categories", get(list_categories).post(create_category))
        .route("/api/tickets/sla", get(list_sla_policies))
        .route("/api/tickets/tags", get(list_tags))
}
