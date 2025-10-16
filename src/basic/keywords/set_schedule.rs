use diesel::prelude::*;
use log::info;
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};

use crate::shared::models::TriggerKind;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;

pub fn set_schedule_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["SET_SCHEDULE", "$string$"], true, {
            move |context, inputs| {
                let cron = context.eval_expression_tree(&inputs[0])?.to_string();
                let param = format!("cron_{}.rhai", cron.replace(' ', "_"));

                let mut conn = state_clone.conn.lock().unwrap();
                let result = execute_set_schedule(&mut *conn, &cron, &param)
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
    conn: &mut diesel::PgConnection,
    cron: &str,
    param: &str,
) -> Result<Value, Box<dyn std::error::Error>> {
    info!(
        "Starting execute_set_schedule with cron: {}, param: {}",
        cron, param
    );

    use crate::shared::models::system_automations;

    let new_automation = (
        system_automations::kind.eq(TriggerKind::Scheduled as i32),
        system_automations::schedule.eq(cron),
        system_automations::param.eq(param),
    );

    let result = diesel::insert_into(system_automations::table)
        .values(&new_automation)
        .execute(&mut *conn)?;

    Ok(json!({
        "command": "set_schedule",
        "schedule": cron,
        "param": param,
        "rows_affected": result
    }))
}
