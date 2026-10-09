use super::*;

pub async fn ai_suggest_for_ticket(
    State(state): State<Arc<TicketsState>>,
    Path(ticket_id): Path<Uuid>,
) -> Result<Json<AiSuggestion>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let ticket: SupportTicket = support_tickets::table
        .filter(support_tickets::id.eq(ticket_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".to_string()))?;

    let lower_subject = ticket.subject.to_lowercase();
    let lower_body = ticket.description.unwrap_or_default().to_lowercase();
    let haystack = format!("{lower_subject} {lower_body}");

    let (suggested_category, suggested_priority, reasoning) = if haystack.contains("password")
        || haystack.contains("login")
        || haystack.contains("authentication")
        || haystack.contains("access denied")
    {
        ("Authentication".to_string(), "high".to_string(), "Detected authentication keywords".to_string())
    } else if haystack.contains("payment")
        || haystack.contains("invoice")
        || haystack.contains("billing")
        || haystack.contains("charge")
    {
        ("Billing".to_string(), "high".to_string(), "Detected payment-related keywords".to_string())
    } else if haystack.contains("crash")
        || haystack.contains("error")
        || haystack.contains("exception")
        || haystack.contains("500")
    {
        ("Bug Report".to_string(), "high".to_string(), "Detected error or crash keywords".to_string())
    } else if haystack.contains("how to")
        || haystack.contains("help")
        || haystack.contains("tutorial")
    {
        ("How-To".to_string(), "low".to_string(), "Detected help/tutorial request".to_string())
    } else if haystack.contains("feature")
        || haystack.contains("request")
        || haystack.contains("enhancement")
    {
        ("Feature Request".to_string(), "low".to_string(), "Detected feature request".to_string())
    } else {
        ("General".to_string(), "normal".to_string(), "No specific category matched".to_string())
    };

    let similar: Vec<Uuid> = support_tickets::table
        .filter(support_tickets::id.ne(ticket_id))
        .filter(support_tickets::category.is_not_null())
        .order(support_tickets::created_at.desc())
        .limit(3)
        .select(support_tickets::id)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    let tags: Vec<String> = if suggested_priority == "high" {
        vec!["urgent".to_string(), suggested_category.to_lowercase()]
    } else {
        vec![suggested_category.to_lowercase()]
    };

    let confidence = if haystack.len() > 50 { 0.75 } else { 0.5 };

    Ok(Json(AiSuggestion {
        ticket_id,
        suggested_category,
        suggested_priority,
        suggested_assignee: None,
        suggested_tags: tags,
        similar_tickets: similar,
        confidence,
        reasoning,
    }))
}
