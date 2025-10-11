use crate::email::{fetch_latest_sent_to, save_email_draft, SaveDraftRequest};
use crate::shared::state::AppState;
use crate::shared::models::UserSession;
use rhai::Dynamic;
use rhai::Engine;

pub fn create_draft_keyword(state: &AppState, user: UserSession, engine: &mut Engine) {
    let state_clone = state.clone();

    engine
        .register_custom_syntax(
            &["CREATE_DRAFT", "$expr$", ",", "$expr$", ",", "$expr$"],
            true,
            move |context, inputs| {
                let to = context.eval_expression_tree(&inputs[0])?.to_string();
                let subject = context.eval_expression_tree(&inputs[1])?.to_string();
                let reply_text = context.eval_expression_tree(&inputs[2])?.to_string();

                let fut = execute_create_draft(&state_clone, &to, &subject, &reply_text);
                let result =
                    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
                        .map_err(|e| format!("Draft creation error: {}", e))?;

                Ok(Dynamic::from(result))
            },
        )
        .unwrap();
}

async fn execute_create_draft(
    state: &AppState,
    to: &str,
    subject: &str,
    reply_text: &str,
) -> Result<String, String> {
    let get_result = fetch_latest_sent_to(&state.config.clone().unwrap().email, to).await;
    let email_body = if let Ok(get_result_str) = get_result {
        if !get_result_str.is_empty() {
            let email_separator = "<br><hr><br>";
            let formatted_reply_text = reply_text.to_string();
            let formatted_old_text = get_result_str.replace("\n", "<br>");
            let fixed_reply_text = formatted_reply_text.replace("FIX", "Fixed");
            format!(
                "{}{}{}",
                fixed_reply_text, email_separator, formatted_old_text
            )
        } else {
            reply_text.to_string()
        }
    } else {
        reply_text.to_string()
    };

    let draft_request = SaveDraftRequest {
        to: to.to_string(),
        subject: subject.to_string(),
        cc: None,
        text: email_body,
    };

    let save_result = save_email_draft(&state.config.clone().unwrap().email, &draft_request).await;
    match save_result {
        Ok(_) => Ok("Draft saved successfully".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
