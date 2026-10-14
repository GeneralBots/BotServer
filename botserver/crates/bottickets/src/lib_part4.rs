use super::*;

pub async fn list_activities(
    State(state): State<Arc<TicketsState>>,
    Path(ticket_id): Path<Uuid>,
) -> Result<Json<Vec<TicketActivity>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let comments: Vec<TicketComment> = ticket_comments::table
        .filter(ticket_comments::ticket_id.eq(ticket_id))
        .order(ticket_comments::created_at.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    let ticket: SupportTicket = support_tickets::table
        .filter(support_tickets::id.eq(ticket_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".to_string()))?;

    let mut activities: Vec<TicketActivity> = comments
        .into_iter()
        .map(|c| TicketActivity {
            id: c.id,
            ticket_id: c.ticket_id,
            actor_id: c.author_id,
            actor_name: c.author_name.unwrap_or_default(),
            activity_type: if c.is_internal { "internal_note".to_string() } else { "comment".to_string() },
            description: c.content,
            metadata: c.attachments,
            created_at: c.created_at,
        })
        .collect();

    activities.push(TicketActivity {
        id: Uuid::new_v4(),
        ticket_id: ticket.id,
        actor_id: ticket.requester_id,
        actor_name: "system".to_string(),
        activity_type: "ticket_created".to_string(),
        description: format!("Ticket #{} created", ticket.ticket_number),
        metadata: serde_json::json!({"priority": ticket.priority, "status": ticket.status}),
        created_at: ticket.created_at,
    });

    if let Some(assigned_to) = ticket.assignee_id {
        activities.push(TicketActivity {
            id: Uuid::new_v4(),
            ticket_id: ticket.id,
            actor_id: Some(assigned_to),
            actor_name: "system".to_string(),
            activity_type: "ticket_assigned".to_string(),
            description: "Ticket assigned".to_string(),
            metadata: serde_json::json!({"assignee": assigned_to}),
            created_at: ticket.updated_at,
        });
    }

    activities.sort_by_key(|a| a.created_at);
    Ok(Json(activities))
}
pub async fn add_activity(
    State(state): State<Arc<TicketsState>>,
    Path(ticket_id): Path<Uuid>,
    Json(req): Json<AddActivityRequest>,
) -> Result<Json<TicketActivity>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let exists: bool = support_tickets::table
        .filter(support_tickets::id.eq(ticket_id))
        .count()
        .get_result::<i64>(&mut conn)
        .map(|c| c > 0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Ticket not found".to_string()));
    }

    let now = Utc::now();
    let id = Uuid::new_v4();
    let comment = TicketComment {
        id,
        ticket_id,
        author_id: None,
        author_name: Some(req.actor_name),
        author_email: None,
        content: req.description,
        is_internal: req.activity_type == "internal_note",
        attachments: req.metadata.unwrap_or(serde_json::json!([])),
        created_at: now,
    };

    diesel::insert_into(ticket_comments::table)
        .values(&comment)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    diesel::update(support_tickets::table.filter(support_tickets::id.eq(ticket_id)))
        .set(support_tickets::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    Ok(Json(TicketActivity {
        id,
        ticket_id,
        actor_id: None,
        actor_name: comment.author_name.unwrap_or_default(),
        activity_type: req.activity_type,
        description: comment.content,
        metadata: comment.attachments,
        created_at: now,
    }))
}
