use super::*;

pub fn register_attendance_keywords(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    debug!("Registering CRM attendance keywords...");

    register_get_queue(Arc::clone(&state), user.clone(), engine);
    register_next_in_queue(Arc::clone(&state), user.clone(), engine);
    register_assign_conversation(Arc::clone(&state), user.clone(), engine);
    register_resolve_conversation(Arc::clone(&state), user.clone(), engine);
    register_set_priority(Arc::clone(&state), user.clone(), engine);

    register_get_attendants(Arc::clone(&state), user.clone(), engine);
    register_set_attendant_status(Arc::clone(&state), user.clone(), engine);
    register_get_attendant_stats(Arc::clone(&state), user.clone(), engine);

    register_get_tips(Arc::clone(&state), user.clone(), engine);
    register_polish_message(Arc::clone(&state), user.clone(), engine);
    register_get_smart_replies(Arc::clone(&state), user.clone(), engine);
    register_get_summary(Arc::clone(&state), user.clone(), engine);
    register_analyze_sentiment(Arc::clone(&state), user.clone(), engine);

    register_tag_conversation(Arc::clone(&state), user.clone(), engine);
    register_add_note(Arc::clone(&state), user.clone(), engine);
    register_get_customer_history(state, user, engine);

    debug!("CRM attendance keywords registered successfully");
}
pub(super) fn register_get_queue(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    engine.register_fn("get_queue", move || -> Dynamic {
        get_queue_impl(&state_clone, None)
    });

    let state_clone2 = Arc::clone(&state);
    engine.register_fn("get_queue", move |filter: &str| -> Dynamic {
        get_queue_impl(&state_clone2, Some(filter.to_string()))
    });

    let state_clone3 = Arc::clone(&state);
    if let Err(e) = engine
        .register_custom_syntax(["GET", "QUEUE"], false, move |_context, _inputs| {
            Ok(get_queue_impl(&state_clone3, None))
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone4 = state;
    if let Err(e) = engine
        .register_custom_syntax(["GET", "QUEUE", "$expr$"], false, move |context, inputs| {
            let filter = context.eval_expression_tree(&inputs[0])?.to_string();
            Ok(get_queue_impl(&state_clone4, Some(filter)))
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn get_queue_impl(state: &Arc<dyn BasicRuntime>, filter: Option<String>) -> Dynamic {
    let conn = state.db_pool().clone();

    let result = std::thread::spawn(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return create_error_result(&format!("DB error: {e}"));
            }
        };

        use botbasic_types::schema::user_sessions;

        let mut query = user_sessions::table
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("needs_human")
                    .eq("true"),
            )
            .into_boxed();

        if let Some(ref filter_str) = filter {
            if filter_str.contains("channel=") {
                let channel = filter_str.replace("channel=", "");
                query = query.filter(
                    user_sessions::context_data
                        .retrieve_as_text("channel")
                        .eq(channel),
                );
            }
        }

        let sessions: Vec<UserSession> = match query.load(&mut db_conn) {
            Ok(s) => s,
            Err(e) => {
                log::error!("Query error: {}", e);
                return create_error_result(&format!("Query error: {}", e));
            }
        };

        let mut waiting = 0;
        let mut assigned = 0;
        let mut active = 0;
        let mut resolved = 0;

        for session in &sessions {
            let status = session
                .context_data
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("waiting");

            match status {
                "assigned" => assigned += 1,
                "active" => active += 1,
                "resolved" => resolved += 1,
                _ => waiting += 1,
            }
        }

        let mut result = Map::new();
        result.insert("success".into(), Dynamic::from(true));
        result.insert("total".into(), Dynamic::from(sessions.len() as i64));
        result.insert("waiting".into(), Dynamic::from(waiting));
        result.insert("assigned".into(), Dynamic::from(assigned));
        result.insert("active".into(), Dynamic::from(active));
        result.insert("resolved".into(), Dynamic::from(resolved));

        let items: Array = sessions
            .iter()
            .take(20)
            .map(|s| {
                let mut item = Map::new();
                item.insert("session_id".into(), Dynamic::from(s.id.to_string()));
                item.insert("user_id".into(), Dynamic::from(s.user_id.to_string()));
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
                            .unwrap_or("waiting")
                            .to_string(),
                    ),
                );
                item.insert(
                    "name".into(),
                    Dynamic::from(
                        s.context_data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                    ),
                );
                item.insert(
                    "priority".into(),
                    Dynamic::from(
                        s.context_data
                            .get("priority")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(1),
                    ),
                );
                Dynamic::from(item)
            })
            .collect();

        result.insert("items".into(), Dynamic::from(items));

        Dynamic::from(result)
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_next_in_queue(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    if let Err(e) = engine
        .register_custom_syntax(["NEXT", "IN", "QUEUE"], false, move |_context, _inputs| {
            Ok(next_in_queue_impl(&state_clone))
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn("next_in_queue", move || -> Dynamic {
        next_in_queue_impl(&state)
    });
}
pub fn next_in_queue_impl(state: &Arc<dyn BasicRuntime>) -> Dynamic {
    let conn = state.db_pool().clone();

    let result = std::thread::spawn(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => return create_error_result(&format!("DB error: {}", e)),
        };

        use botbasic_types::schema::user_sessions;

        let session: Option<UserSession> = user_sessions::table
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("needs_human")
                    .eq("true"),
            )
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("status")
                    .eq("waiting"),
            )
            .order(user_sessions::created_at.asc())
            .first(&mut db_conn)
            .optional()
            .unwrap_or(None);

        match session {
            Some(s) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert("session_id".into(), Dynamic::from(s.id.to_string()));
                result.insert("user_id".into(), Dynamic::from(s.user_id.to_string()));
                result.insert(
                    "name".into(),
                    Dynamic::from(
                        s.context_data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown")
                            .to_string(),
                    ),
                );
                result.insert(
                    "channel".into(),
                    Dynamic::from(
                        s.context_data
                            .get("channel")
                            .and_then(|v| v.as_str())
                            .unwrap_or("web")
                            .to_string(),
                    ),
                );
                result.insert(
                    "phone".into(),
                    Dynamic::from(
                        s.context_data
                            .get("phone")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                );
                Dynamic::from(result)
            }
            None => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(false));
                result.insert("message".into(), Dynamic::from("Queue is empty"));
                Dynamic::from(result)
            }
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
