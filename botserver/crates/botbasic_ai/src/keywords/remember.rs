use diesel::prelude::*;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;
use botbasic_types::UserSession;
use botbasic_types::BasicRuntime;
use chrono::{Duration, Utc};
use log::trace;
use rhai::{Dynamic, Engine};

pub fn remember_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user.clone();

    if let Err(e) = engine
        .register_custom_syntax(
            ["REMEMBER", "$expr$", ",", "$expr$", ",", "$expr$"],
            false,
            move |context, inputs| {
                let key = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?;
                let duration_str = context.eval_expression_tree(&inputs[2])?.to_string();

                trace!(
                    "REMEMBER: key={}, duration={} for user={}",
                    key,
                    duration_str,
                    user_clone.user_id
                );

                let expiry = parse_duration(&duration_str)?;

                let json_value = if value.is_string() {
                    json!(value.to_string())
                } else if value.is_int() {
                    json!(value.as_int().unwrap_or(0))
                } else if value.is_float() {
                    json!(value.as_float().unwrap_or(0.0))
                } else if value.is_bool() {
                    json!(value.as_bool().unwrap_or(false))
                } else if value.is_array() {
                    let arr = value.cast::<rhai::Array>();
                    let json_arr: Vec<serde_json::Value> =
                        arr.iter().map(|v| json!(v.to_string())).collect();
                    json!(json_arr)
                } else {
                    json!(value.to_string())
                };

                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let key_for_task = key.clone();

                let (tx, rx) = std::sync::mpsc::channel();

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Builder::new_multi_thread()
                        .worker_threads(2)
                        .enable_all()
                        .build();

                    let send_err = if let Ok(rt) = rt {
                        let result = rt.block_on(async move {
                            store_memory(
                                &state_for_task,
                                &user_for_task,
                                &key_for_task,
                                json_value,
                                expiry,
                            )
                        });
                        tx.send(result).err()
                    } else {
                        tx.send(Err("Failed to build tokio runtime".to_string()))
                            .err()
                    };

                    if send_err.is_some() {
                        log::error!("Failed to send REMEMBER result from thread");
                    }
                });

                match rx.recv_timeout(std::time::Duration::from_secs(5)) {
                    Ok(Ok(_)) => Ok(Dynamic::from(format!(
                        "Remembered '{}' for {}",
                        key, duration_str
                    ))),
                    Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("REMEMBER failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                            "REMEMBER timed out".into(),
                            rhai::Position::NONE,
                        )))
                    }
                    Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        format!("REMEMBER thread failed: {}", e).into(),
                        rhai::Position::NONE,
                    ))),
                }
            },
        )
    {
        log::error!("Failed to register the custom syntax: {e}");
    }

    let state_clone2 = Arc::clone(&state);
    let user_clone2 = user;

    if let Err(e) = engine
        .register_custom_syntax(["RECALL", "$expr$"], false, move |context, inputs| {
            let key = context.eval_expression_tree(&inputs[0])?.to_string();

            trace!("RECALL: key={} for user={}", key, user_clone2.user_id);

            let state_for_task = Arc::clone(&state_clone2);
            let user_for_task = user_clone2.clone();

            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        retrieve_memory(&state_for_task, &user_for_task, &key)
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("Failed to build tokio runtime".to_string()))
                        .err()
                };

                if send_err.is_some() {
                    log::error!("Failed to send RECALL result from thread");
                }
            });

            match rx.recv_timeout(std::time::Duration::from_secs(5)) {
                Ok(Ok(value)) => {
                    if value.is_string() {
                        Ok(Dynamic::from(value.as_str().unwrap_or("").to_string()))
                    } else if value.is_number() {
                        if let Some(i) = value.as_i64() {
                            Ok(Dynamic::from(i))
                        } else if let Some(f) = value.as_f64() {
                            Ok(Dynamic::from(f))
                        } else {
                            Ok(Dynamic::from(value.to_string()))
                        }
                    } else if value.is_boolean() {
                        Ok(Dynamic::from(value.as_bool().unwrap_or(false)))
                    } else if value.is_array() {
                        let arr_str = value.to_string();
                        Ok(Dynamic::from(arr_str))
                    } else {
                        Ok(Dynamic::from(value.to_string()))
                    }
                }
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("RECALL failed: {}", e).into(),
                    rhai::Position::NONE,
                ))),
                Err(_) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    "RECALL timed out".into(),
                    rhai::Position::NONE,
                ))),
            }
        })
    {
        log::error!("Failed to register the custom syntax: {e}");
    }
}

