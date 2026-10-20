use super::*;

pub fn tag_conversation_impl(
    state: &Arc<dyn BasicRuntime>,
    session_id: &str,
    tags: Vec<String>,
) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };
    let tags_clone = tags;

    let result = std::thread::spawn(move || {
        let Ok(mut db_conn) = conn.get() else {
            return create_error_result("DB error: failed to get connection");
        };

        use botbasic_types::schema::user_sessions;

        let session: UserSession = match user_sessions::table.find(session_uuid).first(&mut db_conn)
        {
            Ok(s) => s,
            Err(_) => return create_error_result("Session not found"),
        };

        let mut ctx = session.context_data;

        let mut existing_tags: Vec<String> = ctx
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        for tag in tags_clone {
            if !existing_tags.contains(&tag) {
                existing_tags.push(tag);
            }
        }

        ctx["tags"] = serde_json::json!(existing_tags);

        match diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_uuid)))
            .set(user_sessions::context_data.eq(&ctx))
            .execute(&mut db_conn)
        {
            Ok(_) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert(
                    "tags".into(),
                    Dynamic::from(
                        existing_tags
                            .iter()
                            .map(|t| Dynamic::from(t.clone()))
                            .collect::<Vec<_>>(),
                    ),
                );
                Dynamic::from(result)
            }
            Err(e) => create_error_result(&format!("Update error: {}", e)),
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_add_note(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["ADD", "NOTE", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let note = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(add_note_impl(&state_clone, &session_id, &note, None))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state;
    engine.register_fn("add_note", move |session_id: &str, note: &str| -> Dynamic {
        add_note_impl(&state_clone2, session_id, note, None)
    });
}
pub fn add_note_impl(
    state: &Arc<dyn BasicRuntime>,
    session_id: &str,
    note: &str,
    author: Option<String>,
) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };
    let note_clone = note.to_string();
    let author_clone = author;

    let result = std::thread::spawn(move || {
        let Ok(mut db_conn) = conn.get() else {
            return create_error_result("DB error: failed to get connection");
        };

        use botbasic_types::schema::user_sessions;

        let session: UserSession = match user_sessions::table.find(session_uuid).first(&mut db_conn)
        {
            Ok(s) => s,
            Err(_) => return create_error_result("Session not found"),
        };

        let mut ctx = session.context_data;

        let mut notes: Vec<serde_json::Value> = ctx
            .get("notes")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        notes.push(serde_json::json!({
            "text": &note_clone,
            "author": author_clone.unwrap_or_else(|| "system".to_string()),
            "timestamp": Utc::now().to_rfc3339()
        }));

        ctx["notes"] = serde_json::json!(notes);

        match diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_uuid)))
            .set(user_sessions::context_data.eq(&ctx))
            .execute(&mut db_conn)
        {
            Ok(_) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert("note_count".into(), Dynamic::from(notes.len() as i64));
                Dynamic::from(result)
            }
            Err(e) => create_error_result(&format!("Update error: {}", e)),
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_get_customer_history(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "CUSTOMER", "HISTORY", "$expr$"],
            false,
            move |context, inputs| {
                let user_id = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(get_customer_history_impl(&state_clone, &user_id))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state;
    engine.register_fn("get_customer_history", move |user_id: &str| -> Dynamic {
        get_customer_history_impl(&state_clone2, user_id)
    });
}
pub fn get_customer_history_impl(state: &Arc<dyn BasicRuntime>, user_id: &str) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(user_uuid) = Uuid::parse_str(user_id) else {
        return create_error_result("Invalid user ID");
    };

    let result = std::thread::spawn(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => return create_error_result(&format!("DB error: {}", e)),
        };

        use botbasic_types::schema::user_sessions;

        let sessions: Vec<UserSession> = user_sessions::table
            .filter(user_sessions::user_id.eq(user_uuid.to_string()))
            .order(user_sessions::created_at.desc())
            .limit(10)
            .load(&mut db_conn)
            .unwrap_or_default();

        let session_items: Vec<Dynamic> = sessions
            .iter()
            .map(|s| {
                let mut item = Map::new();
                item.insert("session_id".into(), Dynamic::from(s.id.to_string()));
                item.insert(
                    "channel".into(),
                    Dynamic::from(
                        s.context_data
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("web")
                            .to_string(),
                    ),
                );
                item.insert(
                    "status".into(),
                    Dynamic::from(
                        s.context_data
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                    ),
                );
                item.insert("created_at".into(), Dynamic::from(s.created_at.to_string()));
                Dynamic::from(item)
            })
            .collect();

        let mut result = Map::new();
        result.insert("success".into(), Dynamic::from(true));
        result.insert("user_id".into(), Dynamic::from(user_uuid.to_string()));
        result.insert("session_count".into(), Dynamic::from(sessions.len() as i64));
        result.insert("sessions".into(), Dynamic::from(session_items));
        Dynamic::from(result)
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub fn create_error_result(message: &str) -> Dynamic {
    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(false));
    result.insert("error".into(), Dynamic::from(message.to_string()));
    Dynamic::from(result)
}
