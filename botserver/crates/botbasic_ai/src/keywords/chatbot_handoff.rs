use std::sync::Arc;
use std::time::Duration;

use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use chrono::Utc;
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::{json, Value};
use uuid::Uuid;

use super::chatbot_handoff_storage::{
    append_event, ensure_schema, fetch_handoff, insert_queue_entry, queue_position,
    set_agent_presence, update_handoff_status, NewQueueEntry,
};

/// Chatbot handoff BASIC keywords for issue #621.
///
/// Provides: TRANSFER TO HUMAN, ESCALATE, REQUEST AGENT,
/// ASSIGN AGENT, COMPLETE HANDOFF.
///
/// Persistence model:
/// - handoff_queue: a single table holding all open handoff tickets with
///   status, priority, assigned agent, topic and full JSON metadata.
/// - handoff_events: append-only audit log for escalations, assignments,
///   completions and CSAT ratings.
/// - Redis is used for fast agent presence tracking
///   (presence:{agent_id} = "online" with TTL).
pub fn register_chatbot_handoff_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    if let Err(e) = ensure_default_schema(&state) {
        eprintln!("chatbot_handoff: bootstrap schema failed: {e}");
    }
    register_transfer_to_human(state.clone(), user.clone(), engine);
    register_escalate(state.clone(), user.clone(), engine);
    register_request_agent(state.clone(), user.clone(), engine);
    register_assign_agent(state.clone(), user.clone(), engine);
    register_complete_handoff(state, user, engine);
}

fn ensure_default_schema(state: &Arc<dyn BasicRuntime>) -> Result<(), String> {
    ensure_schema(state.db_pool())
}

fn runtime_error(message: impl Into<String>) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(message.into().into(), rhai::Position::NONE))
}

fn execute_with_timeout<F, T>(work: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("handoff-worker".into())
        .spawn(move || {
            let outcome = work();
            let _ = tx.send(outcome);
        })
        .map_err(|e| format!("Spawn: {e}"))?;
    rx.recv_timeout(Duration::from_secs(15))
        .map_err(|_| "Operation timed out after 15s".to_string())?
}

fn bot_id_from_session(user: &UserSession) -> Uuid {
    user.bot_id
}

fn user_id_from_session(user: &UserSession) -> Option<Uuid> {
    Some(user.user_id)
}

fn priority_int(priority: &str) -> i32 {
    match priority.to_ascii_lowercase().as_str() {
        "low" | "baixa" => 1,
        "normal" | "media" | "média" => 5,
        "high" | "alta" => 8,
        "urgent" | "critica" | "crítica" => 10,
        _ => 5,
    }
}

