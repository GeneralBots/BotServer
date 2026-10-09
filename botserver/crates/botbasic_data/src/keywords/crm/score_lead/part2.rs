use super::*;

pub fn ai_score_lead_keyword(_state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();

    engine.register_fn("AI SCORE LEAD", move |lead_data: Map| -> Map {
        trace!(
            "AI SCORE LEAD called for user {} with data: {:?}",
            user_clone.user_id,
            lead_data
        );

        let base_score = calculate_lead_score(&lead_data, None);
        let mut result = Map::new();

        result.insert("score".into(), Dynamic::from(base_score));
        result.insert("confidence".into(), Dynamic::from(0.85_f64));
        result.insert(
            "recommendation".into(),
            Dynamic::from(get_recommendation(base_score)),
        );
        result.insert(
            "priority".into(),
            Dynamic::from(determine_priority(base_score)),
        );
        result.insert(
            "suggested_action".into(),
            Dynamic::from(get_suggested_action(base_score)),
        );

        debug!("AI Score Lead result - score: {base_score}, confidence: 0.85");
        result
    });

    let user_clone2 = user.clone();

    engine.register_fn("ai score lead", move |lead_data: Map| -> Map {
        trace!(
            "ai score lead called for user {} with data: {:?}",
            user_clone2.user_id,
            lead_data
        );

        let base_score = calculate_lead_score(&lead_data, None);
        let mut result = Map::new();
        result.insert("score".into(), Dynamic::from(base_score));
        result.insert("confidence".into(), Dynamic::from(0.85_f64));
        result.insert(
            "priority".into(),
            Dynamic::from(determine_priority(base_score)),
        );
        result
    });

    let user_clone3 = user;

    engine.register_fn(
        "AI SCORE LEAD",
        move |lead_data: Map, _context: &str| -> Map {
            trace!(
                "AI SCORE LEAD with context called for user {} with data: {:?}",
                user_clone3.user_id,
                lead_data
            );

            let base_score = calculate_lead_score(&lead_data, None);
            let mut result = Map::new();
            result.insert("score".into(), Dynamic::from(base_score));
            result.insert("confidence".into(), Dynamic::from(0.90_f64));
            result.insert(
                "priority".into(),
                Dynamic::from(determine_priority(base_score)),
            );
            result.insert(
                "recommendation".into(),
                Dynamic::from(get_recommendation(base_score)),
            );

            result
        },
    );

    debug!("Registered AI SCORE LEAD keyword");
}
pub(super) fn calculate_lead_score(lead_data: &Map, custom_rules: Option<&Map>) -> i64 {
    let mut score: i64 = 0;

    if let Some(title) = lead_data.get("job_title") {
        let title_lower = title.to_string().to_lowercase();
        match title_lower.as_str() {
            t if t.contains("cto") || t.contains("ceo") => score += 30,
            t if t.contains("cfo") || t.contains("director") => score += 25,
            t if t.contains("vp") || t.contains("vice") => score += 20,
            t if t.contains("manager") || t.contains("lead") => score += 15,
            _ => score += 5,
        }
    }

    if let Some(size_val) = lead_data.get("company_size") {
        if let Ok(size) = size_val.as_int() {
            if size > 1000 {
                score += 20;
            } else if size > 500 {
                score += 15;
            } else if size > 100 {
                score += 10;
            } else if size > 0 {
                score += 5;
            }
        }
    }

    if let Some(email_val) = lead_data.get("email") {
        let email = email_val.to_string();
        if email.contains('@') {
            score += 10;
            if !email.ends_with("@gmail.com") && !email.ends_with("@yahoo.com") {
                score += 10;
            }
        }
    }

    if let Some(budget_val) = lead_data.get("budget") {
        if let Ok(budget) = budget_val.as_int() {
            if budget > 100_000 {
                score += 25;
            } else if budget > 50000 {
                score += 20;
            } else if budget > 10000 {
                score += 15;
            } else if budget > 0 {
                score += 10;
            }
        }
    }

    if let Some(industry_val) = lead_data.get("industry") {
        let industry_lower = industry_val.to_string().to_lowercase();
        if industry_lower.contains("tech")
            || industry_lower.contains("software")
            || industry_lower.contains("finance")
            || industry_lower.contains("banking")
        {
            score += 15;
        } else if industry_lower.contains("healthcare") || industry_lower.contains("pharma") {
            score += 10;
        }
    }

    if let Some(rules) = custom_rules {
        if let Some(weight_val) = rules.get("weight") {
            if let Ok(weight_multiplier) = weight_val.as_int() {
                score = (score as f64 * (weight_multiplier as f64 / 100.0)) as i64;
            }
        }
        if let Some(bonus_val) = rules.get("bonus") {
            if let Ok(bonus) = bonus_val.as_int() {
                score += bonus;
            }
        }
    }

    score.clamp(0, 100)
}
pub(super) fn determine_priority(score: i64) -> String {
    match score {
        90..=100 => "CRITICAL".to_string(),
        70..=89 => "HIGH".to_string(),
        50..=69 => "MEDIUM".to_string(),
        30..=49 => "LOW".to_string(),
        _ => "MINIMAL".to_string(),
    }
}
pub(super) fn get_recommendation(score: i64) -> String {
    match score {
        90..=100 => "Contact immediately - Schedule meeting within 24 hours".to_string(),
        70..=89 => "Contact within 48 hours - Prepare tailored proposal".to_string(),
        50..=69 => "Nurture campaign - Send valuable content".to_string(),
        30..=49 => "Keep in pipeline - Occasional touchpoints".to_string(),
        _ => "Monitor for engagement signals".to_string(),
    }
}
pub(super) fn get_suggested_action(score: i64) -> String {
    match score {
        90..=100 => "Call and schedule demo".to_string(),
        70..=89 => "Send personalized email with case study".to_string(),
        50..=69 => "Add to drip campaign".to_string(),
        30..=49 => "Request more information".to_string(),
        _ => "Monitor for budget signals".to_string(),
    }
}
pub(super) fn get_lead_score_from_db(state: &Arc<dyn BasicRuntime>, lead_id: &str) -> Option<i64> {
    let memory_key = format!("lead_score:{lead_id}");

    let mut conn = match state.db_pool().get() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to get database connection for lead score lookup: {e}");
            return None;
        }
    };

    let result = bot_memories::table
        .filter(bot_memories::key.eq(&memory_key))
        .select(bot_memories::value)
        .first::<String>(&mut conn)
        .optional();

    match result {
        Ok(Some(value)) => match value.parse::<i64>() {
            Ok(score) => {
                debug!("Retrieved lead score {score} for lead {lead_id}");
                Some(score)
            }
            Err(e) => {
                log::error!("Failed to parse lead score '{value}' for lead {lead_id}: {e}");
                None
            }
        },
        Ok(None) => {
            debug!("No lead score found for lead {lead_id}");
            None
        }
        Err(e) => {
            log::error!("Database error retrieving lead score for {lead_id}: {e}");
            None
        }
    }
}
pub(super) fn update_lead_score_in_db(state: &Arc<dyn BasicRuntime>, lead_id: &str, score: i64) {
    let memory_key = format!("lead_score:{lead_id}");
    let score_value = score.to_string();
    let now = Utc::now();

    let mut conn = match state.db_pool().get() {
        Ok(c) => c,
        Err(e) => {
            log::error!("Failed to get database connection for lead score update: {e}");
            return;
        }
    };

    let existing = bot_memories::table
        .filter(bot_memories::key.eq(&memory_key))
        .select(bot_memories::id)
        .first::<Uuid>(&mut conn)
        .optional();

    match existing {
        Ok(Some(existing_id)) => {
            let update_result = diesel::update(bot_memories::table.find(existing_id))
                .set((
                    bot_memories::value.eq(&score_value),
                    bot_memories::updated_at.eq(now),
                ))
                .execute(&mut conn);

            match update_result {
                Ok(_) => {
                    info!("Updated lead score to {score} for lead {lead_id}");
                }
                Err(e) => {
                    log::error!("Failed to update lead score for {lead_id}: {e}");
                }
            }
        }
        Ok(None) => {
            let new_id = Uuid::new_v4();
            let bot_id = Uuid::nil();

            let insert_result = diesel::insert_into(bot_memories::table)
                .values((
                    bot_memories::id.eq(new_id),
                    bot_memories::bot_id.eq(bot_id),
                    bot_memories::key.eq(&memory_key),
                    bot_memories::value.eq(&score_value),
                    bot_memories::created_at.eq(now),
                    bot_memories::updated_at.eq(now),
                ))
                .execute(&mut conn);

            match insert_result {
                Ok(_) => {
                    info!("Inserted new lead score {score} for lead {lead_id}");
                }
                Err(e) => {
                    log::error!("Failed to insert lead score for {lead_id}: {e}");
                }
            }
        }
        Err(e) => {
            log::error!("Database error checking existing lead score for {lead_id}: {e}");
        }
    }
}
