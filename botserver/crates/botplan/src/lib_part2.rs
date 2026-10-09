use super::*;

pub(super) fn default_status() -> String {
    "todo".to_string()
}
pub(super) fn default_priority() -> String {
    "medium".to_string()
}
/// AI planning assistant: takes the current plan context and a free-form
/// instruction and asks the configured LLM to propose tasks. The LLM call is
/// built from environment configuration (`LLM_URL`, `LLM_KEY`, `LLM_MODEL`);
/// when those are absent the endpoint degrades gracefully instead of panicking.
pub async fn handle_plan_ai_assist(Json(req): Json<PlanAiRequest>) -> impl IntoResponse {
    let url = match std::env::var("LLM_URL") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            return Json(serde_json::json!({ "ok": false, "error": "llm_not_configured" }))
                .into_response();
        }
    };
    let key = std::env::var("LLM_KEY").unwrap_or_default();
    let model = match std::env::var("LLM_MODEL") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            return Json(serde_json::json!({ "ok": false, "error": "llm_model_not_configured" }))
                .into_response();
        }
    };

    let ptype: LLMProviderType = url.as_str().into();
    let provider = create_llm_provider(ptype, url, None, None);
    let config = Value::Null;

    let prompt = format!(
        "You are a project planning assistant. Given the project context and a user instruction, \
         propose concrete tasks as a JSON array. Each item must be an object with keys: \
         \"title\" (string), \"status\" (one of todo|inprogress|review|done), \
         \"priority\" (one of low|medium|high|urgent), \"assignee\" (string or null). \
         Respond with ONLY the JSON array, no markdown fences, no prose.\n\n\
         Project context:\n{}\n\nInstruction: {}",
        req.context, req.instruction
    );

    match provider.generate(&prompt, &config, &model, &key).await {
        Ok(text) => {
            let suggestions = parse_plan_suggestions(&text);
            Json(serde_json::json!({ "ok": true, "suggestions": suggestions })).into_response()
        }
        Err(e) => {
            log::error!("plan ai assist llm call failed: {e}");
            Json(serde_json::json!({ "ok": false, "error": "ai_assist_failed" })).into_response()
        }
    }
}
pub(super) fn parse_plan_suggestions(text: &str) -> Vec<PlanAiSuggestion> {
    let start = text.find('[');
    let end = text.rfind(']');
    if let (Some(s), Some(e)) = (start, end) {
        if e > s {
            if let Ok(v) = serde_json::from_slice::<Vec<PlanAiSuggestion>>(text[s..=e].as_bytes()) {
                return v;
            }
        }
    }
    Vec::new()
}
