use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{error, info};
use rhai::{Dynamic, Engine};
use std::sync::Arc;
use std::time::Duration;

/// Registers the `LLM` keyword for Rhai scripts.
/// Example usage inside Rhai:
/// ```rhai
/// result = LLM "Summarize the following text about AI:";
/// ```
pub fn llm_keyword(state: Arc<AppState>, _user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);

    engine
        .register_custom_syntax(&["LLM", "$expr$"], false, move |context, inputs| {
            let text = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("LLM processing text: {}", text);

            let state_for_thread = Arc::clone(&state_clone);
            let prompt = build_llm_prompt(&text);

            // ---- safe runtime isolation: no deadlocks possible ----
            let (tx, rx) = std::sync::mpsc::channel();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build();

                let send_err = if let Ok(rt) = rt {
                    let result = rt.block_on(async move {
                        execute_llm_generation(state_for_thread, prompt).await
                    });
                    tx.send(result).err()
                } else {
                    tx.send(Err("failed to build tokio runtime".into())).err()
                };

                if send_err.is_some() {
                    error!("Failed to send LLM thread result");
                }
            });

            match rx.recv_timeout(Duration::from_secs(500)) {
                Ok(Ok(result)) => Ok(Dynamic::from(result)),
                Ok(Err(e)) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    e.to_string().into(),
                    rhai::Position::NONE,
                ))),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                        "LLM generation timed out".into(),
                        rhai::Position::NONE,
                    )))
                }
                Err(e) => Err(Box::new(rhai::EvalAltResult::ErrorRuntime(
                    format!("LLM thread failed: {e}").into(),
                    rhai::Position::NONE,
                ))),
            }
        })
        .unwrap();
}

/// Builds a consistent LLM prompt used by all Rhai scripts.
/// You can change the style/structure here to guide the model's behavior.
fn build_llm_prompt(user_text: &str) -> String {
    format!(
        "You are a AI assistant in form of KEYWORD called LLM
         running inside a General Bots BASIC environment.
Task: Process and respond concisely to the following call to x = LLM 'prompt' syntax.
---
User Input:
{}
---
Respond clearly and accurately in the same language as the input.",
        user_text.trim()
    )
}

/// Runs the async LLM provider call safely.
pub async fn execute_llm_generation(
    state: Arc<AppState>,
    prompt: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting LLM generation for prompt: '{}'", prompt);

    state
        .llm_provider
        .generate(&prompt, &serde_json::Value::Null)
        .await
        .map_err(|e| format!("LLM call failed: {}", e).into())
}
