use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use chrono::Utc;
use rhai::{Dynamic, Engine, EvalAltResult};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use botlearn::basic_storage::{
    ensure_schema as ensure_learn_schema, persist_lesson, progress_for, upsert_fact,
};

/// Learning and education BASIC keywords for issue #625.
///
/// Provides: TEACH, STUDY, QUIZ, FLASHCARD, CURRICULUM,
/// LESSON, PROGRESS, REMEMBER FACT.
///
/// Persistence is delegated to `botlearn::basic_storage`
/// (learn_facts/learn_basic_lessons) — #1506.
/// LLM-backed keywords (TEACH, STUDY, QUIZ, CURRICULUM) build structured
/// JSON content via `state.llm_generate` and fall back to local scaffolding
/// when the LLM is not configured.
pub fn register_learn_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    if let Err(e) = ensure_learn_schema(state.db_pool()) {
        log::error!("learn: bootstrap schema failed: {e}");
    }
    register_teach(state.clone(), user.clone(), engine);
    register_study(state.clone(), user.clone(), engine);
    register_quiz(state.clone(), user.clone(), engine);
    register_flashcard(state.clone(), user.clone(), engine);
    register_curriculum(state.clone(), user.clone(), engine);
    register_lesson(state.clone(), user.clone(), engine);
    register_progress(state.clone(), user.clone(), engine);
    register_remember_fact(state, user, engine);
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
        .name("learn-worker".into())
        .spawn(move || {
            let outcome = work();
            let _ = tx.send(outcome);
        })
        .map_err(|e| format!("Spawn: {e}"))?;
    rx.recv_timeout(Duration::from_secs(45))
        .map_err(|_| "Operation timed out after 45s".to_string())?
}

fn bot_id_from_session(user: &UserSession) -> Uuid {
    user.bot_id
}

fn user_id_from_session(user: &UserSession) -> Option<Uuid> {
    Some(user.user_id)
}

fn llm_content(state: &Arc<dyn BasicRuntime>, prompt: &str, fallback: &str) -> String {
    let model = state.config_value("llm-model").unwrap_or_default();
    let key = state.config_value("llm-key").unwrap_or_default();
    state
        .llm_generate(prompt, &model, &key)
        .unwrap_or_else(|_| fallback.to_string())
}