fn register_transfer_to_human(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["TRANSFER", "TO", "HUMAN", "REASON", "$expr$"],
            false,
            move |context, inputs| {
                let reason = context.eval_expression_tree(&inputs[0])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let reason_clone = reason.clone();
                let outcome = execute_with_timeout(move || -> Result<Value, String> {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let session_id = user_clone.id;
                    let topic = format!("User {user_id:?} request");
                    let metadata = json!({
                        "channel": "bot",
                        "session_id": session_id,
                    });
                    let handoff_id = insert_queue_entry(NewQueueEntry {
                        pool,
                        bot_id,
                        session_id: Some(session_id),
                        user_id,
                        topic: &topic,
                        reason: Some(&reason_clone),
                        priority: "normal",
                        metadata: &metadata,
                    })?;
                    append_event(
                        pool,
                        handoff_id,
                        "transfer",
                        &json!({ "reason": reason_clone.clone() }),
                        user_id,
                    )?;
                    let pos = queue_position(pool, bot_id, handoff_id)?;
                    Ok(json!({
                        "kind": "transfer_to_human",
                        "handoff_id": handoff_id.to_string(),
                        "reason": reason_clone,
                        "queue_position": pos,
                        "estimated_wait_seconds": pos * 60,
                        "queued_at": Utc::now().to_rfc3339(),
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("TRANSFER TO HUMAN: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("TRANSFER TO HUMAN registration: {e}")))
        .ok();
}

fn register_escalate(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["ESCALATE", "$expr$", "PRIORITY", "$expr$"],
            false,
            move |context, inputs| {
                let issue = context.eval_expression_tree(&inputs[0])?.to_string();
                let priority = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let issue_clone = issue.clone();
                let priority_clone = priority.clone();
                let outcome = execute_with_timeout(move || -> Result<Value, String> {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let session_id = user_clone.id;
                    let metadata = json!({
                        "channel": "bot",
                        "session_id": session_id,
                        "issue": issue_clone,
                    });
                    let handoff_id = insert_queue_entry(NewQueueEntry {
                        pool,
                        bot_id,
                        session_id: Some(session_id),
                        user_id,
                        topic: &issue_clone,
                        reason: Some(&issue_clone),
                        priority: &priority_clone,
                        metadata: &metadata,
                    })?;
                    append_event(
                        pool,
                        handoff_id,
                        "escalation",
                        &json!({
                            "priority": priority_clone,
                            "priority_rank": priority_int(&priority_clone),
                        }),
                        user_id,
                    )?;
                    Ok(json!({
                        "kind": "escalation",
                        "handoff_id": handoff_id.to_string(),
                        "issue": issue_clone,
                        "priority": priority_clone,
                        "priority_rank": priority_int(&priority_clone),
                        "escalated": true,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("ESCALATE: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("ESCALATE registration: {e}")))
        .ok();
}

fn register_request_agent(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["REQUEST", "AGENT", "TOPIC", "$expr$"],
            false,
            move |context, inputs| {
                let topic = context.eval_expression_tree(&inputs[0])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let topic_clone = topic.clone();
                let outcome = execute_with_timeout(move || -> Result<Value, String> {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let session_id = user_clone.id;
                    let metadata = json!({ "channel": "bot", "session_id": session_id });
                    let handoff_id = insert_queue_entry(NewQueueEntry {
                        pool,
                        bot_id,
                        session_id: Some(session_id),
                        user_id,
                        topic: &topic_clone,
                        reason: Some(&topic_clone),
                        priority: "normal",
                        metadata: &metadata,
                    })?;
                    append_event(
                        pool,
                        handoff_id,
                        "request",
                        &json!({ "topic": topic_clone }),
                        user_id,
                    )?;
                    let pos = queue_position(pool, bot_id, handoff_id)?;
                    Ok(json!({
                        "kind": "request_agent",
                        "handoff_id": handoff_id.to_string(),
                        "topic": topic_clone,
                        "agent_id": null,
                        "queue_position": pos,
                        "waiting": true,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("REQUEST AGENT: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("REQUEST AGENT registration: {e}")))
        .ok();
}

fn register_assign_agent(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["ASSIGN", "AGENT", "$expr$", "TO", "$expr$"],
            false,
            move |context, inputs| {
                let agent = context.eval_expression_tree(&inputs[0])?.to_string();
                let session_str = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let agent_clone = agent.clone();
                let outcome = execute_with_timeout(move || -> Result<Value, String> {
                    let agent_id = Uuid::parse_str(&agent_clone)
                        .map_err(|e| format!("agent must be UUID: {e}"))?;
                    let handoff_id = Uuid::parse_str(&session_str)
                        .map_err(|e| format!("session must be UUID: {e}"))?;
                    let pool = state.db_pool();
                    let user_id = user_id_from_session(&user_clone);
                    update_handoff_status(pool, handoff_id, "assigned", Some(agent_id))?;
                    append_event(
                        pool,
                        handoff_id,
                        "assigned",
                        &json!({ "agent_id": agent_id }),
                        user_id,
                    )?;
                    set_agent_presence(&state, agent_id, true);
                    Ok(json!({
                        "kind": "assign_agent",
                        "agent": agent_id.to_string(),
                        "session": handoff_id.to_string(),
                        "assigned": true,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("ASSIGN AGENT: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("ASSIGN AGENT registration: {e}")))
        .ok();
}

fn register_complete_handoff(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["COMPLETE", "HANDOFF", "$expr$", "RATING", "$expr$"],
            false,
            move |context, inputs| {
                let session_str = context.eval_expression_tree(&inputs[0])?.to_string();
                let rating_str = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let outcome = execute_with_timeout(move || -> Result<Value, String> {
                    let handoff_id = Uuid::parse_str(&session_str)
                        .map_err(|e| format!("session must be UUID: {e}"))?;
                    let rating: f64 = rating_str
                        .parse()
                        .map_err(|e| format!("rating must be numeric: {e}"))?;
                    let pool = state.db_pool();
                    let user_id = user_id_from_session(&user_clone);
                    let prior_agent = fetch_handoff(pool, handoff_id)?;
                    update_handoff_status(pool, handoff_id, "completed", prior_agent)?;
                    append_event(
                        pool,
                        handoff_id,
                        "completed",
                        &json!({ "csat": rating }),
                        user_id,
                    )?;
                    if let Some(agent_id) = prior_agent {
                        set_agent_presence(&state, agent_id, false);
                    }
                    Ok(json!({
                        "kind": "complete_handoff",
                        "session": handoff_id.to_string(),
                        "rating": rating,
                        "agent": prior_agent.map(|a| a.to_string()),
                        "completed": true,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("COMPLETE HANDOFF: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("COMPLETE HANDOFF registration: {e}")))
        .ok();
}
