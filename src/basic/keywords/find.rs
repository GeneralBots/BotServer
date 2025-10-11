use diesel::prelude::*;
use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};

use crate::shared::state::AppState;
use crate::shared::models::UserSession;
use crate::shared::utils;
use crate::shared::utils::row_to_json;
use crate::shared::utils::to_array;

pub fn find_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["FIND", "$expr$", ",", "$expr$"], false, {
            move |context, inputs| {
                let table_name = context.eval_expression_tree(&inputs[0])?;
                let filter = context.eval_expression_tree(&inputs[1])?;

                let table_str = table_name.to_string();
                let filter_str = filter.to_string();

                let conn = state_clone.conn.lock().unwrap().clone();
                let result = execute_find(&conn, &table_str, &filter_str)
                    .map_err(|e| format!("DB error: {}", e))?;

                if let Some(results) = result.get("results") {
                    let array = to_array(utils::json_value_to_dynamic(results));
                    Ok(Dynamic::from(array))
                } else {
                    Err("No results".into())
                }
            }
        })
        .unwrap();
}

pub fn execute_find(
    conn: &PgConnection,
    table_str: &str,
    filter_str: &str,
) -> Result<Value, String> {
    info!(
        "Starting execute_find with table: {}, filter: {}",
        table_str, filter_str
    );

    let where_clause = parse_filter_for_diesel(filter_str).map_err(|e| e.to_string())?;

    let query = format!(
        "SELECT * FROM {} WHERE {} LIMIT 10",
        table_str, where_clause
    );
    info!("Executing query: {}", query);

    let mut conn_mut = conn.clone();

    #[derive(diesel::QueryableByName, Debug)]
    struct JsonRow {
        #[diesel(sql_type = diesel::sql_types::Jsonb)]
        json: serde_json::Value,
    }

    let json_query = format!(
        "SELECT row_to_json(t) AS json FROM {} t WHERE {} LIMIT 10",
        table_str, where_clause
    );

    let rows: Vec<JsonRow> = diesel::sql_query(&json_query)
        .load::<JsonRow>(&mut conn_mut)
        .map_err(|e| {
            error!("SQL execution error: {}", e);
            e.to_string()
        })?;

    info!("Query successful, got {} rows", rows.len());

    let mut results = Vec::new();
    for row in rows {
        results.push(row.json);
    }

    Ok(json!({
        "command": "find",
        "table": table_str,
        "filter": filter_str,
        "results": results
    }))
}

fn parse_filter_for_diesel(filter_str: &str) -> Result<String, Box<dyn std::error::Error>> {
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
