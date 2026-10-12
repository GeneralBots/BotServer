use super::*;

pub fn polish_message_impl(_state: &Arc<dyn BasicRuntime>, message: &str, _tone: &str) -> Dynamic {
    let mut polished = message.to_string();

    polished = polished
        .replace("thx", "Thank you")
        .replace("u ", "you ")
        .replace(" u", " you")
        .replace("ur ", "your ")
        .replace("ill ", "I'll ")
        .replace("dont ", "don't ")
        .replace("cant ", "can't ")
        .replace("wont ", "won't ")
        .replace("im ", "I'm ")
        .replace("ive ", "I've ");

    polished = botlib::text::capitalize_first(&polished);

    if !polished.ends_with('.') && !polished.ends_with('!') && !polished.ends_with('?') {
        polished.push('.');
    }

    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(true));
    result.insert("original".into(), Dynamic::from(message.to_string()));
    result.insert("text".into(), Dynamic::from(polished.clone()));
    result.insert("polished".into(), Dynamic::from(polished));
    Dynamic::from(result)
}
pub(super) fn register_get_smart_replies(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "SMART", "REPLIES", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(get_smart_replies_impl(&state_clone, &session_id))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn("get_smart_replies", move |session_id: &str| -> Dynamic {
        get_smart_replies_impl(&state, session_id)
    });
}
pub fn get_smart_replies_impl(_state: &Arc<dyn BasicRuntime>, _session_id: &str) -> Dynamic {
    let mut replies = Vec::new();

    let mut reply1 = Map::new();
    reply1.insert(
        "text".into(),
        Dynamic::from("Thank you for reaching out! I'd be happy to help you with that."),
    );
    reply1.insert("tone".into(), Dynamic::from("friendly"));
    reply1.insert("category".into(), Dynamic::from("greeting"));
    replies.push(Dynamic::from(reply1));

    let mut reply2 = Map::new();
    reply2.insert(
        "text".into(),
        Dynamic::from("I understand your concern. Let me look into this for you right away."),
    );
    reply2.insert("tone".into(), Dynamic::from("empathetic"));
    reply2.insert("category".into(), Dynamic::from("acknowledgment"));
    replies.push(Dynamic::from(reply2));

    let mut reply3 = Map::new();
    reply3.insert(
        "text".into(),
        Dynamic::from("Is there anything else I can help you with today?"),
    );
    reply3.insert("tone".into(), Dynamic::from("professional"));
    reply3.insert("category".into(), Dynamic::from("follow_up"));
    replies.push(Dynamic::from(reply3));

    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(true));
    result.insert("items".into(), Dynamic::from(replies));
    Dynamic::from(result)
}
pub(super) fn register_get_summary(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["GET", "SUMMARY", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                Ok(get_summary_impl(&state_clone, &session_id))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn("get_summary", move |session_id: &str| -> Dynamic {
        get_summary_impl(&state, session_id)
    });
}
pub fn get_summary_impl(state: &Arc<dyn BasicRuntime>, session_id: &str) -> Dynamic {
    let conn = state.db_pool().clone();
    let Ok(session_uuid) = Uuid::parse_str(session_id) else {
        return create_error_result("Invalid session ID");
    };

    let result = std::thread::spawn(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => return create_error_result(&format!("DB error: {}", e)),
        };

        use botbasic_types::schema::message_history;

        let message_count: i64 = message_history::table
            .filter(message_history::session_id.eq(session_uuid))
            .count()
            .get_result(&mut db_conn)
            .unwrap_or(0);

        let mut result = Map::new();
        result.insert("success".into(), Dynamic::from(true));
        result.insert("session_id".into(), Dynamic::from(session_uuid.to_string()));
        result.insert("message_count".into(), Dynamic::from(message_count));
        result.insert(
            "brief".into(),
            Dynamic::from(format!("Conversation with {} messages", message_count)),
        );
        result.insert("key_points".into(), Dynamic::from(Vec::<Dynamic>::new()));
        result.insert("sentiment_trend".into(), Dynamic::from("neutral"));
        Dynamic::from(result)
    })
    .join()
    .unwrap_or_else(|_| create_error_result("Thread panic"));

    result
}
pub(super) fn register_analyze_sentiment(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["ANALYZE", "SENTIMENT", "$expr$", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let message = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(analyze_sentiment_impl(&state_clone, &session_id, &message))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = state;
    engine.register_fn(
        "analyze_sentiment",
        move |session_id: &str, message: &str| -> Dynamic {
            analyze_sentiment_impl(&state_clone2, session_id, message)
        },
    );
}
pub fn analyze_sentiment_impl(_state: &Arc<dyn BasicRuntime>, _session_id: &str, message: &str) -> Dynamic {
    let msg_lower = message.to_lowercase();

    let positive_words = [
        "thank",
        "great",
        "perfect",
        "awesome",
        "excellent",
        "good",
        "happy",
        "love",
    ];
    let negative_words = [
        "angry",
        "frustrated",
        "terrible",
        "awful",
        "horrible",
        "hate",
        "disappointed",
        "problem",
        "issue",
    ];
    let urgent_words = [
        "urgent",
        "asap",
        "immediately",
        "emergency",
        "now",
        "critical",
    ];

    let positive_count = positive_words
        .iter()
        .filter(|w| msg_lower.contains(*w))
        .count();
    let negative_count = negative_words
        .iter()
        .filter(|w| msg_lower.contains(*w))
        .count();
    let urgent_count = urgent_words
        .iter()
        .filter(|w| msg_lower.contains(*w))
        .count();

    let (overall, score, emoji) = match positive_count.cmp(&negative_count) {
        std::cmp::Ordering::Greater => ("positive", 0.5, "😊"),
        std::cmp::Ordering::Less => ("negative", -0.5, "😟"),
        std::cmp::Ordering::Equal => ("neutral", 0.0, "😐"),
    };

    let escalation_risk = if negative_count >= 3 {
        "high"
    } else if negative_count >= 1 {
        "medium"
    } else {
        "low"
    };

    let urgency = if urgent_count >= 2 {
        "urgent"
    } else if urgent_count >= 1 {
        "high"
    } else {
        "normal"
    };

    let mut result = Map::new();
    result.insert("success".into(), Dynamic::from(true));
    result.insert("overall".into(), Dynamic::from(overall));
    result.insert("score".into(), Dynamic::from(score));
    result.insert("emoji".into(), Dynamic::from(emoji));
    result.insert("escalation_risk".into(), Dynamic::from(escalation_risk));
    result.insert("urgency".into(), Dynamic::from(urgency));
    Dynamic::from(result)
}
pub(super) fn register_tag_conversation(state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["TAG", "CONVERSATION", "$expr$", "WITH", "$expr$"],
            false,
            move |context, inputs| {
                let session_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let tag = context.eval_expression_tree(&inputs[1])?.to_string();
                Ok(tag_conversation_impl(&state_clone, &session_id, vec![tag]))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    engine.register_fn(
        "tag_conversation",
        move |session_id: &str, tag: &str| -> Dynamic {
            tag_conversation_impl(&state, session_id, vec![tag.to_string()])
        },
    );
}
