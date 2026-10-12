use crate::types::{AiRequest, AiResponse, DocsAiRequest, DocsAiResponse};
use axum::{extract::State, http::StatusCode, Json, response::IntoResponse};
use std::sync::Arc;

use crate::state::DocState;

/// Calls the configured LLM (LLM_URL/LLM_KEY/LLM_MODEL env vars) with a
/// system + user prompt and returns the assistant text. Returns `Err(None)`
/// when the LLM is not configured so callers can fall back to local logic.
async fn llm_complete(system_prompt: &str, user_prompt: &str) -> Result<String, Option<String>> {
    let llm_url = std::env::var("LLM_URL").unwrap_or_default();
    let llm_key = std::env::var("LLM_KEY").unwrap_or_default();
    let llm_model = std::env::var("LLM_MODEL").unwrap_or_default();
    if llm_url.is_empty() || llm_key.is_empty() {
        return Err(None); // LLM not configured — caller falls back
    }
    let model = if llm_model.is_empty() { "default".to_string() } else { llm_model };

    let body = serde_json::json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ],
        "temperature": 0.3,
        "response_format": { "type": "json_object" }
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| Some(format!("Failed to build LLM client: {e}")))?;

    let resp = client
        .post(&llm_url)
        .header("Authorization", format!("Bearer {llm_key}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| Some(format!("LLM request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(Some(format!("LLM API returned {status}: {text}")));
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Some(format!("Failed to parse LLM response: {e}")))?;

    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Some("No content in LLM response".to_string()))?;

    // Some providers return JSON-wrapped text; unwrap the common shapes.
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap_or_default();
    if let Some(text) = parsed["response"].as_str() {
        return Ok(text.to_string());
    }
    if let Some(text) = parsed["result"].as_str() {
        return Ok(text.to_string());
    }
    Ok(content)
}

fn fallback_docs_ai(command: &str) -> String {
    if command.contains("summarize") || command.contains("summary") {
        "I've created a summary of your document. The key points are highlighted above."
    } else if command.contains("expand") || command.contains("longer") {
        "I've expanded the selected text with more details and examples."
    } else if command.contains("shorter") || command.contains("concise") {
        "I've made the text more concise while preserving the key information."
    } else if command.contains("formal") {
        "I've rewritten the text in a more formal, professional tone."
    } else if command.contains("casual") || command.contains("friendly") {
        "I've rewritten the text in a more casual, friendly tone."
    } else if command.contains("grammar") || command.contains("fix") {
        "I've corrected the grammar and spelling errors in your text."
    } else if command.contains("translate") {
        "I've translated the selected text. Please specify the target language if needed."
    } else if command.contains("improve") {
        "I've improved the selected text for clarity, grammar, and flow."
    } else if command.contains("simplify") {
        "I've simplified the selected text using clearer, more concise language."
    } else if command.contains("bullet") || command.contains("list") {
        "I've converted the text into a bulleted list format."
    } else if command.contains("help") {
        "I can help you with:\n\u{2022} Summarize text\n\u{2022} Expand or shorten content\n\u{2022} Fix grammar\n\u{2022} Change tone (formal/casual)\n\u{2022} Translate text\n\u{2022} Convert to bullet points"
    } else {
        "I understand you want help with your document. Try commands like 'summarize', 'make shorter', 'fix grammar', or 'make formal'."
    }
    .to_string()
}

pub async fn handle_docs_ai(
    State(_state): State<Arc<DocState>>,
    Json(req): Json<DocsAiRequest>,
) -> impl IntoResponse {
    // The editor sends `{id, action, prompt}`; the natural-language `command`
    // is optional. Prefer the action when present so both payloads work.
    let command = req
        .command
        .clone()
        .unwrap_or_else(|| req.action.clone().unwrap_or_default())
        .to_lowercase();
    let selected = req.selected_text.clone().unwrap_or_default();
    let text = req.text.clone().unwrap_or(selected);
    let prompt = req.prompt.clone().unwrap_or_default();

    if (!text.trim().is_empty() || !prompt.trim().is_empty()) && !command.contains("help") {
        let system_prompt = "You are an AI writing assistant embedded in a document editor. \
            Apply the user's requested edit to the provided text and return ONLY a JSON object \
            with a single key \"response\" containing the resulting text.";
        let instruction = if prompt.trim().is_empty() { command.clone() } else { prompt.clone() };
        let user_prompt = format!("Command: {}\n\nText:\n{}", instruction, text);
        match llm_complete(system_prompt, &user_prompt).await {
            Ok(result) => {
                return Json(DocsAiResponse {
                    response: result,
                    result: None,
                });
            }
            Err(Some(e)) => log::error!("Docs AI command failed, using fallback: {e}"),
            Err(None) => {}
        }
    }

    Json(DocsAiResponse {
        response: fallback_docs_ai(&command),
        result: None,
    })
}

