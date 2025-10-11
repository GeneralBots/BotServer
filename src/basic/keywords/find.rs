use diesel::deserialize::QueryableByName;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::Text;
use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};

use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use crate::shared::utils;
use crate::shared::utils::to_array;

pub fn find_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let connection = state.custom_conn.clone();

    engine
        .register_custom_syntax(&["FIND", "$expr$", ",", "$expr$"], false, {
            move |context, inputs| {
                let table_name = context.eval_expression_tree(&inputs[0])?;
                let filter = context.eval_expression_tree(&inputs[1])?;
                let mut binding = connection.lock().unwrap();

                // Use the current async context instead of creating a new runtime
                let binding2 = table_name.to_string();
                let binding3 = filter.to_string();

                // Since execute_find is async but we're in a sync context, we need to block on it
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { execute_find(&mut binding, &binding2, &binding3).await })
                })
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

pub async fn execute_find(
    conn: &mut PgConnection,
    table_str: &str,
    filter_str: &str,
) -> Result<Value, String> {
    // Changed to String error like your Actix code
    info!(
        "Starting execute_find with table: {}, filter: {}",
        table_str, filter_str
    );

    let (where_clause, params) = utils::parse_filter(filter_str).map_err(|e| e.to_string())?;

    let query = format!(
        "SELECT * FROM {} WHERE {} LIMIT 10",
        table_str, where_clause
    );
    info!("Executing query: {}", query);

    // Define a struct that can deserialize from named rows
    #[derive(QueryableByName)]
    struct DynamicRow {
        #[diesel(sql_type = Text)]
        _placeholder: String,
    }

    // Execute raw SQL and get raw results
    let raw_result = diesel::sql_query(&query)
        .bind::<diesel::sql_types::Text, _>(&params[0])
        .execute(conn)
        .map_err(|e| {
            error!("SQL execution error: {}", e);
            e.to_string()
        })?;

    info!("Query executed successfully, affected {} rows", raw_result);

    // For now, create placeholder results since we can't easily deserialize dynamic rows
    let mut results = Vec::new();

    // This is a simplified approach - in a real implementation you'd need to:
    // 1. Query the table schema to know column types
    // 2. Build a proper struct or use a more flexible approach
    // 3. Or use a different database library that supports dynamic queries better

    // Placeholder result for demonstration
    let json_row = serde_json::json!({
        "note": "Dynamic row deserialization not implemented - need table schema"
    });
    results.push(json_row);

    Ok(json!({
        "command": "find",
        "table": table_str,
        "filter": filter_str,
        "results": results
    }))
}
