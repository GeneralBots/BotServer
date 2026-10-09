use super::*;

/// Query a single text field from a table using the same filter/WHERE clause as UPDATE.
/// Returns None if no matching row is found.
pub(super) fn query_table_field(
    conn: &mut diesel::PgConnection,
    table: &str,
    field: &str,
    filter: &str,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let where_clause = parse_filter_clause(filter)?;
    let query = format!(
        "SELECT {} FROM {} WHERE {} LIMIT 1",
        sanitize_identifier(field),
        sanitize_identifier(table),
        where_clause
    );

    #[derive(QueryableByName)]
    struct FieldResult {
        #[diesel(sql_type = Text)]
        value: String,
    }

    match sql_query(&query).get_result::<FieldResult>(conn) {
        Ok(row) => Ok(Some(row.value)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
/// Query a single text field from a table by id.
pub(super) fn query_table_field_by_id(
    conn: &mut diesel::PgConnection,
    table: &str,
    field: &str,
    id: &str,
) -> Result<Option<String>, Box<dyn Error + Send + Sync>> {
    let query = format!(
        "SELECT {} FROM {} WHERE id = '{}' LIMIT 1",
        sanitize_identifier(field),
        sanitize_identifier(table),
        sanitize_sql_value(id)
    );

    #[derive(QueryableByName)]
    struct FieldResult {
        #[diesel(sql_type = Text)]
        value: String,
    }

    match sql_query(&query).get_result::<FieldResult>(conn) {
        Ok(row) => Ok(Some(row.value)),
        Err(diesel::result::Error::NotFound) => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub fn register_data_operations(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_save_keyword(state.clone(), user.clone(), engine);
    register_insert_keyword(state.clone(), user.clone(), engine);
    register_update_keyword(state.clone(), user.clone(), engine);
    register_delete_keyword(state.clone(), user.clone(), engine);
    register_merge_keyword(state.clone(), user.clone(), engine);
    register_fill_keyword(state.clone(), user.clone(), engine);
    register_map_keyword(state.clone(), user.clone(), engine);
    register_filter_keyword(state.clone(), user.clone(), engine);
    register_aggregate_keyword(state.clone(), user.clone(), engine);
    register_join_keyword(state.clone(), user.clone(), engine);
    register_pivot_keyword(state.clone(), user.clone(), engine);
    register_group_by_keyword(state, user, engine);
}
pub fn register_save_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let user_roles = UserRoles::from_user_session(&user);

    // SAVE with variable arguments: SAVE "table", id, field1, field2, ...
    // Each pattern: table + id + (1 to 64 fields)
    // Minimum: table + id + 1 field = 4 expressions total
    register_save_variants(state.clone(), user.clone(), user_roles, engine);
}
pub(super) fn register_save_variants(state: Arc<dyn BasicRuntime>, user: UserSession, user_roles: UserRoles, engine: &mut Engine) {
    // Rhai 1.25.x stores custom syntax in a BTreeMap keyed by the first token,
    // so only ONE pattern per starting keyword survives.
    // The compiler converts positional SAVE → SAVE table, data (2-arg)
    // and SAVE () TO WHERE → SAVE "table", data (2-arg with id embedded in data).
    // The handler checks if data contains an "id" field to decide insert vs upsert.
    {
        let state_clone = Arc::clone(&state);
        let user_roles_clone = user_roles.clone();
        let user_clone = user.clone();
        if let Err(e) = engine
            .register_custom_syntax(
                ["SAVE", "$expr$", ",", "$expr$"],
                false,
                move |context, inputs| {
                    let table = context.eval_expression_tree(&inputs[0])?.to_string();
                    let data = context.eval_expression_tree(&inputs[1])?;

                    trace!("SAVE: table={}", table);

                    // Use bot database if available (tables are per-bot), fallback to main
                    let pool_opt = state_clone
                        .bot_database_manager()
                        .get_bot_pool(user_clone.bot_id);
                    let mut conn = if let Some(pool) = pool_opt {
                        pool.get().map_err(|e| format!("Bot DB error: {}", e))?
                    } else {
                        state_clone
                            .db_pool()
                            .get()
                            .map_err(|e| format!("DB error: {}", e))?
                    };

                    if let Err(e) =
                        check_table_access(&mut conn, &table, &user_roles_clone, AccessType::Write)
                    {
                        warn!("SAVE access denied: {}", e);
                        return Err(e.into());
                    }

                    // Check if data has an "id" field to decide insert vs upsert
                    let data_map = dynamic_to_map(&data);
                    let has_id = data_map.contains_key("id");
                    let id_value = data_map.get("id").map(|v| v.to_string()).unwrap_or_default();

                    let result = if has_id {
                        // SAVE () TO WHERE with id → upsert
                        execute_save(&mut conn, &table, &Dynamic::from(id_value.clone()), &data)
                            .map_err(|e| format!("SAVE error: {}", e))?
                    } else {
                        // Positional SAVE without id → insert
                        execute_insert(&mut conn, &table, &data)
                            .map_err(|e| format!("SAVE error: {}", e))?
                    };

                    let rid = if id_value.is_empty() {
                        result.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    } else {
                        id_value.clone()
                    };

                    if has_id && !rid.is_empty() {
                        fire_table_triggers(
                            &mut conn,
                            &state_clone,
                            &user_clone,
                            &table,
                            1,
                            Some(rid),
                            None,
                            None,
                        );
                    }

                    if !has_id {
                        fire_table_triggers(
                            &mut conn,
                            &state_clone,
                            &user_clone,
                            &table,
                            2,
                            None,
                            None,
                            None,
                        );
                    }

                    Ok(json_value_to_dynamic(&result))
                },
            )
        {
            log::error!("Failed to register the custom syntax: {e}");
        }
    }
}
pub fn register_insert_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let user_clone = user.clone();
    let user_roles = UserRoles::from_user_session(&user);

    if let Err(e) = engine
        .register_custom_syntax(
            ["INSERT", "$expr$", ",", "$expr$"],
            true,
            move |context, inputs| {
                let table = context.eval_expression_tree(&inputs[0])?.to_string();
                let data = context.eval_expression_tree(&inputs[1])?;

                trace!("INSERT into table: {}", table);

                // Get bot's database connection instead of main connection
                let bot_pool = state_clone
.bot_database_manager()
.get_bot_pool(user_clone.bot_id);

let mut conn = match bot_pool {
            Some(pool) => pool.get().map_err(|e| format!("Bot DB error: {}", e))?,
            None => state_clone
                .db_pool()
                .get()
                .map_err(|e| format!("DB error: {}", e))?,
        };

                // Check write access
                if let Err(e) =
                    check_table_access(&mut conn, &table, &user_roles, AccessType::Write)
                {
                    warn!("INSERT access denied: {}", e);
                    return Err(e.into());
                }

                let result = execute_insert(&mut conn, &table, &data)
                    .map_err(|e| format!("INSERT error: {}", e))?;

                fire_table_triggers(&mut conn, &state_clone, &user_clone, &table, 2, None, None, None);

                Ok(json_value_to_dynamic(&result))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
pub fn register_update_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();
    let user_clone = user.clone();
    let user_roles = UserRoles::from_user_session(&user);

    if let Err(e) = engine
        .register_custom_syntax(
            ["UPDATE", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let table = context.eval_expression_tree(&inputs[0])?.to_string();
                let filter = context.eval_expression_tree(&inputs[1])?.to_string();
                let data = context.eval_expression_tree(&inputs[2])?;

                trace!("UPDATE table: {}, filter: {}", table, filter);

                // Use bot database if available, fallback to main
                let pool_opt = state_clone
                    .bot_database_manager()
                    .get_bot_pool(user_clone.bot_id);
                let mut conn = if let Some(pool) = pool_opt {
                    pool.get().map_err(|e| format!("Bot DB error: {}", e))?
                } else {
                    state_clone
                        .db_pool()
                        .get()
                        .map_err(|e| format!("DB error: {}", e))?
                };

                // Check write access
                if let Err(e) =
                    check_table_access(&mut conn, &table, &user_roles, AccessType::Write)
                {
                    warn!("UPDATE access denied: {}", e);
                    return Err(e.into());
                }

                // Capture old status BEFORE the UPDATE (for trigger context)
                let old_status = query_table_field(&mut conn, &table, "status", &filter)
                    .map_err(|e| format!("Failed to query old status: {}", e))?;

                let (row_count, ids) = execute_update(&mut conn, &table, &filter, &data)
                    .map_err(|e| format!("UPDATE error: {}", e))?;

                // Capture new status AFTER the UPDATE (from first updated record)
                let new_status = ids
                    .first()
                    .and_then(|id| query_table_field_by_id(&mut conn, &table, "status", id).ok())
                    .flatten();

                fire_table_triggers(
                    &mut conn,
                    &state_clone,
                    &user_clone,
                    &table,
                    1,
                    ids.first().cloned(),
                    old_status,
                    new_status,
                );

                Ok(Dynamic::from(row_count))
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}
