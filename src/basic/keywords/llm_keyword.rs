use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::info;
use rhai::{Dynamic, Engine};

pub fn llm_keyword(state: &AppState, _user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(&["LLM", "$expr$"], false, move |context, inputs| {
            let text = context.eval_expression_tree(&inputs[0])?;
            let text_str = text.to_string();

            info!("LLM processing text: {}", text_str);

            let state_inner = state_clone.clone();
            let fut = async move {
                let prompt = text_str;
                state_inner
                    .llm_provider
                    .generate(&prompt, &serde_json::Value::Null)
                    .await
                    .map_err(|e| format!("LLM call failed: {}", e))
            };

            let result =
                tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))?;

            Ok(Dynamic::from(result))
        })
        .unwrap();
}
