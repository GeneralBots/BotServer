use super::*;

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Login attempt for: {}", req.email);

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

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

    let admin_token = resolve_admin_token(&http_client, &*auth_service).await;

    // If we have an admin token, try Zitadel sessions API first
    if let Some(ref admin_token) = admin_token {
        // Zitadel matches `loginName` against a user's login names, which are
        // the username and its org-domain forms. When a demo/user account was
        // created with a bare username (e.g. `sample`), sending the full email
        // (`sample@example.com`) fails. Try the full email first, then fall
        // back to the username prefix.
        let username = req.email.split('@').next().unwrap_or(&req.email).to_string();
        let login_names = [req.email.clone(), username];

        let mut session_response: Option<reqwest::Response> = None;
        let mut session_error: Option<String> = None;
        // Set when the address was resolved to a user id before the password
        // check (the email fallback below), so the id does not have to be
        // recovered from the session afterwards.
        let mut resolved_user_id: Option<String> = None;

        for login_name in &login_names {
            let session_url = format!("{}/v2/sessions", auth_service.api_url());
            let session_body = serde_json::json!({
                "checks": {
                    "user": {
                        "loginName": login_name
                    },
                    "password": {
                        "password": req.password
                    }
                }
            });

            match http_client
                .post(&session_url)
                .bearer_auth(admin_token)
                .json(&session_body)
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => {
                    session_response = Some(resp);
                    break;
                }
                Ok(resp) => {
                    let status = resp.status();
                    let err = resp.text().await.unwrap_or_default();
                    log::warn!(
                        "Zitadel sessions API returned {} for loginName '{}': {}",
                        status,
                        login_name,
                        err
                    );
                    session_error = Some(format!("{status} {err}"));
                }
                Err(e) => {
                    log::warn!(
                        "Zitadel sessions API request failed for loginName '{}': {}",
                        login_name,
                        e
                    );
                    session_error = Some(e.to_string());
                }
            }
        }

        // Last resort before failing: the form collects an email, but Zitadel
        // only accepts login names — an account created with a bare username
        // (e.g. `contato`) is unreachable by its address. Resolve the email to
        // a user id and retry the password check against that id.
        if session_response.is_none() {
            if let Some(user_id) = resolve_user_id_by_email(
                &http_client,
                &*auth_service,
                admin_token.as_str(),
                &req.email,
            )
            .await
            {
                let session_url = format!("{}/v2/sessions", auth_service.api_url());
                let session_body = serde_json::json!({
                    "checks": {
                        "user": {
                            "userId": user_id
                        },
                        "password": {
                            "password": req.password
                        }
                    }
                });

                match http_client
                    .post(&session_url)
                    .bearer_auth(admin_token)
                    .json(&session_body)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        info!("Session created for '{}' via resolved user id", req.email);
                        resolved_user_id = Some(user_id.clone());
                        session_response = Some(resp);
                    }
                    Ok(resp) => {
                        let status = resp.status();
                        let err = resp.text().await.unwrap_or_default();
                        log::warn!(
                            "Zitadel sessions API returned {} for the user resolved from '{}': {}",
                            status,
                            req.email,
                            err
                        );
                        session_error = Some(format!("{status} {err}"));
                    }
                    Err(e) => {
                        log::warn!(
                            "Zitadel sessions API request failed for the user resolved from '{}': {}",
                            req.email,
                            e
                        );
                        session_error = Some(e.to_string());
                    }
                }
            }
        }

        if let Some(resp) = session_response {
            let session_data: serde_json::Value = resp.json().await.map_err(|e| {
                log::error!("Failed to parse session response: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: "Invalid response from authentication server".to_string(),
                        details: None,
                    }),
                )
            })?;

            let session_id = session_data
                .get("sessionId")
                .and_then(|s| s.as_str())
                .map(String::from);

            let session_token = session_data
                .get("sessionToken")
                .and_then(|s| s.as_str())
                .map(String::from);

            // Zitadel returns only a sessionId/sessionToken when the session
            // is created; `factors` (and therefore the user id) appear once the
            // session is resolved. Requiring them here rejected every valid
            // login with "No user ID in session response". Use, in order: the
            // id from the email resolution, factors if the API included them,
            // then a follow-up session read (the id is `factors.user.id`, not
            // `userId`).
            let user_id_str = match resolved_user_id.take() {
                Some(id) => id,
                None => {
                    let from_factors = session_data
                        .get("factors")
                        .and_then(|f| f.get("user"))
                        .and_then(|u| u.get("id").or_else(|| u.get("userId")))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    match from_factors {
                        Some(id) => id,
                        None => {
                            let resolved = match session_id.as_deref() {
                                Some(id) => {
                                    resolve_session_user_id(
                                        &http_client,
                                        &auth_service.api_url(),
                                        admin_token,
                                        id,
                                    )
                                    .await
                                }
                                None => None,
                            };

                            resolved.ok_or_else(|| {
                                log::error!(
                                    "Could not resolve the user id for a valid session ({}): {}",
                                    req.email,
                                    session_data
                                );
                                (
                                    StatusCode::UNAUTHORIZED,
                                    Json(ErrorResponse {
                                        error: "Invalid email or password".to_string(),
                                        details: None,
                                    }),
                                )
                            })?
                        }
                    }
                }
            };

            let api_token = format!("gb_{}_{}", uuid::Uuid::new_v4(), chrono::Utc::now().timestamp());

            let session_user = SessionUserData {
                user_id: user_id_str.clone(),
                email: req.email.clone(),
                username: req.email.split('@').next().unwrap_or("user").to_string(),
                first_name: None,
                last_name: None,
                display_name: Some(req.email.split('@').next().unwrap_or("User").to_string()),
                organization_id: None,
                roles: vec!["admin".to_string()],
                bucket: None,
                created_at: chrono::Utc::now().timestamp(),
            };

            {
                let mut cache = SESSION_CACHE.write().await;
                cache.insert(api_token.clone(), session_user.clone());
                info!("Session cached for user: {}", req.email);
                persist_session(&api_token, &session_user);
            }

            info!("Login successful for: {} (user_id: {})", req.email, user_id_str);

            return Ok(Json(LoginResponse {
                success: true,
                user_id: Some(user_id_str),
                session_id: session_id.clone(),
                access_token: Some(api_token),
                refresh_token: None,
                expires_in: Some(3600),
                requires_2fa: false,
                session_token,
                redirect: Some("/".to_string()),
                message: Some("Login successful".to_string()),
            }));
        } else {
            log::warn!(
                "Zitadel sessions API failed for {}: {} — falling back to local credential check",
                req.email,
                session_error.unwrap_or_default()
            );
        }
    } else {
        log::info!("No admin token available, falling back to local credential check");
    }

    // Zitadel sessions API failed — no fallback; Zitadel is the only auth provider
    log::error!("Zitadel authentication failed for: {}", req.email);
    Err((
        StatusCode::UNAUTHORIZED,
        Json(ErrorResponse {
            error: "Invalid email or password".to_string(),
            details: None,
        }),
    ))
}
