use super::*;

pub(crate) fn branch_scope(state: &Arc<AppState>, bot_uuid: &Uuid) -> Result<Uuid, String> {
    botbanking::cashflow::resolve_bot_scope(&state.conn, bot_uuid, None)
        .map(|s| s.branch_id)
        .ok_or_else(|| "bot not found".to_string())
}
/// Resolves an `@project-name` mention inside the intent text to
/// `(project_id, project_name)`. The chat composer prefills the running
/// app's name as a mention, so the mention text itself identifies the target
/// workspace — matching by name avoids the wrong-project edit that the
/// latest-project fallback caused (#e2e).
pub(crate) fn mentioned_project(
    state: &Arc<AppState>,
    bot_uuid: &Uuid,
    intent: &str,
) -> Result<Option<(String, String)>, String> {
    // First @token in the text (alphanumerics, dash, underscore, dot).
    let mention = intent
        .split(|c: char| c.is_whitespace() || c == ',' || c == '!' || c == '?' || c == '.')
        .find_map(|word| {
            let w = word.strip_prefix('@').filter(|s| !s.is_empty())?;
            (w.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'))
                .then(|| w.to_ascii_lowercase())
        });
    let Some(name) = mention else {
        return Ok(None);
    };
    let branch = branch_scope(state, bot_uuid).unwrap_or_else(|_| Uuid::nil());
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;
    let row = diesel::sql_query(
        "SELECT id AS project_id, name AS project_name FROM vibe_projects \
         WHERE lower(name) = $1 OR branch_id = ANY($2) AND lower(name) = $1 \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(name.clone())
    .bind::<diesel::sql_types::Array<diesel::sql_types::Uuid>, _>(vec![branch, Uuid::nil()])
    .get_result::<ProjectNameRow>(&mut conn)
    .ok();
    Ok(row.map(|r| (r.project_id.to_string(), r.project_name)))
}
/// Executes one registered provider action from the chat command path (#950).
///
/// The chat session is server-side and bot-bound: the tenant scope derives
/// exclusively from the `bots` row via `resolve_scope_parts` (see the
/// trade-off documented there), while the actor stays the authenticated
/// `user_id`. Credentials load strictly from Vault inside
/// `invoke_registered` and are stripped from every response.
#[cfg(feature = "integrations")]
pub(super) async fn integrations_invoke(
    state: &Arc<AppState>,
    bot_uuid: Uuid,
    user_id: Uuid,
    provider: &str,
    action: &str,
    params: &Value,
) -> Result<Value, String> {
    let scope = botintegrations::scope::resolve_scope_parts(&state.conn, user_id, bot_uuid)
        .map_err(|error| match error {
            botintegrations::error::IntegrationError::NotFound => {
                "integration provider requires an active bot".to_string()
            }
            other => format!("integration scope rejected: {other}"),
        })?;
    let secrets_manager = botcoresecrets::SecretsManager::get_clone()
        .map_err(|_| "credential store unavailable; request rejected".to_string())?;
    let integration_state = botintegrations::IntegrationState::new(
        state.conn.clone(),
        botintegrations::secrets::ConnectionVault::new(secrets_manager),
    );
    let outcome = botintegrations::providers::invoke_registered(
        &integration_state,
        &scope,
        provider,
        action,
        params,
    )
    .await?;
    Ok(json!({
        "provider": provider,
        "action": action,
        "summary": outcome.summary,
        "data": outcome.data,
        "truncated": outcome.truncated,
    }))
}
