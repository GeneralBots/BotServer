use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use rhai::{Dynamic, Engine};

pub fn llm_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["LLM", "$expr$"], false, move |context, inputs| {
            let text = context.eval_expression_tree(&inputs[0])?.to_string();

            info!("LLM processing text: {}", text);

            let state_inner = state_clone.clone();
            let fut = execute_llm_generation(state_inner, text);

            let result =
                tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                    .map_err(|e| format!("LLM generation failed: {}", e))?;

            Ok(Dynamic::from(result))
        })
        .unwrap();
}

pub async fn execute_llm_generation(
    state: AppState,
    prompt: String,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("Starting LLM generation for prompt: '{}'", prompt);

    state
        .llm_provider
        .generate(&prompt, &serde_json::Value::Null)
        .await
        .map_err(|e| format!("LLM call failed: {}", e).into())
}
