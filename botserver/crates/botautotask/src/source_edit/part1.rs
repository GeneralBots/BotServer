use super::*;

pub(super) fn fail(name: &str, message: &str, error: &str) -> Json<SourceResponse> {
    Json(SourceResponse {
        success: false,
        name: name.to_string(),
        content: None,
        committed: false,
        message: message.to_string(),
        error: Some(error.to_string()),
    })
}
/// `GET /api/autotask/source` — the committed content of a bot source file.
pub async fn get_source(
    State(api): State<Arc<AutoTaskApi>>,
    Query(query): Query<SourceQuery>,
) -> Json<SourceResponse> {
    let bot_id = canonical_bot_id(query.bot_id.clone());
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail(&query.name, "No source repository for this bot", "git sources unavailable"),
    };
    match sources.read_source(bot_id, &query.name) {
        Ok(Some(content)) => Json(SourceResponse {
            success: true,
            name: query.name,
            content: Some(content),
            committed: true,
            message: "Source loaded from the bot repository".to_string(),
            error: None,
        }),
        Ok(None) => fail(&query.name, "Source file not found", "not found"),
        Err(e) => {
            warn!("[autotask] read source {} failed: {e}", query.name);
            let error = err_msg("read_source", &*e);
            fail(&query.name, "Could not read the source file", &error)
        }
    }
}
/// `PUT /api/autotask/source` — commit an edited revision.
pub async fn put_source(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<UpdateSourceRequest>,
) -> Json<SourceResponse> {
    let bot_id = canonical_bot_id(req.bot_id.clone());
    write_source(&api, bot_id, &req.name, &req.content, req.message.as_deref(), "edit")
}
/// `POST /api/autotask/source/rephrase` — let the LLM rewrite the source.
pub async fn rephrase_source(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<RephraseSourceRequest>,
) -> Json<SourceResponse> {
    let bot_id = canonical_bot_id(req.bot_id.clone());
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail(&req.name, "No source repository for this bot", "git sources unavailable"),
    };
    let current = match sources.read_source(bot_id, &req.name) {
        Ok(Some(content)) => content,
        Ok(None) => return fail(&req.name, "Source file not found", "not found"),
        Err(e) => {
            warn!("[autotask] rephrase read {} failed: {e}", req.name);
            let error = err_msg("rephrase_read", &*e);
            return fail(&req.name, "Could not read the source file", &error);
        }
    };
    let rewritten = match call_llm(&api, bot_id, &rephrase_prompt(&req.name, &current, &req.instruction)).await {
        Ok(text) => strip_code_fence(&text),
        Err(e) => return fail(&req.name, "The model could not rewrite the automation", &e),
    };
    if rewritten.trim().is_empty() {
        return fail(&req.name, "The model returned an empty program", "empty completion");
    }
    if req.dry_run.unwrap_or(false) {
        return Json(SourceResponse {
            success: true,
            name: req.name,
            content: Some(rewritten),
            committed: false,
            message: "Rephrased source (not committed)".to_string(),
            error: None,
        });
    }
    write_source(&api, bot_id, &req.name, &rewritten, None, "rephrase")
}