fn fallback_ai(req: &AiRequest) -> AiResponse {
    let text = req.selected_text.clone().unwrap_or_default();
    let content = match req.action.as_str() {
        "summarize" if text.len() > 200 => format!(
            "Summary: {}...",
            botlib::text::truncate_at_char_boundary(&text, 200)
        ),
        "summarize" => format!("Summary: {}", text),
        "expand" => format!("{}\n\n[Additional context and details would be added here by AI]", text),
        "translate" => {
            let lang = req.translate_lang.clone().unwrap_or_else(|| "English".to_string());
            format!("[Translated to {}]: {}", lang, text)
        }
        _ => text,
    };
    AiResponse {
        result: "success".to_string(),
        content,
        error: None,
    }
}

async fn ai_action(req: &AiRequest, system_prompt: &str, user_prompt: &str) -> AiResponse {
    match llm_complete(system_prompt, user_prompt).await {
        Ok(content) => AiResponse {
            result: "success".to_string(),
            content,
            error: None,
        },
        Err(Some(e)) => {
            log::error!("Docs AI ({}) failed: {e}", req.action);
            let mut resp = fallback_ai(req);
            resp.error = Some(format!("LLM unavailable, using local fallback: {e}"));
            resp
        }
        Err(None) => fallback_ai(req),
    }
}

fn text_or_selected(req: &AiRequest) -> String {
    req.selected_text.clone().unwrap_or_default()
}

pub async fn handle_ai_summarize(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let system = "You are a summarization assistant. Summarize the given text concisely, \
        preserving the key facts. Return ONLY a JSON object with key \"response\" containing the summary.";
    let user = format!("Summarize this text:\n\n{}", text);
    Ok(Json(ai_action(&req, system, &user).await))
}

pub async fn handle_ai_expand(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let system = "You are a writing assistant. Expand the given text with additional relevant \
        details and examples, keeping the original meaning. Return ONLY a JSON object with key \"response\".";
    let user = format!("Expand this text:\n\n{}", text);
    Ok(Json(ai_action(&req, system, &user).await))
}

pub async fn handle_ai_improve(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let system = "You are a professional editor. Improve the given text: fix grammar, spelling, \
        clarity and flow while preserving meaning. Return ONLY a JSON object with key \"response\".";
    let user = format!("Improve this text:\n\n{}", text);
    Ok(Json(ai_action(&req, system, &user).await))
}

pub async fn handle_ai_simplify(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let system = "You are a plain-language editor. Rewrite the given text more simply and concisely, \
        using everyday words. Return ONLY a JSON object with key \"response\".";
    let user = format!("Simplify this text:\n\n{}", text);
    Ok(Json(ai_action(&req, system, &user).await))
}

pub async fn handle_ai_translate(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let lang = req.translate_lang.clone().unwrap_or_else(|| "English".to_string());
    let system = "You are a professional translator. Translate the given text faithfully, \
        preserving tone and meaning. Return ONLY a JSON object with key \"response\".";
    let user = format!("Translate this text into {lang}:\n\n{}", text);
    Ok(Json(ai_action(&req, system, &user).await))
}

pub async fn handle_ai_custom(
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = text_or_selected(&req);
    let system = "You are an AI writing assistant. Apply the user's requested transformation to the \
        given text. Return ONLY a JSON object with key \"response\" containing the resulting text.";
    let user = format!("Request: {}\n\nText:\n{}", req.prompt, text);
    Ok(Json(ai_action(&req, system, &user).await))
}