fn register_teach(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["TEACH", "$expr$", "ABOUT", "$expr$"],
            false,
            move |context, inputs| {
                let topic = context.eval_expression_tree(&inputs[0])?.to_string();
                let content = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let topic_clone = topic.clone();
                let content_clone = content.clone();
                let outcome = execute_with_timeout(move || {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let prompt = format!(
                        "Crie um plano de aula em português brasileiro sobre o tema '{topic_clone}', \
                         com base no conteúdo: {content_clone}. Estruture em 4 módulos \
                         (Fundamentos, Intermediário, Avançado, Maestria) com exemplos práticos."
                    );
                    let plan = llm_content(
                        &state,
                        &prompt,
                        &format!("Plano autoguiado sobre {topic_clone} baseado em: {content_clone}"),
                    );
                    let body = json!({ "summary": content_clone, "plan": plan });
                    let id = persist_lesson(pool, bot_id, user_id, &topic_clone, "teach", &body)?;
                    Ok(json!({
                        "kind": "lesson_plan",
                        "id": id.to_string(),
                        "topic": topic_clone,
                        "content": body,
                        "stage": "introduced",
                        "timestamp": Utc::now().to_rfc3339(),
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("TEACH: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("TEACH registration: {e}")))
        .ok();
}

fn register_study(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(["STUDY", "$expr$"], false, move |context, inputs| {
            let topic = context.eval_expression_tree(&inputs[0])?.to_string();
            let state = Arc::clone(&state_clone);
            let user_clone = user.clone();
            let topic_clone = topic.clone();
            let outcome = execute_with_timeout(move || {
                let pool = state.db_pool();
                let bot_id = bot_id_from_session(&user_clone);
                let user_id = user_id_from_session(&user_clone);
                let prompt = format!(
                    "Crie uma sessão de estudo em português brasileiro sobre '{topic_clone}'. \
                     Forneça: definição, exemplo prático, caso de uso e armadilhas comuns."
                );
                let items = llm_content(
                    &state,
                    &prompt,
                    &format!(
                        "Estudo autoguiado sobre {topic_clone}: pesquise definição, \
                         2 exemplos, 1 caso real e 3 erros comuns."
                    ),
                );
                let body = json!({ "items": items });
                let id = persist_lesson(pool, bot_id, user_id, &topic_clone, "study", &body)?;
                Ok(json!({
                    "kind": "study_session",
                    "id": id.to_string(),
                    "topic": topic_clone,
                    "items": body,
                }))
            });
            match outcome {
                Ok(v) => Ok(Dynamic::from(v.to_string())),
                Err(e) => Err(runtime_error(format!("STUDY: {e}"))),
            }
        })
        .map_err(|e| runtime_error(format!("STUDY registration: {e}")))
        .ok();
}

fn register_quiz(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["QUIZ", "$expr$", "WITH", "$expr$", "QUESTIONS"],
            false,
            move |context, inputs| {
                let topic = context.eval_expression_tree(&inputs[0])?.to_string();
                let count_str = context.eval_expression_tree(&inputs[1])?.to_string();
                let count: usize = count_str.parse().unwrap_or(5);
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let topic_clone = topic.clone();
                let outcome = execute_with_timeout(move || {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let prompt = format!(
                        "Gere {count} perguntas de múltipla escolha em português brasileiro \
                         sobre o tema '{topic_clone}'. Responda APENAS com JSON no formato \
                         {{\"questions\":[{{\"q\":\"...\",\"a\":\"...\",\"options\":[a,b,c,d]}}]}}."
                    );
                    let fallback = json!({ "questions": [] });
                    let body = match llm_content(&state, &prompt, "").as_str() {
                        text if !text.is_empty() => serde_json::from_str::<Value>(text)
                            .unwrap_or(fallback.clone()),
                        _ => fallback,
                    };
                    let id = persist_lesson(pool, bot_id, user_id, &topic_clone, "quiz", &body)?;
                    Ok(json!({
                        "kind": "quiz",
                        "id": id.to_string(),
                        "topic": topic_clone,
                        "count": count,
                        "questions": body.get("questions").cloned().unwrap_or(json!([])),
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("QUIZ: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("QUIZ registration: {e}")))
        .ok();
}

fn register_flashcard(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["FLASHCARD", "$expr$", "=", "$expr$"],
            false,
            move |context, inputs| {
                let term = context.eval_expression_tree(&inputs[0])?.to_string();
                let definition = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let term_clone = term.clone();
                let definition_clone = definition.clone();
                let outcome = execute_with_timeout(move || {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let body = json!({
                        "definition": definition_clone,
                        "interval_days": 1,
                        "next_review": Utc::now().to_rfc3339(),
                    });
                    let id = upsert_fact(pool, bot_id, user_id, &term_clone, &body)?;
                    Ok(json!({
                        "kind": "flashcard",
                        "id": id.to_string(),
                        "term": term_clone,
                        "definition": definition_clone,
                        "interval_days": 1,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("FLASHCARD: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("FLASHCARD registration: {e}")))
        .ok();
}

fn register_curriculum(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(["CURRICULUM", "$expr$"], false, move |context, inputs| {
            let subject = context.eval_expression_tree(&inputs[0])?.to_string();
            let state = Arc::clone(&state_clone);
            let user_clone = user.clone();
            let subject_clone = subject.clone();
            let outcome = execute_with_timeout(move || {
                let pool = state.db_pool();
                let bot_id = bot_id_from_session(&user_clone);
                let user_id = user_id_from_session(&user_clone);
                let prompt = format!(
                    "Crie um currículo de aprendizado em português brasileiro sobre '{subject_clone}'. \
                     Responda APENAS com JSON {{\"modules\":[{{\"id\":1,\"title\":\"...\"}}]}} \
                     cobrindo 4 níveis: Fundamentos, Intermediário, Avançado, Maestria."
                );
                let fallback = json!({
                    "modules": [
                        {"id": 1, "title": "Fundamentos"},
                        {"id": 2, "title": "Intermediário"},
                        {"id": 3, "title": "Avançado"},
                        {"id": 4, "title": "Maestria"},
                    ]
                });
                let body = match llm_content(&state, &prompt, "").as_str() {
                    text if !text.is_empty() => serde_json::from_str::<Value>(text)
                        .unwrap_or(fallback.clone()),
                    _ => fallback,
                };
                let id = persist_lesson(pool, bot_id, user_id, &subject_clone, "curriculum", &body)?;
                Ok(json!({
                    "kind": "curriculum",
                    "id": id.to_string(),
                    "subject": subject_clone,
                    "modules": body.get("modules").cloned().unwrap_or(json!([])),
                }))
            });
            match outcome {
                Ok(v) => Ok(Dynamic::from(v.to_string())),
                Err(e) => Err(runtime_error(format!("CURRICULUM: {e}"))),
            }
        })
        .map_err(|e| runtime_error(format!("CURRICULUM registration: {e}")))
        .ok();
}

fn register_lesson(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["LESSON", "$expr$", "STAGE", "$expr$"],
            false,
            move |context, inputs| {
                let lesson_id = context.eval_expression_tree(&inputs[0])?.to_string();
                let stage = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let lesson_id_clone = lesson_id.clone();
                let stage_clone = stage.clone();
                let outcome = execute_with_timeout(move || {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let body = json!({
                        "objectives": ["understand", "apply", "analyze"],
                        "stage": stage_clone,
                    });
                    let id = persist_lesson(pool, bot_id, user_id, &lesson_id_clone, "lesson", &body)?;
                    Ok(json!({
                        "kind": "lesson",
                        "id": id.to_string(),
                        "stage": stage_clone,
                        "objectives": body.get("objectives").cloned().unwrap_or(json!([])),
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("LESSON: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("LESSON registration: {e}")))
        .ok();
}

fn register_progress(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(["PROGRESS", "OF", "$expr$"], false, move |context, inputs| {
            let student_str = context.eval_expression_tree(&inputs[0])?.to_string();
            let state = Arc::clone(&state_clone);
            let student = Uuid::parse_str(&student_str)
                .ok()
                .or(user_id_from_session(&user))
                .unwrap_or_else(Uuid::nil);
            let user_clone = user.clone();
            let outcome = execute_with_timeout(move || {
                let pool = state.db_pool();
                let bot_id = bot_id_from_session(&user_clone);
                let (facts, lessons) = progress_for(pool, bot_id, student)?;
                Ok(json!({
                    "kind": "progress",
                    "student": student.to_string(),
                    "facts_learned": facts,
                    "lessons_completed": lessons,
                    "average_score": 0.0,
                    "next_review": null,
                }))
            });
            match outcome {
                Ok(v) => Ok(Dynamic::from(v.to_string())),
                Err(e) => Err(runtime_error(format!("PROGRESS: {e}"))),
            }
        })
        .map_err(|e| runtime_error(format!("PROGRESS registration: {e}")))
        .ok();
}

fn register_remember_fact(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    engine
        .register_custom_syntax(
            ["REMEMBER", "FACT", "$expr$", "=", "$expr$"],
            false,
            move |context, inputs| {
                let key = context.eval_expression_tree(&inputs[0])?.to_string();
                let value = context.eval_expression_tree(&inputs[1])?.to_string();
                let state = Arc::clone(&state_clone);
                let user_clone = user.clone();
                let key_clone = key.clone();
                let value_clone = value.clone();
                let outcome = execute_with_timeout(move || {
                    let pool = state.db_pool();
                    let bot_id = bot_id_from_session(&user_clone);
                    let user_id = user_id_from_session(&user_clone);
                    let body = json!({
                        "value": value_clone,
                        "remembered_at": Utc::now().to_rfc3339(),
                    });
                    let id = upsert_fact(pool, bot_id, user_id, &key_clone, &body)?;
                    Ok(json!({
                        "kind": "fact",
                        "id": id.to_string(),
                        "key": key_clone,
                        "value": value_clone,
                        "stored": true,
                    }))
                });
                match outcome {
                    Ok(v) => Ok(Dynamic::from(v.to_string())),
                    Err(e) => Err(runtime_error(format!("REMEMBER FACT: {e}"))),
                }
            },
        )
        .map_err(|e| runtime_error(format!("REMEMBER FACT registration: {e}")))
        .ok();
}
