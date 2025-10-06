use log::error;

use actix_web::{
    web::{self, Bytes},
    HttpResponse, Responder,
};
use anyhow::Result;
use futures::StreamExt;
use langchain_rust::{
    chain::{Chain, LLMChainBuilder},
    fmt_message, fmt_template,
    language_models::llm::LLM,
    llm::openai::OpenAI,
    message_formatter,
    prompt::HumanMessagePromptTemplate,
    prompt_args,
    schemas::messages::Message,
    template_fstring,
};

use crate::{state::AppState, utils::azure_from_config};

#[derive(serde::Deserialize)]
struct ChatRequest {
    input: String,
}

#[derive(serde::Serialize)]
struct ChatResponse {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<ChatAction>,
}

#[derive(serde::Serialize)]
#[serde(tag = "type", content = "content")]
enum ChatAction {
    ReplyEmail { content: String },
    // Add other action variants here as needed
}

#[actix_web::post("/chat")]
pub async fn chat(
    web::Json(request): web::Json<String>,
    state: web::Data<AppState>,
) -> Result<impl Responder, actix_web::Error> {
    let azure_config = azure_from_config(&state.config.clone().unwrap().ai);
    let open_ai = OpenAI::new(azure_config);

    // Parse the context JSON
    let context: serde_json::Value = match serde_json::from_str(&request) {
        Ok(ctx) => ctx,
        Err(_) => serde_json::json!({}),
    };

    // Check view type and prepare appropriate prompt
    let view_type = context
        .get("viewType")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let (prompt, might_trigger_action) = match view_type {
        "email" => (
            format!(
                "Respond to this email: {}. Keep it professional and concise. \
                If the email requires a response, provide one in the 'replyEmail' action format.",
                request
            ),
            true,
        ),
        _ => (request, false),
    };

    let response_text = match open_ai.invoke(&prompt).await {
        Ok(res) => res,
        Err(err) => {
            error!("Error invoking API: {}", err);
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to invoke OpenAI API",
            ));
        }
    };

    // Prepare response with potential action
    let mut chat_response = ChatResponse {
        text: response_text.clone(),
        action: None,
    };

    // If in email view and the response looks like an email reply, add action
    if might_trigger_action && view_type == "email" {
        chat_response.action = Some(ChatAction::ReplyEmail {
            content: response_text,
        });
    }

    Ok(HttpResponse::Ok().json(chat_response))
}

#[actix_web::post("/stream")]
pub async fn chat_stream(
    web::Json(request): web::Json<ChatRequest>,
    state: web::Data<AppState>,
) -> Result<impl Responder, actix_web::Error> {
    let azure_config = azure_from_config(&state.config.clone().unwrap().ai);
    let open_ai = OpenAI::new(azure_config);

    let prompt = message_formatter![
        fmt_message!(Message::new_system_message(
            "You are world class technical documentation writer."
        )),
        fmt_template!(HumanMessagePromptTemplate::new(template_fstring!(
            "{input}", "input"
        )))
    ];

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(open_ai)
        .build()
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let mut stream = chain
        .stream(prompt_args! { "input" => request.input })
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let actix_stream = async_stream::stream! {
        while let Some(result) = stream.next().await {
            match result {
                Ok(value) => yield Ok::<_, actix_web::Error>(Bytes::from(value.content)),
                Err(e) => yield Err(actix_web::error::ErrorInternalServerError(e)),
            }
        }
    };

    Ok(HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(actix_stream))
}
