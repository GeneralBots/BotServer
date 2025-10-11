use diesel::prelude::*;
use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};
use std::error::Error;

use crate::shared::models::UserSession;
use crate::shared::state::AppState;

pub fn set_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["SET", "$expr$", ",", "$expr$", ",", "$expr$"], false, {
            move |context, inputs| {
                let table_name = context.eval_expression_tree(&inputs[0])?;
                let filter = context.eval_expression_tree(&inputs[1])?;
                let updates = context.eval_expression_tree(&inputs[2])?;

                let table_str = table_name.to_string();
                let filter_str = filter.to_string();
                let updates_str = updates.to_string();

                let mut conn = state_clone.conn.lock().unwrap();
                let result = execute_set(&mut *conn, &table_str, &filter_str, &updates_str)
                    .map_err(|e| format!("DB error: {}", e))?;

                if let Some(rows_affected) = result.get("rows_affected") {
                    Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                } else {
                    Err("No rows affected".into())
                }
            }
        })
        .unwrap();
}

pub fn execute_set(
    conn: &mut diesel::PgConnection,
    table_str: &str,
    filter_str: &str,
    updates_str: &str,
) -> Result<Value, String> {
    info!(
        "Starting execute_set with table: {}, filter: {}, updates: {}",
        table_str, filter_str, updates_str
    );

    let (set_clause, _update_values) = parse_updates(updates_str).map_err(|e| e.to_string())?;

    let where_clause = parse_filter_for_diesel(filter_str).map_err(|e| e.to_string())?;

    let query = format!(
        "UPDATE {} SET {} WHERE {}",
        table_str, set_clause, where_clause
    );
    info!("Executing query: {}", query);

    let result = diesel::sql_query(&query).execute(conn).map_err(|e| {
        error!("SQL execution error: {}", e);
        e.to_string()
    })?;

    Ok(json!({
        "command": "set",
        "table": table_str,
        "filter": filter_str,
        "updates": updates_str,
        "rows_affected": result
    }))
}

fn parse_updates(updates_str: &str) -> Result<(String, Vec<String>), Box<dyn Error>> {
    let mut set_clauses = Vec::new();
    let mut params = Vec::new();

    for (i, update) in updates_str.split(',').enumerate() {
        let parts: Vec<&str> = update.split('=').collect();
        if parts.len() != 2 {
            return Err("Invalid update format".into());
        }

        let column = parts[0].trim();
        let value = parts[1].trim();

        if !column
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err("Invalid column name".into());
        }

        set_clauses.push(format!("{} = ${}", column, i + 1));
        params.push(value.to_string());
    }

    Ok((set_clauses.join(", "), params))
}

fn parse_filter_for_diesel(filter_str: &str) -> Result<String, Box<dyn Error>> {
    let parts: Vec<&str> = filter_str.split('=').collect();
    if parts.len() != 2 {
        return Err("Invalid filter format. Expected 'KEY=VALUE'".into());
    }

    let column = parts[0].trim();
    let value = parts[1].trim();

    if !column
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Invalid column name in filter".into());
    }

    Ok(format!("{} = '{}'", column, value))
}
