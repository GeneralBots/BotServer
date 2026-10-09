use super::*;

pub fn on_email_keyword(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_on_email(state, user.clone(), engine);
    register_on_email_from(state, user.clone(), engine);
    register_on_email_subject(state, user, engine);
}

pub(super) fn register_on_email(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let bot_id = user.bot_id;

    if let Err(e) = engine
        .register_custom_syntax(["ON", "EMAIL", "$string$"], true, move |context, inputs| {
            let email_address = context
                .eval_expression_tree(&inputs[0])?
                .to_string()
                .trim_matches('"')
                .to_string();

            trace!("ON EMAIL '{}' for bot: {}", email_address, bot_id);

            let script_name = format!(
                "on_email_{}.rhai",
                email_address.replace('@', "_at_").replace('.', "_")
            );

            let mut conn = state_clone
                .db_pool()
                .get()
                .map_err(|e| format!("DB error: {}", e))?;

            let result =
                execute_on_email(&mut conn, bot_id, &email_address, &script_name, None, None)
                    .map_err(|e| format!("DB error: {}", e))?;

            if let Some(rows_affected) = result.get("rows_affected") {
                info!(
                    "Email monitor registered for '{}' on bot {}",
                    email_address, bot_id
                );
                Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
            } else {
                Err("Failed to register email monitor".into())
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}

pub(super) fn register_on_email_from(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let bot_id = user.bot_id;

    if let Err(e) = engine
        .register_custom_syntax(
            ["ON", "EMAIL", "$string$", "FROM", "$string$"],
            true,
            move |context, inputs| {
                let email_address = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                let filter_from = context
                    .eval_expression_tree(&inputs[1])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                trace!(
                    "ON EMAIL '{}' FROM '{}' for bot: {}",
                    email_address,
                    filter_from,
                    bot_id
                );

                let script_name = format!(
                    "on_email_{}_from_{}.rhai",
                    email_address.replace('@', "_at_").replace('.', "_"),
                    filter_from.replace('@', "_at_").replace('.', "_")
                );

                let mut conn = state_clone
                    .db_pool()
                    .get()
                    .map_err(|e| format!("DB error: {}", e))?;

                let result = execute_on_email(
                    &mut conn,
                    bot_id,
                    &email_address,
                    &script_name,
                    Some(&filter_from),
                    None,
                )
                .map_err(|e| format!("DB error: {}", e))?;

                if let Some(rows_affected) = result.get("rows_affected") {
                    info!(
                        "Email monitor registered for '{}' from '{}' on bot {}",
                        email_address, filter_from, bot_id
                    );
                    Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                } else {
                    Err("Failed to register email monitor".into())
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}

pub(super) fn register_on_email_subject(state: &Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let bot_id = user.bot_id;

    if let Err(e) = engine
        .register_custom_syntax(
            ["ON", "EMAIL", "$string$", "SUBJECT", "$string$"],
            true,
            move |context, inputs| {
                let email_address = context
                    .eval_expression_tree(&inputs[0])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                let filter_subject = context
                    .eval_expression_tree(&inputs[1])?
                    .to_string()
                    .trim_matches('"')
                    .to_string();

                trace!(
                    "ON EMAIL '{}' SUBJECT '{}' for bot: {}",
                    email_address,
                    filter_subject,
                    bot_id
                );

                let script_name = format!(
                    "on_email_{}_subject.rhai",
                    email_address.replace('@', "_at_").replace('.', "_")
                );

                let mut conn = state_clone
                    .db_pool()
                    .get()
                    .map_err(|e| format!("DB error: {}", e))?;

                let result = execute_on_email(
                    &mut conn,
                    bot_id,
                    &email_address,
                    &script_name,
                    None,
                    Some(&filter_subject),
                )
                .map_err(|e| format!("DB error: {}", e))?;

                if let Some(rows_affected) = result.get("rows_affected") {
                    info!(
                        "Email monitor registered for '{}' with subject filter '{}' on bot {}",
                        email_address, filter_subject, bot_id
                    );
                    Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                } else {
                    Err("Failed to register email monitor".into())
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
