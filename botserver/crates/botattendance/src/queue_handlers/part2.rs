use super::*;

pub async fn resolve_conversation(
    State(config): State<Arc<AttendanceConfig>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let session_id = payload.get("session_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok()).unwrap_or_else(Uuid::nil);
    info!("Resolving session {}", session_id);
    let result = tokio::task::spawn_blocking({
        let pool = config.pool.clone();
        move || {
            let mut db_conn = pool.get().map_err(|e| format!("Failed to get database connection: {}", e))?;
            let session: UserSession = user_sessions::table
                .filter(user_sessions::id.eq(session_id))
                .first(&mut db_conn)
                .map_err(|e| format!("Session not found: {}", e))?;
            let mut ctx = session.context_data;
            ctx["status"] = serde_json::json!("resolved");
            ctx["resolved_at"] = serde_json::json!(Utc::now().to_rfc3339());
            ctx["resolved"] = serde_json::json!(true);
            diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_id)))
                .set((user_sessions::context_data.eq(&ctx), user_sessions::updated_at.eq(Utc::now())))
                .execute(&mut db_conn)
                .map_err(|e| format!("Failed to update session: {e}"))?;
            crate::webhooks::emit_webhook_event(&mut db_conn, session.bot_id, "session.resolved", serde_json::json!({
                "session_id": session_id, "resolved_at": Utc::now().to_rfc3339()
            }));
            Ok::<(), String>(())
        }
    })
    .await;
    match result {
        Ok(Ok(())) => (StatusCode::OK, Json(serde_json::json!({
            "success": true, "session_id": session_id, "resolved_at": Utc::now().to_rfc3339()
        }))),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"success": false, "error": e}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"success": false, "error": format!("{:?}", e)}))),
    }
}
pub async fn get_insights(
    State(config): State<Arc<AttendanceConfig>>,
    Path(session_id): Path<Uuid>,
) -> impl IntoResponse {
    info!("Getting insights for session {}", session_id);
    let result = tokio::task::spawn_blocking({
        let pool = config.pool.clone();
        let master_key = config.master_key.clone();
        move || {
            let mut db_conn = pool.get().map_err(|e| format!("Failed to get database connection: {}", e))?;
            let messages: Vec<(String, i32)> = message_history::table
                .filter(message_history::session_id.eq(session_id))
                .select((message_history::content_encrypted, message_history::role))
                .order(message_history::created_at.desc())
                .limit(10)
                .load(&mut db_conn)
                .map_err(|e| format!("Failed to load messages: {}", e))?;

            let bot_id: Uuid = user_sessions::table
                .filter(user_sessions::id.eq(session_id))
                .select(user_sessions::bot_id)
                .first(&mut db_conn)
                .map_err(|e| format!("Failed to get bot_id: {}", e))?;
            let key_bytes = derive_scope_key(&master_key, "message", &bot_id);

            let user_messages: Vec<String> = messages.iter()
                .filter(|(_, r)| *r == 0)
                .map(|(c, _)| {
                    decrypt_field(c, &key_bytes)
                        .unwrap_or_else(|_| c.clone())
                })
                .collect();

            let sentiment = if user_messages.iter().any(|m| {
                m.to_lowercase().contains("urgent") || m.to_lowercase().contains("problem") || m.to_lowercase().contains("issue")
            }) {
                "negative"
            } else if user_messages.iter().any(|m| m.to_lowercase().contains("thanks") || m.to_lowercase().contains("great")) {
                "positive"
            } else {
                "neutral"
            };
            let suggested_reply = if sentiment == "negative" {
                "I understand this is frustrating. Let me help you resolve this immediately."
            } else {
                "How can I assist you further?"
            };
            Ok::<serde_json::Value, String>(serde_json::json!({
                "session_id": session_id, "sentiment": sentiment, "message_count": messages.len(),
                "suggested_reply": suggested_reply, "key_topics": ["support", "technical"],
                "priority": if sentiment == "negative" { "high" } else { "normal" }, "language": "en"
            }))
        }
    })
    .await;
    match result {
        Ok(Ok(insights)) => (StatusCode::OK, Json(insights)),
        Ok(Err(e)) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Task error: {}", e)}))),
    }
}
pub async fn get_kanban(
    State(config): State<Arc<AttendanceConfig>>,
    Query(query): Query<KanbanQuery>,
) -> Result<Json<KanbanBoard>, (StatusCode, String)> {
    let pool = config.pool.clone();
    let get_default_bot = config.get_default_bot.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;
        let resolved_bot_id = query.bot_id.unwrap_or_else(|| {
            let branch_id = get_default_bot(&mut conn);
            branch_id
        });
        let sessions: Vec<UserSession> = user_sessions::table
            .filter(user_sessions::bot_id.eq(resolved_bot_id))
            .filter(user_sessions::context_data.contains(serde_json::json!({"status": ""})))
            .load(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
        let mut new_items = Vec::new();
        let mut waiting_items = Vec::new();
        let mut active_items = Vec::new();
        let mut pending_customer_items = Vec::new();
        let mut resolved_items = Vec::new();
        for session in sessions {
            let status = session.context_data.get("status").and_then(|v| v.as_str()).unwrap_or("new").to_string();
            let assigned_to = session.context_data.get("attendant_id").and_then(|v| v.as_str()).and_then(|s| Uuid::parse_str(s).ok());
            let assigned_to_name = session.context_data.get("attendant_name").and_then(|v| v.as_str()).map(String::from);
            let last_message = session.context_data.get("last_message").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let last_message_time = session.context_data.get("last_message_time").and_then(|v| v.as_str()).map(String::from).unwrap_or_else(|| session.created_at.to_rfc3339());
            let waiting_time = Utc::now().signed_duration_since(session.created_at).num_seconds();
            let priority = session.context_data.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let channel = session.context_data.get("channel").and_then(|v| v.as_str()).unwrap_or("web").to_string();
            let user_name = session.context_data.get("user_name").and_then(|v| v.as_str()).unwrap_or("Anonymous").to_string();
            let user_email = session.context_data.get("user_email").and_then(|v| v.as_str()).map(String::from);
            let item = QueueItem {
                session_id: session.id, user_id: session.user_id.and_then(|u| Uuid::parse_str(&u).ok()).unwrap_or(Uuid::nil()), bot_id: session.bot_id,
                channel, user_name, user_email, last_message, last_message_time,
                waiting_time_seconds: waiting_time, priority,
                status: match status.as_str() {
                    "waiting" => QueueStatus::Waiting,
                    "assigned" => QueueStatus::Assigned,
                    "active" => QueueStatus::Active,
                    "resolved" => QueueStatus::Resolved,
                    "abandoned" => QueueStatus::Abandoned,
                    _ => QueueStatus::Waiting,
                },
                assigned_to, assigned_to_name,
            };
            if let Some(ref ch) = query.channel {
                if item.channel != *ch { continue; }
            }
            match status.as_str() {
                "new" | "waiting" => new_items.push(item),
                "pending_customer" => pending_customer_items.push(item),
                "assigned" => waiting_items.push(item),
                "active" => active_items.push(item),
                "resolved" | "closed" | "abandoned" => resolved_items.push(item),
                _ => new_items.push(item),
            }
        }
        let columns = vec![
            KanbanColumn { id: "new".to_string(), title: "New".to_string(), items: new_items },
            KanbanColumn { id: "waiting".to_string(), title: "Waiting".to_string(), items: waiting_items },
            KanbanColumn { id: "active".to_string(), title: "Active".to_string(), items: active_items },
            KanbanColumn { id: "pending_customer".to_string(), title: "Pending Customer".to_string(), items: pending_customer_items },
            KanbanColumn { id: "resolved".to_string(), title: "Resolved".to_string(), items: resolved_items },
        ];
        Ok::<KanbanBoard, (StatusCode, String)>(KanbanBoard { columns })
    })
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Task error: {e}")))?
    .map(|board| Json(board))
}
