use log::info;
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::shared::models::TriggerKind;
use crate::shared::state::AppState;

pub fn set_schedule_keyword(state: &AppState, engine: &mut Engine) {
    let db = state.db_custom.clone();

    engine
        .register_custom_syntax(["SET_SCHEDULE", "$string$"], true, {
            let db = db.clone();

            move |context, inputs| {
                let cron = context.eval_expression_tree(&inputs[0])?.to_string();
                let script_name = format!("cron_{}.rhai", cron.replace(' ', "_"));

                let binding = db.as_ref().unwrap();
                let fut = execute_set_schedule(binding, &cron, &script_name);

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

pub async fn execute_set_schedule(
    pool: &PgPool,
    cron: &str,
    script_name: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    info!(
        "Starting execute_set_schedule with cron: {}, script_name: {}",
        cron, script_name
    );

    let result = sqlx::query(
        r#"
        INSERT INTO system_automations
        (kind, schedule, script_name)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(TriggerKind::Scheduled as i32) // Cast to i32
    .bind(cron)
    .bind(script_name)
    .execute(pool)
    .await?;

    Ok(json!({
        "command": "set_schedule",
        "schedule": cron,
        "script_name": script_name,
        "rows_affected": result.rows_affected()
    }))
}
