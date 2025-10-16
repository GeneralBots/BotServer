use diesel::prelude::*;
use log::{error, info};
use rhai::Dynamic;
use rhai::Engine;
use serde_json::{json, Value};

use crate::shared::models::TriggerKind;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;

pub fn on_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(
            &["ON", "$ident$", "OF", "$string$"],
            true,
            move |context, inputs| {
                let trigger_type = context.eval_expression_tree(&inputs[0])?.to_string();
                let table = context.eval_expression_tree(&inputs[1])?.to_string();
                let name = format!("{}_{}.rhai", table, trigger_type.to_lowercase());

                let kind = match trigger_type.to_uppercase().as_str() {
                    "UPDATE" => TriggerKind::TableUpdate,
                    "INSERT" => TriggerKind::TableInsert,
                    "DELETE" => TriggerKind::TableDelete,
                    _ => return Err(format!("Invalid trigger type: {}", trigger_type).into()),
                };

                let mut conn = state_clone.conn.lock().unwrap();
                let result = execute_on_trigger(&mut *conn, kind, &table, &name)
                    .map_err(|e| format!("DB error: {}", e))?;

                if let Some(rows_affected) = result.get("rows_affected") {
                    Ok(Dynamic::from(rows_affected.as_i64().unwrap_or(0)))
                } else {
                    Err("No rows affected".into())
                }
            },
        )
        .unwrap();
}

pub fn execute_on_trigger(
    conn: &mut diesel::PgConnection,
    kind: TriggerKind,
    table: &str,
    param: &str,
) -> Result<Value, String> {
    info!(
        "Starting execute_on_trigger with kind: {:?}, table: {}, param: {}",
        kind, table, param
    );

    use crate::shared::models::system_automations;

    let new_automation = (
        system_automations::kind.eq(kind as i32),
        system_automations::target.eq(table),
        system_automations::param.eq(param),
    );

    let result = diesel::insert_into(system_automations::table)
        .values(&new_automation)
        .execute(conn)
        .map_err(|e| {
            error!("SQL execution error: {}", e);
            e.to_string()
        })?;

    Ok(json!({
        "command": "on_trigger",
        "trigger_type": format!("{:?}", kind),
        "table": table,
        "param": param,
        "rows_affected": result
    }))
}
