use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::shared::models::automation_model::TriggerKind;
use crate::shared::state::AppState;

pub fn on_keyword(state: &AppState, engine: &mut Engine) {
    let db = state.db_custom.clone();

    engine
        .register_custom_syntax(
            ["ON", "$ident$", "OF", "$string$"], // Changed $string$ to $ident$ for operation
            true,
            {
                let db = db.clone();

                move |context, inputs| {
                    let trigger_type = context.eval_expression_tree(&inputs[0])?.to_string();
                    let table = context.eval_expression_tree(&inputs[1])?.to_string();
                    let script_name = format!("{}_{}.rhai", table, trigger_type.to_lowercase());

                    // Determine the trigger kind based on the trigger type
                    let kind = match trigger_type.to_uppercase().as_str() {
                        "UPDATE" => TriggerKind::TableUpdate,
                        "INSERT" => TriggerKind::TableInsert,
                        "DELETE" => TriggerKind::TableDelete,
                        _ => return Err(format!("Invalid trigger type: {}", trigger_type).into()),
                    };

                    let binding = db.as_ref().unwrap();
                    let fut = execute_on_trigger(binding, kind, &table, &script_name);

                    let result = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(fut)
                    })
                    .map_err(|e| format!("DB error: {}", e))?;

                    if let Some(rows_affected) = result.get("rows_affected") {
                        Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                    } else {
                        Err("No rows affected".into())
                    }
                }
            },
        )
        .unwrap();
}

pub async fn execute_on_trigger(
    pool: &PgPool,
    kind: TriggerKind,
    table: &str,
    script_name: &str,
) -> Result<Value, String> {
    info!(
        "Starting execute_on_trigger with kind: {:?}, table: {}, script_name: {}",
        kind, table, script_name
    );

    // Option 1: Use query_with macro if you need to pass enum values
    let result = sqlx::query(
        "INSERT INTO system_automations
        (kind, target, script_name)
        VALUES ($1, $2, $3)",
    )
    .bind(kind.clone() as i32) // Assuming TriggerKind is #[repr(i32)]
    .bind(table)
    .bind(script_name)
    .execute(pool)
    .await
    .map_err(|e| {
        error!("SQL execution error: {}", e);
        e.to_string()
    })?;

    Ok(json!({
        "command": "on_trigger",
        "trigger_type": format!("{:?}", kind),
        "table": table,
        "script_name": script_name,
        "rows_affected": result.rows_affected()
    }))
}
