use super::*;

pub fn configure(state: Arc<OAuthState_>) -> Router {
    Router::new()
        .route("/auth/oauth/providers", get(list_providers))
        .route("/auth/oauth/:provider", get(start_oauth))
        .route("/auth/oauth/:provider/callback", get(oauth_callback))
        .with_state(state)
}
pub(super) async fn list_providers(State(state): State<Arc<OAuthState_>>) -> impl IntoResponse {
    let bot_config = get_bot_config(&state).await;
    let base_url = &state.base_url;

    let enabled = get_enabled_providers(&bot_config, base_url);

    let providers: Vec<ProviderInfo> = enabled
        .iter()
        .map(|config| ProviderInfo {
            id: config.provider.to_string().to_lowercase(),
            name: config.provider.display_name().to_string(),
            icon: config.provider.icon().to_string(),
            login_url: format!("/auth/oauth/{}", config.provider.to_string().to_lowercase()),
        })
        .collect();

    Json(EnabledProvidersResponse { providers })
}
pub(super) async fn start_oauth(
    State(state): State<Arc<OAuthState_>>,
    Path(provider_name): Path<String>,
    Query(params): Query<OAuthStartParams>,
) -> Response {
    let Some(provider) = OAuthProvider::parse(&provider_name) else {
        return (
            StatusCode::BAD_REQUEST,
            Html(format!(
                r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
                <h1>Invalid OAuth Provider</h1><p>Provider '{}' is not supported.</p>
                <a href="/auth/login">Back to Login</a></body></html>"#,
                html_escape(&provider_name)
            )),
        )
            .into_response();
    };

    let bot_config = get_bot_config(&state).await;
    let base_url = &state.base_url;

    let config = match load_oauth_config(provider, &bot_config, base_url) {
        Some(c) if c.is_valid() => c,
        _ => {
            warn!("OAuth provider {} is not configured or enabled", provider);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Html(format!(
                    r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
                    <h1>OAuth Provider Not Configured</h1><p>Login with {} is not currently enabled.</p>
                    <a href="/auth/login">Back to Login</a></body></html>"#,
                    provider.display_name()
                )),
            )
                .into_response();
        }
    };

    let oauth_state = OAuthState::new(provider, params.redirect).with_pkce();
    let state_encoded = oauth_state.encode();

    debug!("OAuth state created for provider {}", provider);

    let auth_url = provider.build_auth_url(&config, &state_encoded, oauth_state.pkce_verifier.as_deref());

    info!("Starting OAuth flow for {} - redirecting to provider", provider);

    Redirect::temporary(&auth_url).into_response()
}
pub(super) async fn oauth_callback(
    State(state): State<Arc<OAuthState_>>,
    Path(provider_name): Path<String>,
    Query(params): Query<OAuthCallbackParams>,
) -> Response {
    if let Some(error) = &params.error {
        let description = params.error_description.as_deref().unwrap_or("Unknown error");
        warn!("OAuth error from provider: {} - {}", error, description);
        return (
            StatusCode::UNAUTHORIZED,
            Html(format!(
                r#"<!DOCTYPE html><html><head><title>Login Failed</title></head><body>
                <h1>Login Failed</h1><p>The OAuth provider returned an error: {}</p><p>{}</p>
                <a href="/auth/login">Try Again</a></body></html>"#,
                error, description
            )),
        )
            .into_response();
    }

    let Some(code) = &params.code else {
        return (
            StatusCode::BAD_REQUEST,
            Html(r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
            <h1>Missing Authorization Code</h1><a href="/auth/login">Try Again</a></body></html>"#.to_string()),
        )
            .into_response();
    };

    let Some(state_param) = &params.state else {
        return (
            StatusCode::BAD_REQUEST,
            Html(r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
            <h1>Missing State Parameter</h1><a href="/auth/login">Try Again</a></body></html>"#.to_string()),
        )
            .into_response();
    };

    let Some(oauth_state) = OAuthState::decode(state_param) else {
        warn!("Failed to decode OAuth state parameter");
        return (
            StatusCode::BAD_REQUEST,
            Html(r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
            <h1>Invalid State</h1><a href="/auth/login">Try Again</a></body></html>"#.to_string()),
        )
            .into_response();
    };

    if oauth_state.is_expired() {
        warn!("OAuth state expired");
        return (
            StatusCode::BAD_REQUEST,
            Html(r#"<!DOCTYPE html><html><head><title>Session Expired</title></head><body>
            <h1>Session Expired</h1><a href="/auth/login">Try Again</a></body></html>"#.to_string()),
        )
            .into_response();
    }

    let Some(provider) = OAuthProvider::parse(&provider_name) else {
        return (StatusCode::BAD_REQUEST, Html("Invalid provider".to_string())).into_response();
    };

    if provider != oauth_state.provider {
        warn!("Provider mismatch: URL says {}, state says {}", provider, oauth_state.provider);
        return (
            StatusCode::BAD_REQUEST,
            Html(r#"<!DOCTYPE html><html><head><title>Error</title></head><body>
            <h1>Provider Mismatch</h1><a href="/auth/login">Try Again</a></body></html>"#.to_string()),
        )
            .into_response();
    }

    let bot_config = get_bot_config(&state).await;
    let base_url = &state.base_url;

    let Some(config) = load_oauth_config(provider, &bot_config, base_url) else {
        return (StatusCode::SERVICE_UNAVAILABLE, Html("OAuth provider not configured".to_string())).into_response();
    };

    let http_client = reqwest::Client::new();
    let token = match provider
        .exchange_code(&config, code, &http_client, oauth_state.pkce_verifier.as_deref())
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("Failed to exchange OAuth code: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(r#"<!DOCTYPE html><html><head><title>Login Failed</title></head><body>
                <h1>Login Failed</h1><p>Failed to complete the OAuth login: {}</p>
                <a href="/auth/login">Try Again</a></body></html>"#, e)),
            ).into_response();
        }
    };

    let user_info = match provider.fetch_user_info(&token.access_token, &http_client).await {
        Ok(info) => info,
        Err(e) => {
            log::error!("Failed to fetch user info: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,
                Html(format!(r#"<!DOCTYPE html><html><head><title>Login Failed</title></head><body>
                <h1>Login Failed</h1><p>Failed to retrieve user information: {}</p>
                <a href="/auth/login">Try Again</a></body></html>"#, e)),
            ).into_response();
        }
    };

    info!(
        "OAuth login successful for {} user: {} ({})",
        provider,
        user_info.name.as_deref().unwrap_or("unknown"),
        user_info.email.as_deref().unwrap_or("no email")
    );

    let user_id = match create_or_get_oauth_user(&state, &user_info).await {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to create/get user: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create user account".to_string())).into_response();
        }
    };

    let session_token = match create_user_session(&state, user_id).await {
        Ok(token) => token,
        Err(e) => {
            log::error!("Failed to create session: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to create session".to_string())).into_response();
        }
    };

    let redirect_url = oauth_state.redirect_after.unwrap_or_else(|| "/".to_string());

    debug!("OAuth complete, redirecting to {}", redirect_url);

    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, redirect_url.clone())
        .header(
            header::SET_COOKIE,
            format!("session={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400", session_token),
        )
        .body(axum::body::Body::empty())
        .unwrap_or_else(|e| {
            log::error!("Failed to build OAuth redirect response: {}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::Body::empty())
                .unwrap_or_default()
        })
}
pub(super) async fn get_bot_config(state: &OAuthState_) -> HashMap<String, String> {
    let conn = state.conn.clone();
    let active_bot_id: Uuid = match tokio::task::spawn_blocking(move || {
        use diesel::prelude::*;
        use crate::schema::bots;

        let mut db_conn = conn.get().ok()?;
        bots::dsl::bots
            .filter(bots::dsl::is_active.eq(true))
            .select(bots::dsl::id)
            .first(&mut db_conn)
            .optional()
            .ok()?
    })
    .await
    {
        Ok(Some(id)) => id,
        _ => return HashMap::new(),
    };

    if let Some(vault_configs) = read_bot_config_from_vault(&active_bot_id) {
        vault_configs
    } else {
        HashMap::new()
    }
}
pub(super) fn read_bot_config_from_vault(bot_id: &Uuid) -> Option<HashMap<String, String>> {
    use botcoresecrets::manager::SecretsManager;
    let sm = SecretsManager::get_clone().ok()?;
    if !sm.is_enabled() {
        return None;
    }
    let path = format!("gbo/{}/{}/{}", uuid::Uuid::nil(), uuid::Uuid::nil(), bot_id);
    let sm_clone = sm.clone();
    let path_clone = path.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(async move { sm_clone.get_secret(&path_clone).await.ok() })
        } else {
            None
        };
        let _ = tx.send(result);
    });
    rx.recv().ok().flatten()
}
