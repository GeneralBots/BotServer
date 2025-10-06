use log::info;

use crate::{shared::state::AppState, utils::call_llm};
use rhai::{Dynamic, Engine};

pub fn llm_keyword(state: &AppState, engine: &mut Engine) {
    let ai_config = state.config.clone().unwrap().ai.clone();

    engine
        .register_custom_syntax(
            &["LLM", "$expr$"], // Syntax: LLM "text to process"
            false,              // Expression, not statement
            move |context, inputs| {
                let text = context.eval_expression_tree(&inputs[0])?;
                let text_str = text.to_string();

                info!("LLM processing text: {}", text_str);

                // Use the same pattern as GET

                let fut = call_llm(&text_str, &ai_config);
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("LLM call failed: {}", e))?;

                Ok(Dynamic::from(result))
            },
        )
        .unwrap();
}
