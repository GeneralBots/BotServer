use super::*;

pub async fn logout(
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<Json<LogoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "))
        .map(String::from);

    if let Some(ref token_str) = token {
        let mut cache = SESSION_CACHE.write().await;
        if cache.remove(token_str).is_some() {
            info!("User logged out, session removed from cache");
        } else {
            info!("User logged out (session was not in cache)");
        }
        remove_persisted_session(token_str);
    }

    Ok(Json(LogoutResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    }))
}
pub async fn get_current_user(
    State(_state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Json<CurrentUserResponse> {
    let session_token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "));

    match session_token {
        None => {
            info!("get_current_user: no authorization header - returning anonymous user");
            Json(CurrentUserResponse {
                id: None,
                username: None,
                email: None,
                first_name: None,
                last_name: None,
                display_name: None,
                roles: None,
                organization_id: None,
                bucket: None,
                avatar_url: None,
                is_anonymous: true,
            })
        }
        Some("") => {
            info!("get_current_user: empty authorization token - returning anonymous user");
            Json(CurrentUserResponse {
                id: None,
                username: None,
                email: None,
                first_name: None,
                last_name: None,
                display_name: None,
                roles: None,
                organization_id: None,
                bucket: None,
                avatar_url: None,
                is_anonymous: true,
            })
        }
        Some(session_token) => {
            info!("get_current_user: looking up session token (len={}, prefix={}...)",
                  session_token.len(),
                  &session_token[..std::cmp::min(20, session_token.len())]);

            let cache = SESSION_CACHE.read().await;
            // Clone the entry so the read guard's borrow ends before any write
            // lock is taken for TTL eviction (E0505: cannot drop while borrowed).
            if let Some(user_data) = cache.get(session_token).cloned() {
                if session_expired(&user_data) {
                    drop(cache);
                    info!("get_current_user: session expired (TTL exceeded) - removing {}", user_data.email);
                    let mut cache = SESSION_CACHE.write().await;
                    cache.remove(session_token);
                    remove_persisted_session(session_token);
                    return Json(CurrentUserResponse {
                        id: None,
                        username: None,
                        email: None,
                        first_name: None,
                        last_name: None,
                        display_name: None,
                        roles: None,
                        organization_id: None,
                        bucket: None,
                        avatar_url: None,
                        is_anonymous: true,
                    });
                }
                info!("get_current_user: found cached session for user: {}", user_data.email);
                return Json(CurrentUserResponse {
                    id: Some(user_data.user_id),
                    username: Some(user_data.username),
                    email: Some(user_data.email),
                    first_name: user_data.first_name,
                    last_name: user_data.last_name,
                    display_name: user_data.display_name,
                    roles: Some(user_data.roles),
                    organization_id: user_data.organization_id,
                    bucket: user_data.bucket,
                    avatar_url: None,
                    is_anonymous: false,
                });
            }
            drop(cache);

            // Cache miss: rehydrate from the persisted `login_sessions` table so
            // sessions survive server restarts instead of silently logging users out.
            if let Some(user_data) = session_from_persisted(session_token) {
                if session_expired(&user_data) {
                    info!("get_current_user: persisted session expired - removing {}", user_data.email);
                    remove_persisted_session(session_token);
                    return Json(CurrentUserResponse {
                        id: None,
                        username: None,
                        email: None,
                        first_name: None,
                        last_name: None,
                        display_name: None,
                        roles: None,
                        organization_id: None,
                        bucket: None,
                        avatar_url: None,
                        is_anonymous: true,
                    });
                }
                info!("get_current_user: rehydrated persisted session for user: {}", user_data.email);
                {
                    let mut cache = SESSION_CACHE.write().await;
                    cache.insert(session_token.to_string(), user_data.clone());
                }
                Json(CurrentUserResponse {
                    id: Some(user_data.user_id.clone()),
                    username: Some(user_data.username.clone()),
                    email: Some(user_data.email.clone()),
                    first_name: user_data.first_name.clone(),
                    last_name: user_data.last_name.clone(),
                    display_name: user_data.display_name.clone(),
                    roles: Some(user_data.roles.clone()),
                    organization_id: user_data.organization_id.clone(),
                    bucket: user_data.bucket.clone(),
                    avatar_url: None,
                    is_anonymous: false,
                })
            } else if let Some(user_data) = resolve_cloud_bearer(session_token) {
                // #1364 — not a suite session token: it may still be a cloud
                // management JWT (SaaS surface). Resolve it through the wired
                // external verifier so signed-in cloud users see their identity
                // in the suite sidebar instead of a stale "Sign in" affordance.
                info!("get_current_user: resolved external (cloud JWT) bearer");
                Json(CurrentUserResponse {
                    id: Some(user_data.user_id),
                    username: Some(user_data.username),
                    email: Some(user_data.email),
                    first_name: user_data.first_name,
                    last_name: user_data.last_name,
                    display_name: user_data.display_name,
                    roles: Some(user_data.roles),
                    organization_id: user_data.organization_id,
                    bucket: user_data.bucket,
                    avatar_url: None,
                    is_anonymous: false,
                })
            } else {
                info!("get_current_user: session not found in cache or persisted store - returning anonymous user");
                Json(CurrentUserResponse {
                    id: None,
                    username: None,
                    email: None,
                    first_name: None,
                    last_name: None,
                    display_name: None,
                    roles: None,
                    organization_id: None,
                    bucket: None,
                    avatar_url: None,
                    is_anonymous: true,
                })
            }
        }
    }
}
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let token_url = format!("{}/oauth/v2/token", auth_service.api_url());

    let http_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| {
            log::error!("Failed to create HTTP client: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Internal server error".to_string(),
                    details: None,
                }),
            )
        })?;

    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", &req.refresh_token),
        ("scope", "openid profile email offline_access"),
    ];

    let response = http_client
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            log::error!("Failed to refresh token: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Token refresh failed".to_string(),
                    details: None,
                }),
            )
        })?;

    if !response.status().is_success() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid or expired refresh token".to_string(),
                details: None,
            }),
        ));
    }

    let token_data: serde_json::Value = response.json().await.map_err(|e| {
        log::error!("Failed to parse token response: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Invalid response from authentication server".to_string(),
                details: None,
            }),
        )
    })?;

    let access_token = token_data
        .get("access_token")
        .and_then(|t| t.as_str())
        .map(String::from);

    let refresh_token = token_data
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .map(String::from);

    let expires_in = token_data.get("expires_in").and_then(|t| t.as_i64());

    Ok(Json(LoginResponse {
        success: true,
        user_id: None,
        session_id: None,
        access_token,
        refresh_token,
        expires_in,
        requires_2fa: false,
        session_token: None,
        redirect: None,
        message: Some("Token refreshed successfully".to_string()),
    }))
}
pub async fn verify_2fa(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<TwoFactorRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!(
        "2FA verification attempt for session: {}",
        req.session_token
    );

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "2FA verification not yet implemented".to_string(),
            details: Some("This feature will be available in a future update".to_string()),
        }),
    ))
}
