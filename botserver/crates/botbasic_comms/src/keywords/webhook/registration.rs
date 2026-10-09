use super::*;

pub fn webhook_keyword(state: &Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    let _state_clone = state.clone();

    if let Err(e) = engine
        .register_custom_syntax(["WEBHOOK", "$expr$"], false, move |context, inputs| {
            let endpoint = context.eval_expression_tree(&inputs[0])?.to_string();

            trace!("WEBHOOK registration for endpoint: {}", endpoint);

            Ok(Dynamic::from(format!("webhook:{}", endpoint)))
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}

pub fn execute_webhook_registration(
    conn: &mut diesel::PgConnection,
    endpoint: &str,
    script_name: &str,
    bot_uuid: Uuid,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    trace!(
        "Registering WEBHOOK endpoint: {}, script: {}, bot_id: {:?}",
        endpoint,
        script_name,
        bot_uuid
    );

    use botschema::bots::dsl::bots;
    let bot_exists: bool = diesel::select(diesel::dsl::exists(
        bots.filter(botschema::bots::dsl::id.eq(bot_uuid)),
    ))
    .get_result(conn)?;

    if !bot_exists {
        return Err(format!("Bot with id {} does not exist", bot_uuid).into());
    }

    let clean_endpoint = endpoint
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();

    use botschema::system_automations::dsl::*;

    // branch_id is NOT NULL (migration 6.5.23); resolve it from the bot before
    // inserting so the webhook is branch-scoped.
    let bot_branch = botschema::system_automation_branch_id(conn, bot_uuid)?;

    let new_automation = (
        bot_id.eq(bot_uuid),
        kind.eq(TriggerKind::Webhook as i32),
        target.eq(&clean_endpoint),
        param.eq(script_name),
        is_active.eq(true),
        branch_id.eq(bot_branch),
    );

    let update_result = diesel::update(system_automations)
        .filter(bot_id.eq(bot_uuid))
        .filter(kind.eq(TriggerKind::Webhook as i32))
        .filter(target.eq(&clean_endpoint))
        .set((param.eq(script_name), is_active.eq(true)))
        .execute(&mut *conn)?;

    let result = if update_result == 0 {
        diesel::insert_into(system_automations)
            .values(&new_automation)
            .execute(&mut *conn)?
    } else {
        update_result
    };

    Ok(json!({
        "command": "webhook",
        "endpoint": clean_endpoint,
        "script": script_name,
        "bot_id": bot_uuid.to_string(),
        "rows_affected": result
    }))
}

pub fn remove_webhook_registration(
    conn: &mut diesel::PgConnection,
    endpoint: &str,
    bot_uuid: Uuid,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    use botschema::system_automations::dsl::*;

    let clean_endpoint = endpoint
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase()
        .replace(' ', "-");

    let result = diesel::delete(
        system_automations
            .filter(bot_id.eq(bot_uuid))
            .filter(kind.eq(TriggerKind::Webhook as i32))
            .filter(target.eq(&clean_endpoint)),
    )
    .execute(&mut *conn)?;

    Ok(result)
}

pub type WebhookResult = Vec<(String, String, bool)>;

pub fn get_bot_webhooks(
    conn: &mut diesel::PgConnection,
    bot_uuid: Uuid,
) -> Result<WebhookResult, Box<dyn Error + Send + Sync>> {
    #[derive(QueryableByName)]
    struct WebhookRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        target: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Text)]
        param: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        is_active: bool,
    }

    let results: Vec<WebhookRow> = diesel::sql_query(
        "SELECT target, param, is_active FROM system_automations WHERE bot_id = $1 AND kind = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_uuid)
    .bind::<diesel::sql_types::Int4, _>(TriggerKind::Webhook as i32)
    .load(conn)?;

    Ok(results
        .into_iter()
        .map(|r| (r.target.unwrap_or_default(), r.param, r.is_active))
        .collect())
}

pub fn find_webhook_script(
    conn: &mut diesel::PgConnection,
    bot_uuid: Uuid,
    endpoint: &str,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    use botschema::system_automations::dsl::*;

    let clean_endpoint = endpoint
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase();

    let result: Option<String> = system_automations
        .filter(bot_id.eq(bot_uuid))
        .filter(kind.eq(TriggerKind::Webhook as i32))
        .filter(target.eq(&clean_endpoint))
        .filter(is_active.eq(true))
        .select(param)
        .first(conn)
        .optional()?;

    Ok(result)
}
