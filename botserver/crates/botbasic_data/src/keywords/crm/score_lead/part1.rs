use super::*;

pub fn score_lead_keyword(_state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();

    engine.register_fn("SCORE LEAD", move |lead_data: Map| -> i64 {
        trace!(
            "SCORE LEAD called for user {} with data: {:?}",
            user_clone.user_id,
            lead_data
        );
        calculate_lead_score(&lead_data, None)
    });

    let user_clone2 = user.clone();
    engine.register_fn("score lead", move |lead_data: Map| -> i64 {
        trace!(
            "score lead called for user {} with data: {:?}",
            user_clone2.user_id,
            lead_data
        );
        calculate_lead_score(&lead_data, None)
    });

    let user_clone3 = user;
    engine.register_fn(
        "SCORE LEAD",
        move |lead_data: Map, scoring_rules: Map| -> i64 {
            trace!(
                "SCORE LEAD called for user {} with custom rules",
                user_clone3.user_id
            );
            calculate_lead_score(&lead_data, Some(&scoring_rules))
        },
    );

    debug!("Registered SCORE LEAD keyword");
}
pub fn get_lead_score_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();
    let state_for_db = Arc::clone(&state);

    engine.register_fn("GET LEAD SCORE", move |lead_id: &str| -> i64 {
        trace!(
            "GET LEAD SCORE called for lead {} by user {}",
            lead_id,
            user_clone.user_id
        );

        match get_lead_score_from_db(&state_for_db, lead_id) {
            Some(score) => {
                debug!("Retrieved lead score: {score}");
                score
            }
            None => {
                debug!("Lead not found: {lead_id}, returning 0");
                0
            }
        }
    });

    let user_clone2 = user.clone();
    let state_for_db2 = Arc::clone(&state);

    engine.register_fn("get lead score", move |lead_id: &str| -> i64 {
        trace!(
            "get lead score called for lead {} by user {}",
            lead_id,
            user_clone2.user_id
        );

        get_lead_score_from_db(&state_for_db2, lead_id).unwrap_or(0)
    });

    let user_clone3 = user;
    let state_for_db3 = state;

    engine.register_fn(
        "GET LEAD SCORE",
        move |lead_id: &str, _option: &str| -> Map {
            trace!(
                "GET LEAD SCORE (full) called for lead {} by user {}",
                lead_id,
                user_clone3.user_id
            );

            let mut result = Map::new();
            result.insert("lead_id".into(), Dynamic::from(lead_id.to_string()));

            if let Some(score) = get_lead_score_from_db(&state_for_db3, lead_id) {
                result.insert("score".into(), Dynamic::from(score));
                result.insert("qualified".into(), Dynamic::from(score >= 70));

                let breakdown_score = (score as f64 * 0.3) as i64;
                result.insert("engagement_score".into(), Dynamic::from(breakdown_score));
                result.insert(
                    "demographic_score".into(),
                    Dynamic::from((score as f64 * 0.4) as i64),
                );
                result.insert(
                    "behavioral_score".into(),
                    Dynamic::from((score as f64 * 0.3) as i64),
                );
            } else {
                result.insert("score".into(), Dynamic::from(0_i64));
                result.insert("qualified".into(), Dynamic::from(false));
            }

            result
        },
    );

    debug!("Registered GET LEAD SCORE keyword");
}
pub fn qualify_lead_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();
    let state_for_db = Arc::clone(&state);

    engine.register_fn("QUALIFY LEAD", move |lead_id: &str| -> bool {
        trace!(
            "QUALIFY LEAD called for lead {} by user {}",
            lead_id,
            user_clone.user_id
        );

        if let Some(score) = get_lead_score_from_db(&state_for_db, lead_id) {
            let qualified = score >= 70;
            debug!("Lead {lead_id} qualification: {qualified} (score: {score})");
            qualified
        } else {
            debug!("Lead {lead_id} not found");
            false
        }
    });

    let user_clone2 = user.clone();
    let state_for_db2 = Arc::clone(&state);

    engine.register_fn("qualify lead", move |lead_id: &str| -> bool {
        trace!(
            "qualify lead called for lead {} by user {}",
            lead_id,
            user_clone2.user_id
        );
        get_lead_score_from_db(&state_for_db2, lead_id).is_some_and(|s| s >= 70)
    });

    let user_clone3 = user.clone();
    let state_for_db3 = Arc::clone(&state);

    engine.register_fn(
        "QUALIFY LEAD",
        move |lead_id: &str, threshold: i64| -> bool {
            trace!(
                "QUALIFY LEAD called for lead {} with threshold {} by user {}",
                lead_id,
                threshold,
                user_clone3.user_id
            );

            if let Some(score) = get_lead_score_from_db(&state_for_db3, lead_id) {
                let qualified = score >= threshold;
                debug!("Lead {lead_id} qualified: {qualified} against threshold {threshold}");
                qualified
            } else {
                false
            }
        },
    );

    let user_clone4 = user;
    let state_for_db4 = state;

    engine.register_fn(
        "IS QUALIFIED",
        move |lead_id: &str, threshold: i64| -> bool {
            trace!(
                "IS QUALIFIED called for lead {} with threshold {} by user {}",
                lead_id,
                threshold,
                user_clone4.user_id
            );
            get_lead_score_from_db(&state_for_db4, lead_id).is_some_and(|s| s >= threshold)
        },
    );

    debug!("Registered QUALIFY LEAD keyword");
}
pub fn update_lead_score_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_clone = user.clone();
    let state_for_db = Arc::clone(&state);

    engine.register_fn(
        "UPDATE LEAD SCORE",
        move |lead_id: &str, adjustment: i64| -> i64 {
            trace!(
                "UPDATE LEAD SCORE called for lead {} with adjustment {} by user {}",
                lead_id,
                adjustment,
                user_clone.user_id
            );

            if let Some(current) = get_lead_score_from_db(&state_for_db, lead_id) {
                let score = (current + adjustment).clamp(0, 100);
                update_lead_score_in_db(&state_for_db, lead_id, score);
                info!("Updated lead {lead_id} score from {current} to {score} (adjustment: {adjustment})");
                score
            } else {
                let score = adjustment.clamp(0, 100);
                update_lead_score_in_db(&state_for_db, lead_id, score);
                info!("Created lead {lead_id} with initial score {score}");
                score
            }
        },
    );

    let user_clone2 = user.clone();
    let state_for_db2 = Arc::clone(&state);

    engine.register_fn(
        "update lead score",
        move |lead_id: &str, adjustment: i64| -> i64 {
            trace!(
                "update lead score called for lead {} with adjustment {} by user {}",
                lead_id,
                adjustment,
                user_clone2.user_id
            );

            if let Some(current) = get_lead_score_from_db(&state_for_db2, lead_id) {
                let score = (current + adjustment).clamp(0, 100);
                update_lead_score_in_db(&state_for_db2, lead_id, score);
                score
            } else {
                let score = adjustment.clamp(0, 100);
                update_lead_score_in_db(&state_for_db2, lead_id, score);
                score
            }
        },
    );

    let user_clone3 = user.clone();
    let state_for_db3 = Arc::clone(&state);

    engine.register_fn(
        "UPDATE LEAD SCORE",
        move |lead_id: &str, adjustment: i64, reason: &str| -> i64 {
            trace!(
                "UPDATE LEAD SCORE (with reason) called for lead {} with adjustment {} reason '{}' by user {}",
                lead_id,
                adjustment,
                reason,
                user_clone3.user_id
            );

            if let Some(current) = get_lead_score_from_db(&state_for_db3, lead_id) {
                let score = (current + adjustment).clamp(0, 100);
                update_lead_score_in_db(&state_for_db3, lead_id, score);
                info!("Score adjustment for lead {lead_id}: {current} -> {score} | Reason: {reason}");
                score
            } else {
                let score = adjustment.clamp(0, 100);
                update_lead_score_in_db(&state_for_db3, lead_id, score);
                info!("Created lead {lead_id} with score {score} | Reason: {reason}");
                score
            }
        },
    );

    let user_clone4 = user;
    let state_for_db4 = state;

    engine.register_fn("SET LEAD SCORE", move |lead_id: &str, score: i64| -> i64 {
        trace!(
            "SET LEAD SCORE called for lead {} with score {} by user {}",
            lead_id,
            score,
            user_clone4.user_id
        );

        let clamped_score = score.clamp(0, 100);
        update_lead_score_in_db(&state_for_db4, lead_id, clamped_score);
        info!("Set lead {lead_id} score to {clamped_score}");
        clamped_score
    });

    debug!("Registered UPDATE LEAD SCORE keyword");
}
