use super::*;

pub fn get_attendants_impl(_state: &Arc<dyn BasicRuntime>, status_filter: Option<String>) -> Dynamic {
    let work_path = botbasic_core::utils::get_work_path();

    let mut attendants = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&work_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.to_string_lossy().ends_with(".gbai") {
                let attendant_path = path.join("attendant.csv");
                if attendant_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&attendant_path) {
                        for line in content.lines().skip(1) {
                            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                            if parts.len() >= 4 {
                                let mut att = Map::new();
                                att.insert("id".into(), Dynamic::from(parts[0].to_string()));
                                att.insert("name".into(), Dynamic::from(parts[1].to_string()));
                                att.insert("channel".into(), Dynamic::from(parts[2].to_string()));
                                att.insert(
                                    "preferences".into(),
                                    Dynamic::from(parts[3].to_string()),
                                );
                                att.insert("status".into(), Dynamic::from("online".to_string()));
                                if parts.len() >= 5 {
                                    att.insert(
                                        "department".into(),
                                        Dynamic::from(parts[4].to_string()),
                                    );
                                }
                                attendants.push(Dynamic::from(att));
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(filter) = status_filter {
        attendants.retain(|att| {
            if let Some(map) = att.clone().try_cast::<Map>() {
                if let Some(status) = map.get("status") {
                    return status.to_string().to_lowercase() == filter.to_lowercase();
                }
            }
            true
        });
    }

    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(true));
    result.insert("count".into(), Dynamic::from(attendants.len() as i64));
    result.insert("items".into(), Dynamic::from(attendants));

    Dynamic::from(result)
}
pub(super) fn register_set_attendant_status(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["SET", "ATTENDANT", "STATUS", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let attendant_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let status = context.eval_expression_tree(&inputs[1])?.to_string();
                let now = Utc::now().to_rfc3339();

let mut conn = state_clone
            .db_pool()
            .get()
            .map_err(|e| format!("DB connection error: {}", e))?;

                let query = diesel::sql_query(
                    "UPDATE attendants SET status = $1, updated_at = $2 WHERE id = $3",
                )
                .bind::<diesel::sql_types::Text, _>(&status)
                .bind::<diesel::sql_types::Text, _>(&now)
                .bind::<diesel::sql_types::Text, _>(&attendant_id);

                let rows_affected = query.execute(&mut *conn).unwrap_or(0);

                info!(
                    "Set attendant {} status to {} (rows_affected={})",
                    attendant_id, status, rows_affected
                );

                let mut result = Map::new();
                result.insert("success".into(), Dynamic::from(rows_affected > 0));
                result.insert("attendant_id".into(), Dynamic::from(attendant_id));
                result.insert("status".into(), Dynamic::from(status));
                result.insert("rows_affected".into(), Dynamic::from(rows_affected as i64));
                Ok(Dynamic::from(result))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub(super) fn register_get_attendant_stats(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state;

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "ATTENDANTS", "WITH", "STATUS", "$expr$"],
            false,
            move |context, inputs| {
                let attendant_id = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(get_attendant_stats_impl(&state_clone, &attendant_id))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn get_attendant_stats_impl(state: &Arc<dyn BasicRuntime>, attendant_id: &str) -> Dynamic {
    let conn = state.db_pool().clone();
    let att_id = attendant_id.to_string();

    let result = std::thread::spawn(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => return create_error_result(&format!("DB error: {}", e)),
        };

        use botbasic_types::schema::user_sessions;

        let today = Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap_or_else(|| today.and_hms_opt(0, 0, 1).unwrap_or_default());

        let resolved_today: i64 = user_sessions::table
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("assigned_to")
                    .eq(&att_id),
            )
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("status")
                    .eq("resolved"),
            )
            .filter(user_sessions::updated_at.ge(today_start))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);

        let active: i64 = user_sessions::table
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("assigned_to")
                    .eq(&att_id),
            )
            .filter(
                user_sessions::context_data
                    .retrieve_as_text("status")
                    .ne("resolved"),
            )
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);

        let mut result = Map::new();
        result.insert("success".into(), Dynamic::from(true));
        result.insert("attendant_id".into(), Dynamic::from(att_id));
        result.insert("resolved_today".into(), Dynamic::from(resolved_today));
        result.insert("active_conversations".into(), Dynamic::from(active));
        Dynamic::from(result)
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_get_tips(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "TIPS", "$expr$", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(get_tips_impl(&state_clone, &session_id, &message))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state;
    engine.register_fn(
        "get_tips",
        move |session_id: &str, message: &str| -> Dynamic {
            get_tips_impl(&state_clone2, session_id, message)
        },
    );
}
pub fn get_tips_impl(_state: &Arc<dyn BasicRuntime>, _session_id: &str, message: &str) -> Dynamic {
    create_fallback_tips(message)
}
pub fn create_fallback_tips(message: &str) -> Dynamic {
    let msg_lower = message.to_lowercase();
    let mut tips = Vec::new();

    if msg_lower.contains("urgent") || msg_lower.contains("asap") || msg_lower.contains("emergency")
    {
        let mut tip = Map::new();
        tip.insert("type".into(), Dynamic::from("warning"));
        tip.insert(
            "content".into(),
            Dynamic::from("Customer indicates urgency - prioritize quick response"),
        );
        tip.insert("priority".into(), Dynamic::from(1_i64));
        tips.push(Dynamic::from(tip));
    }

    if message.contains('?') {
        let mut tip = Map::new();
        tip.insert("type".into(), Dynamic::from("intent"));
        tip.insert(
            "content".into(),
            Dynamic::from("Customer is asking a question - provide clear answer"),
        );
        tip.insert("priority".into(), Dynamic::from(2_i64));
        tips.push(Dynamic::from(tip));
    }

    if msg_lower.contains("problem")
        || msg_lower.contains("issue")
        || msg_lower.contains("not working")
    {
        let mut tip = Map::new();
        tip.insert("type".into(), Dynamic::from("action"));
        tip.insert(
            "content".into(),
            Dynamic::from("Customer reporting issue - acknowledge and gather details"),
        );
        tip.insert("priority".into(), Dynamic::from(2_i64));
        tips.push(Dynamic::from(tip));
    }

    if tips.is_empty() {
        let mut tip = Map::new();
        tip.insert("type".into(), Dynamic::from("general"));
        tip.insert(
            "content".into(),
            Dynamic::from("Read carefully and respond helpfully"),
        );
        tip.insert("priority".into(), Dynamic::from(3_i64));
        tips.push(Dynamic::from(tip));
    }

    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(true));
    result.insert("items".into(), Dynamic::from(tips));
    Dynamic::from(result)
}
pub(super) fn register_polish_message(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["POLISH", "MESSAGE", "$expr$"],
            false,
            move |context, inputs| {
                let message = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(polish_message_impl(&state_clone, &message, "professional"))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state.clone();
    if let Err(e) = engine
        .register_custom_syntax(
            ["POLISH", "MESSAGE", "$expr$", "$expr$"],
            false,
            move |context, inputs| {
                let message = context.eval_expression_tree(&inputs[0])?.to_string();
                let tone = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(polish_message_impl(&state_clone2, &message, &tone))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn("polish_message", move |message: &str| -> Dynamic {
        polish_message_impl(&state, message, "professional")
    });
}
