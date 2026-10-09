use super::*;

/// GET /api/bots/:bot_id/integrations/oauth/:provider/callback
///
/// Provider browser redirect carrying no JWT. Authenticity comes from the
/// HMAC-signed state issued by [`start`]; the exchanged access token is
/// stored strictly in Vault and an active connection row is created.
pub async fn callback(
    State(state): State<Arc<IntegrationState>>,
    Path((bot_id, provider)): Path<(String, String)>,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, Response> {
    if let Some(error) = query.error {
        return Err(error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation(error)));
    }
    let (code, signed) = match (query.code, query.state) {
        (Some(code), Some(state)) => (code, state),
        _ => {
            return Err(error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation(
                "missing code or state".to_string(),
            )))
        }
    };
    let (encoded, signature) = signed.rsplit_once('.').ok_or_else(|| {
        error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("malformed state".to_string()))
    })?;
    let expected = sign(encoded, &signing_key()?)?;
    if expected != signature {
        return Err(error_response(StatusCode::UNAUTHORIZED, IntegrationError::Validation(
            "state signature mismatch".to_string(),
        )));
    }
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
        error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("malformed state payload".to_string()))
    })?;
    let payload: Value = serde_json::from_slice(&raw).map_err(|_| {
        error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("malformed state payload".to_string()))
    })?;
    let age = chrono::Utc::now().timestamp() - payload.get("t").and_then(Value::as_i64).unwrap_or(i64::MAX);
    if !(0..=STATE_MAX_AGE_SECS).contains(&age) {
        return Err(error_response(StatusCode::UNAUTHORIZED, IntegrationError::Validation("state expired".to_string())));
    }
    if payload.get("p").and_then(Value::as_str) != Some(provider.as_str()) {
        return Err(error_response(StatusCode::UNAUTHORIZED, IntegrationError::Validation(
            "state provider mismatch".to_string(),
        )));
    }

    let config = provider_config(&provider).ok_or_else(|| {
        error_response(StatusCode::NOT_FOUND, IntegrationError::Validation("unknown oauth provider".to_string()))
    })?;

    let user_id = Uuid::parse_str(payload.get("u").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("invalid state user".to_string())))?;
    let org_id = Uuid::parse_str(payload.get("o").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("invalid state org".to_string())))?;
    let branch_id = Uuid::parse_str(payload.get("br").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("invalid state branch".to_string())))?;
    let bot_uuid = Uuid::parse_str(payload.get("b").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("invalid state bot".to_string())))?;
    let base = payload
        .get("base")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("https://") || value.starts_with("http://localhost"))
        .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, IntegrationError::Validation("invalid state base".to_string())))?
        .to_string();

    let mut conn = state.pool.get().map_err(|_| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable)
    })?;
    #[derive(diesel::QueryableByName)]
    struct BotOrgRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: Uuid,
    }
    let row: BotOrgRow = diesel::sql_query(
        "SELECT b.org_id AS org_id, b.branch_id AS branch_id \
         FROM bots b INNER JOIN branches br ON br.id = b.branch_id AND br.is_active = TRUE AND br.org_id = b.org_id \
         WHERE b.id = $1 AND b.is_active = TRUE LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_uuid)
    .get_result(&mut conn)
    .optional()
    .map_err(|_| error_response(StatusCode::UNAUTHORIZED, IntegrationError::UnauthorizedScope))?
    .ok_or_else(|| error_response(StatusCode::UNAUTHORIZED, IntegrationError::UnauthorizedScope))?;
    if row.org_id != org_id || row.branch_id != branch_id {
        return Err(error_response(StatusCode::UNAUTHORIZED, IntegrationError::UnauthorizedScope));
    }
    log::debug!("oauth callback scope validated for bot {bot_uuid}");
    drop(conn);

    let scope = ConnectionScope {
        user_id,
        org_id,
        branch_id,
        bot_id: bot_uuid,
    };

    let vault_path = format!(
        "gbo/{}/{}/{}/integrations/oauth/{provider}",
        scope.org_id, scope.branch_id, scope.bot_id
    );
    let client = state.vault.load_strict(&vault_path).await.map_err(|_| {
        error_response(StatusCode::PRECONDITION_FAILED, IntegrationError::Validation(
            "provider oauth client credentials are not configured for this branch".to_string(),
        ))
    })?;
    let client_id = client
        .get("client_id")
        .and_then(Value::as_str)
        .ok_or_else(|| error_response(StatusCode::PRECONDITION_FAILED, IntegrationError::Validation("client_id missing".to_string())))?
        .to_string();
    let client_secret = client
        .get("client_secret")
        .and_then(Value::as_str)
        .ok_or_else(|| error_response(StatusCode::PRECONDITION_FAILED, IntegrationError::Validation("client_secret missing".to_string())))?
        .to_string();
    let redirect_uri = format!("{base}/api/bots/{bot_id}/integrations/oauth/{provider}/callback");

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;
    let mut outgoing = http
        .post(config.token_url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ]);
    if config.basic_client_auth {
        let basic = base64::engine::general_purpose::STANDARD.encode(format!("{client_id}:{client_secret}"));
        outgoing = outgoing.header(header::AUTHORIZATION, format!("Basic {basic}"));
    }
    let response = outgoing.send().await.map_err(|error| {
        log::warn!("oauth token exchange failed for {provider}: {error}");
        error_response(StatusCode::BAD_GATEWAY, IntegrationError::VaultUnavailable)
    })?;
    let status = response.status();
    let body: Value = response.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        log::warn!("oauth token exchange returned {status} for {provider}");
        return Err(error_response(StatusCode::BAD_GATEWAY, IntegrationError::VaultUnavailable));
    }
    // Meta answers the code exchange with a token that lives about an hour.
    // Instagram publishing needs a Page or system-user token, and the Page
    // token is derived at call time from a long-lived user token, so upgrade
    // before storing. A failed upgrade keeps the short-lived token.
    let short_lived = body
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| error_response(StatusCode::BAD_GATEWAY, IntegrationError::VaultUnavailable))?
        .to_string();
    let mut expires_in = body.get("expires_in").and_then(Value::as_i64);
    let access_token = if is_meta_provider(&provider) {
        match exchange_long_lived_meta_token(
            &http,
            &config,
            &client_id,
            &client_secret,
            &short_lived,
        )
        .await
        {
            Some((long_lived, long_expires_in)) => {
                expires_in = long_expires_in.or(expires_in);
                long_lived
            }
            None => short_lived,
        }
    } else {
        short_lived
    };
    let refresh_token = body.get("refresh_token").and_then(Value::as_str).map(str::to_string);
    let granted_scopes = body.get("scope").cloned().unwrap_or(Value::Null);

    let connection_id = Uuid::now_v7();
    let mut envelope = json!({ "token": access_token });
    if let Some(refresh) = &refresh_token {
        envelope["refresh_token"] = Value::String(refresh.clone());
    }
    let stored_path = state
        .vault
        .store(&scope, connection_id, &envelope)
        .await
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable))?;

    let mut conn = state.pool.get().map_err(|_| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable)
    })?;
    repository::insert(
        &mut conn,
        &scope,
        &repository::NewConnectionInsert {
            connection_id,
            provider_slug: &provider,
            display_name: &format!("{provider} via oauth"),
            auth_kind: "oauth2",
            vault_path: &stored_path,
            granted_scopes: &granted_scopes,
            visibility: "private",
            configuration: &json!({"via": "authorization-code"}),
            expires_at: expires_in.and_then(|seconds| {
                chrono::Duration::try_seconds(seconds).map(|duration| chrono::Utc::now() + duration)
            }),
        },
    )
    .map_err(|error| {
        log::error!("oauth connection insert failed for {provider}: {error:?}");
        error_response(StatusCode::INTERNAL_SERVER_ERROR, IntegrationError::VaultUnavailable)
    })?;
    let event = repository::NewConnectionEvent {
        connection_id: Some(connection_id),
        actor_user_id: user_id,
        event_type: "connect",
        outcome: "success",
        risk_level: "medium",
        metadata: &json!({"method": "oauth2"}),
    };
    repository::record_event(&mut conn, &scope, &event).ok();
    drop(conn);

    let return_to = payload
        .get("r")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with('/'))
        .unwrap_or("/integrations")
        .to_string();
    Ok(Redirect::to(&return_to).into_response())
}
