use log::info;
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};
use diesel::prelude::*;

use crate::shared::models::TriggerKind;
use crate::shared::state::AppState;
use crate::shared::models::UserSession;

pub fn set_schedule_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["SET_SCHEDULE", "$string$"], true, {
            move |context, inputs| {
                let cron = context.eval_expression_tree(&inputs[0])?.to_string();
                let script_name = format!("cron_{}.rhai", cron.replace(' ', "_"));

                let conn = state_clone.conn.lock().unwrap();
                let result = execute_set_schedule(&*conn, &cron, &script_name)
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

pub fn execute_set_schedule(
    conn: &diesel::PgConnection,
    cron: &str,
    script_name: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    info!(
        "Starting execute_set_schedule with cron: {}, script_name: {}",
        cron, script_name
    );

    use crate::shared::models::system_automations;

    let new_automation = (
        system_automations::kind.eq(TriggerKind::Scheduled as i32),
        system_automations::schedule.eq(cron),
        system_automations::script_name.eq(script_name),
    );

    let result = diesel::insert_into(system_automations::table)
        .values(&new_automation)
        .execute(conn)?;

    Ok(json!({
        "command": "set_schedule",
        "schedule": cron,
        "script_name": script_name,
        "rows_affected": result
    }))
}
