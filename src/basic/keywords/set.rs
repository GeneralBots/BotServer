use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::error::Error;

use crate::shared::state::AppState;
use crate::shared::utils;

pub fn set_keyword(state: &AppState, engine: &mut Engine) {
    let db = state.db_custom.clone();

    engine
        .register_custom_syntax(&["SET", "$expr$", ",", "$expr$", ",", "$expr$"], false, {
            let db = db.clone();

            move |context, inputs| {
                let table_name = context.eval_expression_tree(&inputs[0])?;
                let filter = context.eval_expression_tree(&inputs[1])?;
                let updates = context.eval_expression_tree(&inputs[2])?;
                let binding = db.as_ref().unwrap();

                // Use the current async context instead of creating a new runtime
                let binding2 = table_name.to_string();
                let binding3 = filter.to_string();
                let binding4 = updates.to_string();
                let fut = execute_set(binding, &binding2, &binding3, &binding4);

                // Use tokio::task::block_in_place + tokio::runtime::Handle::current().block_on
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
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

pub async fn execute_set(
    pool: &PgPool,
    table_str: &str,
    filter_str: &str,
    updates_str: &str,
) -> Result<Value, String> {
    info!(
        "Starting execute_set with table: {}, filter: {}, updates: {}",
        table_str, filter_str, updates_str
    );

    // Parse updates with proper type handling
    let (set_clause, update_values) = parse_updates(updates_str).map_err(|e| e.to_string())?;
    let update_params_count = update_values.len();

    // Parse filter with proper type handling
    let (where_clause, filter_values) =
        utils::parse_filter_with_offset(filter_str, update_params_count)
            .map_err(|e| e.to_string())?;

    let query = format!(
        "UPDATE {} SET {} WHERE {}",
        table_str, set_clause, where_clause
    );
    info!("Executing query: {}", query);

    // Build query with proper parameter binding
    let mut query = sqlx::query(&query);

    // Bind update values
    for value in update_values {
        query = bind_value(query, value);
    }

    // Bind filter values
    for value in filter_values {
        query = bind_value(query, value);
    }

    let result = query.execute(pool).await.map_err(|e| {
        error!("SQL execution error: {}", e);
        e.to_string()
    })?;

    Ok(json!({
        "command": "set",
        "table": table_str,
        "filter": filter_str,
        "updates": updates_str,
        "rows_affected": result.rows_affected()
    }))
}

fn bind_value<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: String,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    if let Ok(int_val) = value.parse::<i64>() {
        query.bind(int_val)
    } else if let Ok(float_val) = value.parse::<f64>() {
        query.bind(float_val)
    } else if value.eq_ignore_ascii_case("true") {
        query.bind(true)
    } else if value.eq_ignore_ascii_case("false") {
        query.bind(false)
    } else {
        query.bind(value)
    }
}

// Parse updates without adding quotes
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
        params.push(value.to_string()); // Store raw value without quotes
    }

    Ok((set_clauses.join(", "), params))
}
