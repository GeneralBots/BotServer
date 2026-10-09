use super::*;

/// GET /api/bots/:bot_id/integrations/oauth/:provider/start
///
/// Redirects the caller to the provider consent screen. Client credentials
/// load strictly from the branch-scoped Vault path; the signed state carries
/// the tenant scope and expires after a short window.
pub async fn start(
    State(state): State<Arc<IntegrationState>>,
    user: AuthenticatedUser,
    Path((bot_id, provider)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    Query(query): Query<StartQuery>,
) -> Result<Response, Response> {
    // These two are connected centrally (Drive -> External) and their actions
    // read that shared connection. A per-bot credential here would be a second
    // token store for the same account, silently out of sync with the one the
    // Drive app browses.
    if crate::external_mirror::is_mirrored(&provider) {
        return Err(error_response(StatusCode::CONFLICT, IntegrationError::Validation(
            format!("{provider} is connected from Drive -> External and shared with every bot in this branch")
        )));
    }
    let config = provider_config(&provider).ok_or_else(|| {
        error_response(StatusCode::NOT_FOUND, IntegrationError::Validation("unknown oauth provider".to_string()))
    })?;
    let uuid = parse_uuid(&bot_id)?;
    let scope = resolve_scope(&state.pool, &user, uuid)
        .map_err(|error| error_response(StatusCode::FORBIDDEN, error))?;

    let vault_path = format!(
        "gbo/{}/{}/{}/integrations/oauth/{provider}",
        scope.org_id, scope.branch_id, scope.bot_id
    );
    let client_config = state.vault.load_strict(&vault_path).await.map_err(
        |_| {
            log::warn!("oauth start without branch configuration at {vault_path}");
            error_response(StatusCode::PRECONDITION_FAILED, IntegrationError::Validation(
                "this branch has no oauth client credentials configured for the provider".to_string(),
            ))
        },
    )?;
    let client_id = client_config
        .get("client_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            error_response(StatusCode::PRECONDITION_FAILED, IntegrationError::Validation(
                "client_id missing from the provider oauth envelope".to_string(),
            ))
        })?;

    let base = request_base(&headers).ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("request host missing".to_string()))
    })?;
    let redirect_uri = format!("{base}/api/bots/{bot_id}/integrations/oauth/{provider}/callback");

    let state_payload = json!({
        "u": scope.user_id,
        "o": scope.org_id,
        "br": scope.branch_id,
        "b": scope.bot_id,
        "p": provider,
        "base": base,
        "r": query.return_to,
        "t": chrono::Utc::now().timestamp(),
    });
    let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(state_payload.to_string());
    let signature = sign(&encoded, &signing_key()?)?;

    let mut url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&state={}.{}",
        config.authorize_url,
        urlencode(client_id),
        urlencode(&redirect_uri),
        urlencode(&encoded),
        urlencode(&signature),
    );
    if !config.scopes.is_empty() {
        url.push_str("&scope=");
        url.push_str(&urlencode(config.scopes));
    }
    Ok(Redirect::to(&url).into_response())
}
