use super::*;

pub fn register_pivot_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["PIVOT", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let data = context.eval_expression_tree(&inputs[0])?;
                let row_field = context.eval_expression_tree(&inputs[1])?.to_string();
                let value_field = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!("PIVOT on row: {}, value: {}", row_field, value_field);

                let result = execute_pivot(&data, &row_field, &value_field)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub fn register_group_by_keyword(_state: Arc<dyn BasicRuntime>, _user: UserSession, engine: &mut Engine) {
    if let Err(e) = engine
        .register_custom_syntax(
            ["GROUP_BY", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let data = context.eval_expression_tree(&inputs[0])?;
                let field = context.eval_expression_tree(&inputs[1])?.to_string();

                trace!("GROUP_BY field: {}", field);

                let result = execute_group_by(&data, &field)?;

                Ok(result)
            },
        )
{
    log::error!("Failed to register the custom syntax: {e}");
}
}
pub(super) fn execute_save(
    conn: &mut diesel::PgConnection,
    table: &str,
    id: &Dynamic,
    data: &Dynamic,
) -> Result<Value, Box<dyn Error + Send + Sync>> {
    let data_map = dynamic_to_map(data);
    let id_value = id.to_string();

    let mut columns: Vec<String> = vec!["id".to_string()];
    let mut values: Vec<String> = vec![format!("'{}'", sanitize_sql_value(&id_value))];
    let mut update_sets: Vec<String> = Vec::new();

    for (key, value) in &data_map {
        if key == "id" {
            continue;
        }
        let sanitized_key = sanitize_identifier(key).to_lowercase();
        let value_str = value.to_string();
        let converted_value = convert_date_to_iso_format(&value_str);
        let sanitized_value = if converted_value.trim().is_empty() {
            "NULL".to_string()
        } else {
            format!("'{}'", sanitize_sql_value(&converted_value))
        };
        columns.push(sanitized_key.clone());
        values.push(sanitized_value.clone());
        update_sets.push(format!("{} = {}", sanitized_key, sanitized_value));
    }

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT (id) DO UPDATE SET {}",
        sanitize_identifier(table).to_lowercase(),
        columns.join(", "),
        values.join(", "),
        update_sets.join(", ")
    );

    trace!("Executing SAVE query: {}", query);

    let result = sql_query(&query).execute(conn).map_err(|e| {
        log::error!("SAVE SQL error: {}", e);
        e.to_string()
    })?;

    Ok(json!({
        "command": "save",
        "table": table,
        "id": id_value,
        "rows_affected": result
    }))
}
/// The table change a trigger fires for.
pub(super) struct TableTrigger<'a> {
    pub conn: &'a mut diesel::PgConnection,
    pub state: &'a Arc<dyn BasicRuntime>,
    pub user: &'a botbasic_types::UserSession,
    pub table: &'a str,
    pub trigger_kind_val: i32,
    pub record_id: Option<String>,
    pub old_status: Option<String>,
    pub new_status: Option<String>,
}

pub(super) fn fire_table_triggers(trigger: TableTrigger<'_>) {
    let TableTrigger {
        conn,
        state,
        user,
        table,
        trigger_kind_val,
        record_id,
        old_status,
        new_status,
    } = trigger;
    // Use main database connection to query system_automations (metadata)
    let mut meta_conn = match state.db_pool().get() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Failed to get metadata connection for triggers: {}", e);
            return;
        }
    };
    let query = "SELECT param FROM system_automations WHERE bot_id = $1::uuid AND kind = $2 AND target = $3 AND is_active = true";
    let triggers: Vec<String> = match diesel::sql_query(query)
        .bind::<diesel::sql_types::Text, _>(&user.bot_id.to_string())
        .bind::<diesel::sql_types::Integer, _>(trigger_kind_val)
        .bind::<diesel::sql_types::Text, _>(table)
        .load::<TriggerRow>(&mut meta_conn)
    {
        Ok(rows) => rows.into_iter().map(|r| r.param).collect(),
        Err(e) => {
            log::warn!("Failed to query table triggers: {}", e);
            return;
        }
    };

    if triggers.is_empty() {
        return;
    }

    let work_root = botbasic_core::utils::get_work_path();

    // Resolve bot name from bot_id to locate the correct work directory
    let bot_name: String = diesel::sql_query(
        "SELECT name FROM bots WHERE id = $1::uuid"
    )
    .bind::<diesel::sql_types::Text, _>(&user.bot_id.to_string())
    .get_result::<BotNameRow>(conn)
    .ok()
    .map(|r| r.name)
    .unwrap_or_else(|| user.bot_id.to_string());

    for script_name in triggers {
        // Build trigger context from record data
        let mut ctx = serde_json::Map::new();
        if let Some(ref rid) = record_id {
            ctx.insert("trigger_record_id".to_string(), serde_json::Value::String(rid.clone()));
        }
        if let Some(ref os) = old_status {
            ctx.insert("trigger_old_status".to_string(), serde_json::Value::String(os.clone()));
        }
        if let Some(ref ns) = new_status {
            ctx.insert("trigger_new_status".to_string(), serde_json::Value::String(ns.clone()));
        }

        let mut trigger_user = user.clone();
        if !ctx.is_empty() {
            trigger_user.context_data = serde_json::Value::Object(ctx);
        }

        // Try .ast first (pre-compiled Rhai), fall back to .bas (raw BASIC source)
        let ast_rel = format!("{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{script_name}.ast", org_id = botcore::shared::utils::current_org_id());
        let ast_path = std::path::Path::new(&work_root).join(&ast_rel);
        let bas_rel = format!("{org_id}.gborg/{bot_name}.gbai/{bot_name}.gbdialog/{script_name}.bas", org_id = botcore::shared::utils::current_org_id());
        let bas_path = std::path::Path::new(&work_root).join(&bas_rel);

        let content = std::fs::read_to_string(&ast_path)
            .or_else(|_| std::fs::read_to_string(&bas_path));

        match content {
            Ok(script_content) if !script_content.is_empty() => {
                if let Err(e) = state.execute_script(trigger_user, &script_content) {
                    log::warn!("Table trigger '{}' execution failed: {}", script_name, e);
                } else {
                    log::info!("Table trigger '{}' executed successfully", script_name);
                }
            }
            _ => log::warn!("Table trigger script not found: {:?} or {:?}", ast_path, bas_path),
        }
    }
}