fn parse_duration(
    duration_str: &str,
) -> Result<Option<chrono::DateTime<Utc>>, Box<rhai::EvalAltResult>> {
    let duration_lower = duration_str.to_lowercase();

    if duration_lower == "forever" || duration_lower == "permanent" {
        return Ok(None);
    }

    let parts: Vec<&str> = duration_lower.split_whitespace().collect();
    if parts.len() != 2 {
        if let Ok(days) = duration_str.parse::<i64>() {
            return Ok(Some(Utc::now() + Duration::days(days)));
        }
        return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
            format!("Invalid duration format: {}", duration_str).into(),
            rhai::Position::NONE,
        )));
    }

    let amount = parts[0].parse::<i64>().map_err(|_| {
        Box::new(rhai::EvalAltResult::ErrorRuntime(
            format!("Invalid duration amount: {}", parts[0]).into(),
            rhai::Position::NONE,
        ))
    })?;

    let unit = parts[1].trim_end_matches('s');

    let duration = match unit {
        "second" => Duration::seconds(amount),
        "minute" => Duration::minutes(amount),
        "hour" => Duration::hours(amount),
        "day" => Duration::days(amount),
        "week" => Duration::weeks(amount),
        "month" => Duration::days(amount * 30),
        "year" => Duration::days(amount * 365),
        _ => {
            return Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                format!("Invalid duration unit: {}", unit).into(),
                rhai::Position::NONE,
            )))
        }
    };

    Ok(Some(Utc::now() + duration))
}

fn store_memory(
    state: &Arc<dyn BasicRuntime>,
    user: &UserSession,
    key: &str,
    value: serde_json::Value,
    expiry: Option<chrono::DateTime<Utc>>,
) -> Result<(), String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let memory_id = Uuid::new_v4().to_string();
    let user_id = user.user_id.to_string();
    let bot_id = user.bot_id.to_string();
    let session_id = user.id.to_string();
    let created_at = Utc::now().to_rfc3339();
    let expires_at = expiry.map(|e| e.to_rfc3339());

    let query = diesel::sql_query(
        "INSERT INTO bot_memories (id, user_id, bot_id, session_id, key, value, created_at, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (user_id, bot_id, key)
         DO UPDATE SET value = $6, created_at = $7, expires_at = $8, session_id = $4"
    )
    .bind::<diesel::sql_types::Text, _>(&memory_id)
    .bind::<diesel::sql_types::Text, _>(&user_id)
    .bind::<diesel::sql_types::Text, _>(&bot_id)
    .bind::<diesel::sql_types::Text, _>(&session_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::Jsonb, _>(&value)
    .bind::<diesel::sql_types::Text, _>(&created_at)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&expires_at);

    query.execute(&mut *conn).map_err(|e| {
        log::error!("Failed to store memory: {}", e);
        format!("Failed to store memory: {}", e)
    })?;

    trace!("Stored memory key='{}' for user={}", key, user_id);
    Ok(())
}

fn retrieve_memory(
    state: &Arc<dyn BasicRuntime>,
    user: &UserSession,
    key: &str,
) -> Result<serde_json::Value, String> {
    let mut conn = state.db_pool().get().map_err(|e| format!("DB error: {}", e))?;

    let user_id = user.user_id.to_string();
    let bot_id = user.bot_id.to_string();
    let now = Utc::now().to_rfc3339();

    let query = diesel::sql_query(
        "SELECT value FROM bot_memories
         WHERE user_id = $1 AND bot_id = $2 AND key = $3
         AND (expires_at IS NULL OR expires_at > $4)
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(&user_id)
    .bind::<diesel::sql_types::Text, _>(&bot_id)
    .bind::<diesel::sql_types::Text, _>(key)
    .bind::<diesel::sql_types::Text, _>(&now);

    let result: Result<Vec<MemoryRecord>, _> = query.load(&mut *conn);

    match result {
        Ok(records) if !(records.is_empty()) => {
            trace!("Retrieved memory key='{}' for user={}", key, user_id);
            Ok(records[0].value.clone())
        }
        Ok(_) => {
            trace!("No memory found for key='{}' user={}", key, user_id);
            Ok(json!(null))
        }
        Err(e) => {
            log::error!("Failed to retrieve memory: {}", e);
            Err(format!("Failed to retrieve memory: {}", e))
        }
    }
}

#[derive(QueryableByName, Debug)]
struct MemoryRecord {
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    value: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert!(parse_duration("30 days").is_ok());
        assert!(parse_duration("1 hour").is_ok());
        assert!(parse_duration("forever").is_ok());
        assert!(parse_duration("5 minutes").is_ok());
        assert!(parse_duration("invalid").is_err());
    }
}
