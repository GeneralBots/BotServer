use super::*;

pub(super) fn register_assign_conversation(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["ASSIGN", "CONVERSATION", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let attendant_id = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(assign_conversation_impl(
                    &state_clone,
                    &session_id,
                    &attendant_id,
                ))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn(
        "assign_conversation",
        move |session_id: &str, attendant_id: &str| -> Dynamic {
            assign_conversation_impl(&state, session_id, attendant_id)
        },
    );
}
pub fn assign_conversation_impl(
    state: &Arc<dyn BasicRuntime>,
    session_id: &str,
    attendant_id: &str,
) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };
    let attendant = attendant_id.to_string();

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
        ctx["assigned_to"] = serde_json::json!(attendant);
        ctx["assigned_at"] = serde_json::json!(Utc::now().to_rfc3339());
        ctx["status"] = serde_json::json!("assigned");

        match diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_uuid)))
            .set(user_sessions::context_data.eq(&ctx))
            .execute(&mut db_conn)
        {
            Ok(_) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert("session_id".into(), Dynamic::from(session_uuid.to_string()));
                result.insert("assigned_to".into(), Dynamic::from(attendant));
                result.insert("message".into(), Dynamic::from("Conversation assigned"));
                Dynamic::from(result)
            }
            Err(e) => create_error_result(&format!("Update error: {}", e)),
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_resolve_conversation(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["RESOLVE", "CONVERSATION", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(resolve_conversation_impl(&state_clone, &session_id, None))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state.clone();
    if let Err(e) = engine
        .register_custom_syntax(
            ["RESOLVE", "CONVERSATION", "$expr$", "WITH", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let reason = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(resolve_conversation_impl(
                    &state_clone2,
                    &session_id,
                    Some(reason),
                ))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone3 = state;
    engine.register_fn("resolve_conversation", move |session_id: &str| -> Dynamic {
        resolve_conversation_impl(&state_clone3, session_id, None)
    });
}
pub fn resolve_conversation_impl(
    state: &Arc<dyn BasicRuntime>,
    session_id: &str,
    reason: Option<String>,
) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };
    let reason_clone = reason;

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
        ctx["needs_human"] = serde_json::json!(false);
        ctx["status"] = serde_json::json!("resolved");
        ctx["resolved_at"] = serde_json::json!(Utc::now().to_rfc3339());
        if let Some(r) = reason_clone {
            ctx["resolution_reason"] = serde_json::json!(r);
        }

        match diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_uuid)))
            .set(user_sessions::context_data.eq(&ctx))
            .execute(&mut db_conn)
        {
            Ok(_) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert("session_id".into(), Dynamic::from(session_uuid.to_string()));
                result.insert("message".into(), Dynamic::from("Conversation resolved"));
                Dynamic::from(result)
            }
            Err(e) => create_error_result(&format!("Update error: {}", e)),
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_set_priority(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "PRIORITY", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let priority = context.eval_expression_tree(&inputs[1])?;
                Ok(set_priority_impl(&state_clone, &session_id, priority))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state;
    engine.register_fn(
        "set_priority",
        move |session_id: &str, priority: &str| -> Dynamic {
            set_priority_impl(
                &state_clone2,
                session_id,
                Dynamic::from(priority.to_string()),
            )
        },
    );
}
pub fn set_priority_impl(state: &Arc<dyn BasicRuntime>, session_id: &str, priority: Dynamic) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };

    let priority_num: i64 = if priority.is_int() {
        priority.as_int().unwrap_or(2)
    } else {
        let p = priority.to_string().to_lowercase();
        match p.as_str() {
            "low" => 1,
            "high" => 3,
            "urgent" => 4,
            _ => 2,
        }
    };

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
        ctx["priority"] = serde_json::json!(priority_num);

        match diesel::update(user_sessions::table.filter(user_sessions::id.eq(session_uuid)))
            .set(user_sessions::context_data.eq(&ctx))
            .execute(&mut db_conn)
        {
            Ok(_) => {
                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(true));
                result.insert("priority".into(), Dynamic::from(priority_num));
                Dynamic::from(result)
            }
            Err(e) => create_error_result(&format!("Update error: {}", e)),
        }
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_get_attendants(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(["GET", "ATTENDANTS"], false, move |_context, _inputs| {
            Ok(get_attendants_impl(&state_clone, None))
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state.clone();
    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "ATTENDANT", "STATS", "$expr$"],
            false,
            move |context, inputs| {
                let filter = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(get_attendants_impl(&state_clone2, Some(filter)))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn("get_attendants", move || -> Dynamic {
        get_attendants_impl(&state, None)
    });
}
