use super::*;

/// `GET /api/autotask/items/:id/source` — the committed `.bas` of one
/// `auto_tasks` row (the row's script path resolved through the repository).
pub async fn get_item_source(
    State(api): State<Arc<AutoTaskApi>>,
    Path(item_id): Path<String>,
) -> Json<ItemSourceResponse> {
    let item_id_for_fail = item_id.clone();
    let fail = move |message: &str, error: String| {
        Json(ItemSourceResponse {
            success: false,
            item_id: item_id_for_fail.clone(),
            name: String::new(),
            project_id: None,
            content: None,
            committed: false,
            message: message.to_string(),
            error: Some(error),
        })
    };
    let (name, row_bot_id) = {
        let pool = api.state().db_pool().clone();
        let item_id_for_db = item_id.clone();
        match tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().ok()?;
            #[derive(diesel::QueryableByName)]
            struct Row {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                bot_id: Uuid,
                #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
                basic_program: Option<String>,
            }
            diesel::sql_query(
                "SELECT bot_id, basic_program FROM auto_tasks WHERE id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(
                uuid::Uuid::parse_str(&item_id_for_db).ok()?,
            )
            .get_result::<Row>(&mut conn)
            .ok()
            .map(|r| {
                (
                    r.basic_program
                        .map(|p| p.rsplit('/').next().unwrap_or(&p).to_string()),
                    r.bot_id,
                )
            })
        })
        .await
        {
            Ok(Some((Some(name), bot))) => (name, bot),
            _ => return fail("This item has no recorded source path", "not found".to_string()),
        }
    };
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail("No source repository for this bot", "git sources unavailable".to_string()),
    };
    // The project owning the bot's repository, for the editor's project mode:
    // the vibe project whose name matches the bot's.
    let project_id = {
        let pool = api.state().db_pool().clone();
        let bot = row_bot_id;
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().ok()?;
            #[derive(diesel::QueryableByName)]
            struct IdRow {
                #[diesel(sql_type = diesel::sql_types::Uuid)]
                id: Uuid,
            }
            diesel::sql_query(
                "SELECT vp.id FROM vibe_projects vp \
                 JOIN bots b ON b.name = vp.name \
                 WHERE vp.project_type = 'bot' AND vp.source_control = 'git' \
                   AND b.id = $1 LIMIT 1",
            )
            .bind::<diesel::sql_types::Uuid, _>(bot)
            .get_result::<IdRow>(&mut conn)
            .ok()
            .map(|r| r.id)
        })
        .await
        .ok()
        .flatten()
    };
    let content = match sources.read_source(row_bot_id, &name) {
        Ok(content) => content,
        Err(e) => {
            warn!("[autotask] item source read {name} failed: {e}");
            return fail("Could not read the source file", err_msg("item_source", &*e));
        }
    };
    Json(ItemSourceResponse {
        success: true,
        item_id: item_id.clone(),
        name: name.clone(),
        project_id,
        content,
        committed: true,
        message: format!("Source of item {item_id}"),
        error: None,
    })
}
/// `GET /api/autotask/sources` — the bot's `.gbdialog` sources and the project
/// that owns the repository, so a row action can open one in the editor.
pub async fn list_sources(
    State(api): State<Arc<AutoTaskApi>>,
    Query(query): Query<BotQuery>,
) -> Json<SourceListResponse> {
    let bot_id = canonical_bot_id(query.bot_id.clone());
    let fail = |message: &str, error: String| {
        Json(SourceListResponse {
            success: false,
            project_id: None,
            files: Vec::new(),
            message: message.to_string(),
            error: Some(error),
        })
    };
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail("No source repository for this bot", "git sources unavailable".to_string()),
    };
    match sources.list_sources(bot_id) {
        Ok(listing) => Json(SourceListResponse {
            success: true,
            project_id: listing.project_id,
            message: format!("{} source file(s)", listing.files.len()),
            files: listing.files,
            error: None,
        }),
        Err(e) => {
            warn!("[autotask] list sources failed: {e}");
            let error = err_msg("list_sources", &*e);
            fail("Could not list the bot sources", error)
        }
    }
}
/// `POST /api/autotask/tables` — attach `BEGIN TABLE … END TABLE` blocks to the
/// bot's `.gbdialog/tables.bas`, so an automation that needs storage declares
/// its schema instead of embedding DDL in the tool script.
pub async fn attach_tables(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<AttachTablesRequest>,
) -> Json<SourceResponse> {
    const FILE: &str = "tables.bas";
    let bot_id = canonical_bot_id(req.bot_id.clone());
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail(FILE, "No source repository for this bot", "git sources unavailable"),
    };
    let attached = match sources.merge_tables(bot_id, &req.tables) {
        Ok(names) => names,
        Err(e) => {
            warn!("[autotask] attach tables failed: {e}");
            let error = err_msg("attach_tables", &*e);
            return fail(FILE, "Could not attach the schema", &error);
        }
    };
    let content = sources.read_source(bot_id, FILE).ok().flatten();
    if attached.is_empty() {
        return Json(SourceResponse {
            success: true,
            name: FILE.to_string(),
            content,
            committed: false,
            message: "Schema already declared every table".to_string(),
            error: None,
        });
    }
    info!("[autotask] attached tables to {FILE}: {attached:?}");
    Json(SourceResponse {
        success: true,
        name: FILE.to_string(),
        content,
        committed: true,
        message: format!("Attached to {FILE}: {}", attached.join(", ")),
        error: None,
    })
}
/// Commit `content` as `name` through the bot's source repository.
pub(super) fn write_source(
    api: &Arc<AutoTaskApi>,
    bot_id: Uuid,
    name: &str,
    content: &str,
    message: Option<&str>,
    verb: &str,
) -> Json<SourceResponse> {
    let sources = match api.state().source_ops() {
        Some(sources) => sources,
        None => return fail(name, "No source repository for this bot", "git sources unavailable"),
    };
    let commit_message = message
        .map(|m| m.to_string())
        .unwrap_or_else(|| format!("autotask: {verb} {name}"));
    let files = vec![(name.to_string(), content.to_string())];
    match sources.write_sources(bot_id, &files, &commit_message) {
        Ok(written) => {
            info!("[autotask] committed source {name} ({written:?})");
            Json(SourceResponse {
                success: true,
                name: name.to_string(),
                content: Some(content.to_string()),
                committed: true,
                message: format!("Committed to the bot repository: {name}"),
                error: None,
            })
        }
        Err(e) => {
            warn!("[autotask] commit source {name} failed: {e}");
            let error = err_msg("write_source", &*e);
            fail(name, "Could not commit the source file", &error)
        }
    }
}
/// Instruction mixing the current program, the requested change and the
/// response contract (program only, no prose, no code fences).
pub(super) fn rephrase_prompt(name: &str, current: &str, instruction: &str) -> String {
    format!(
        "You maintain a General Bots automation written in the BASIC dialect.\n\
         The file is '{name}' inside the bot's .gbdialog directory.\n\n\
         Requested change:\n{instruction}\n\n\
         Current program:\n```basic\n{current}\n```\n\n\
         Rewrite the whole program applying the change. Keep every declared\n\
         keyword, tool name and output channel working exactly as before unless\n\
         the change explicitly requires otherwise. Reply with the program only —\n\
         no explanation, no markdown fences."
    )
}
/// Run a one-shot completion against the bot's configured model.
pub(super) async fn call_llm(api: &Arc<AutoTaskApi>, bot_id: Uuid, prompt: &str) -> Result<String, String> {
    // Bot scope first, then the global one; never a hardcoded vendor model —
    // a gateway that does not serve it answers 404 and the edit silently dies.
    let mut model = String::new();
    let mut key = String::new();
    for id in [bot_id, uuid::Uuid::nil()] {
        if model.is_empty() {
            if let Ok(value) = api.config_ops().get_config(&id, "llm-model", None) {
                model = value.trim().to_string();
            }
        }
        if key.is_empty() {
            if let Ok(value) = api.config_ops().get_config(&id, "llm-key", None) {
                key = value.trim().to_string();
            }
        }
    }
    if model.is_empty() {
        return Err(format!(
            "no llm-model configured for bot {bot_id} (checked the bot scope, then the global one)"
        ));
    }
    let config = serde_json::json!({ "temperature": 0.2, "max_tokens": 4000 });
    crate::types::collect_llm_stream(api.llm_ops().as_ref(), prompt, &config, &model, &key, None)
        .await
        .map_err(|e| format!("llm call failed: {e}"))
}
/// Drop a surrounding markdown code fence from a completion.
pub(super) fn strip_code_fence(text: &str) -> String {
    let trimmed = text.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_string();
    }
    let without_open = match trimmed.find('\n') {
        Some(idx) => &trimmed[idx + 1..],
        None => return String::new(),
    };
    match without_open.rfind("```") {
        Some(idx) => without_open[..idx].trim_end().to_string(),
        None => without_open.trim_end().to_string(),
    }
}
