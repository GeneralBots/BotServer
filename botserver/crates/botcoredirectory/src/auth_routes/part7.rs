use super::*;

pub async fn resend_2fa(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<serde_json::Value>,
) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(ErrorResponse {
            error: "2FA resend not yet implemented".to_string(),
            details: Some("This feature will be available in a future update".to_string()),
        }),
    )
}
pub async fn bootstrap_admin(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BootstrapAdminRequest>,
) -> Result<Json<BootstrapResponse>, (StatusCode, Json<ErrorResponse>)> {
    info!("Bootstrap admin request received");

    let expected_secret = std::env::var(BOOTSTRAP_SECRET_ENV).unwrap_or_default();

    if expected_secret.is_empty() {
        warn!("Bootstrap endpoint called but GB_BOOTSTRAP_SECRET not set");
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error: "Bootstrap not enabled".to_string(),
                details: Some("Set GB_BOOTSTRAP_SECRET environment variable to enable bootstrap".to_string()),
            }),
        ));
    }

    if req.bootstrap_secret != expected_secret {
        warn!("Bootstrap attempt with invalid secret");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "Invalid bootstrap secret".to_string(),
                details: None,
            }),
        ));
    }

    let auth_service = state.auth_service.as_ref().ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: "No auth service".to_string(), details: None })))?.lock().await;

    let existing_data = auth_service.list_users(1, 0).await.unwrap_or(serde_json::Value::Null);
    let existing_users = existing_data.as_array();
    if let Some(users_arr) = existing_users {
        if !users_arr.is_empty() {
            let has_admin = users_arr.iter().any(|u| {
                u.get("roles")
                    .and_then(|r| r.as_array())
                    .map(|roles| {
                        roles.iter().any(|r| {
                            r.as_str()
                                .map(|s| s.to_lowercase().contains("admin"))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            });

            if has_admin {
                return Err((
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "Admin user already exists".to_string(),
                        details: Some("Bootstrap can only be used for initial setup".to_string()),
                    }),
                ));
            }
        }
    }

    let new_user_id = match auth_service
        .create_user(&req.email, &req.first_name, &req.last_name, Some(&req.username))
        .await
    {
        Ok(id) => {
            info!("Bootstrap admin user created: {}", id);
            id
        }
        Err(e) => {
            log::error!("Failed to create bootstrap admin: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to create admin user".to_string(),
                    details: Some(e.to_string()),
                }),
            ));
        }
    };

    if let Err(e) = auth_service.set_user_password(&new_user_id, &req.password).await {
        log::error!("Failed to set admin password: {}", e);
    }

    let org_name = req.organization_name.unwrap_or_else(|| "Default Organization".to_string());
    let new_org_id = match auth_service.create_organization(&org_name).await {
        Ok(id) => {
            info!("Bootstrap organization created: {}", id);
            Some(id)
        }
        Err(e) => {
            warn!("Failed to create organization (may already exist): {}", e);
            None
        }
    };

    if let Some(ref oid) = new_org_id {
        let admin_roles = vec![
            "admin".to_string(),
            "org_owner".to_string(),
            "user_manager".to_string(),
        ];
        if let Err(e) = auth_service.add_org_member(oid, &new_user_id, admin_roles).await {
            log::error!("Failed to add admin to organization: {}", e);
        } else {
            info!("Admin user added to organization with admin roles");
        }

        let system_groups = vec![
            ("everyone", "Everyone", vec!["basic:access", "kb:read:public"]),
            ("admins", "Administrators", vec!["org:manage", "org:members", "org:settings", "org:billing", "bot:*", "kb:*", "app:*", "analytics:*"]),
            ("managers", "Managers", vec!["org:members:view", "bot:create", "bot:edit", "bot:delete", "kb:read", "kb:write", "analytics:view"]),
            ("developers", "Developers", vec!["bot:create", "bot:edit", "kb:write", "app:create"]),
            ("human_resources", "Human Resources", vec!["people:manage", "kb:read", "org:members:view"]),
            ("finance", "Finance", vec!["billing:view", "analytics:view", "kb:read"]),
            ("marketing", "Marketing", vec!["campaigns:manage", "social:post", "analytics:view"]),
            ("support", "Support Team", vec!["bot:view", "kb:read", "tickets:manage"]),
            ("content_managers", "Content Managers", vec!["kb:write", "kb:admin", "docs:manage"]),
            ("sales", "Sales", vec!["crm:view", "crm:manage", "analytics:view"]),
            ("viewers", "Viewers (Read-Only)", vec!["bot:view", "kb:read", "app:view"]),
            ("integration_services", "Integration Services", vec!["webhooks:manage", "api:access", "sources:connect"]),
        ];

        for (group_name, display_name, perms) in &system_groups {
            let metadata_key = format!("group_{}", group_name);
            let metadata_value = serde_json::json!({
                "name": display_name,
                "description": format!("Auto-provisioned {} group", display_name),
                "permissions": perms,
                "system": true,
                "organization_id": oid
            }).to_string();

            let body = serde_json::json!({
                "key": metadata_key,
                "value": metadata_value
            });

            if let Err(e) = auth_service.http_post(
                format!("{}/metadata/organization", auth_service.api_url()),
                body,
            ).await {
                log::warn!("Failed to provision group {}: {}", group_name, e);
            } else {
                info!("Provisioned system group: {}", group_name);
            }
        }

        info!("Auto-provisioned 12 system groups for organization {}", oid);
    }

    info!(
        "Bootstrap complete: admin user {} created successfully",
        req.username
    );

    Ok(Json(BootstrapResponse {
        success: true,
        message: format!(
            "Admin user '{}' created successfully. You can now login with your credentials.",
            req.username
        ),
        user_id: Some(new_user_id),
        organization_id: new_org_id,
    }))
}
/// Admin token for the Zitadel API. Sources, in order: the PAT file written
/// during provisioning, the `service_token` stored in Vault — Vault is the
/// canonical secret store, and a deployment that never wrote the PAT file (or
/// whose OAuth client credentials are unset) otherwise had no admin token at
/// all, which rejected every interactive login before Zitadel was ever asked —
/// and finally an OAuth client-credentials token.
pub(super) async fn resolve_admin_token(
    http_client: &reqwest::Client,
    auth_service: &dyn botlib::traits::AuthServiceTrait,
) -> Option<String> {
    let stack = get_stack_path();
    let pat_path = std::path::PathBuf::from(format!("{}/conf/directory/admin-pat.txt", stack));
    if let Ok(contents) = std::fs::read_to_string(&pat_path) {
        let token = contents.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }

    if let Ok(manager) = botcoresecrets::manager::SecretsManager::get() {
        match manager.get_secret("gbo/directory").await {
            Ok(secret) => {
                if let Some(token) = secret
                    .get("service_token")
                    .map(|t| t.trim())
                    .filter(|t| !t.is_empty())
                {
                    info!("Using the directory service token from Vault as the admin token");
                    return Some(token.to_string());
                }
            }
            Err(e) => log::warn!("Directory secret unavailable in Vault: {}", e),
        }
    }

    info!("Admin PAT token not found, using OAuth client credentials flow");
    match get_oauth_token(http_client, auth_service).await {
        Ok(token) => Some(token),
        Err(e) => {
            log::warn!("Failed to get OAuth token (will try local auth): {}", e);
            None
        }
    }
}
/// Reads the user id back from an existing session. The session-creation
/// response carries only `sessionId`/`sessionToken`; the user factor (and its
/// id, exposed as `factors.user.id`) is available on the session resource.
pub(super) async fn resolve_session_user_id(
    http_client: &reqwest::Client,
    api_url: &str,
    admin_token: &str,
    session_id: &str,
) -> Option<String> {
    let url = format!("{api_url}/v2/sessions/{session_id}");
    let response = http_client
        .get(&url)
        .bearer_auth(admin_token)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        log::warn!(
            "Session read failed for {}: {} {}",
            session_id,
            response.status(),
            response.text().await.unwrap_or_default()
        );
        return None;
    }

    let data: serde_json::Value = response.json().await.ok()?;
    data.get("session")
        .and_then(|s| s.get("factors"))
        .and_then(|f| f.get("user"))
        .and_then(|u| u.get("id").or_else(|| u.get("userId")))
        .and_then(|v| v.as_str())
        .map(String::from)
}
/// Resolves a login form's email address to the Zitadel user id. Zitadel
/// matches `loginName` against the account's login names, which for accounts
/// created with a bare username do not include the email — resolving the id
/// keeps "sign in with your email" working for every account.
pub(super) async fn resolve_user_id_by_email(
    http_client: &reqwest::Client,
    auth_service: &dyn botlib::traits::AuthServiceTrait,
    admin_token: &str,
    email: &str,
) -> Option<String> {
    let url = format!("{}/management/v1/users/_search", auth_service.api_url());
    let body = serde_json::json!({
        "queries": [{
            "emailQuery": { "emailAddress": email, "method": "TEXT_QUERY_METHOD_EQUALS" }
        }]
    });

    let response = http_client
        .post(&url)
        .bearer_auth(admin_token)
        .json(&body)
        .send()
        .await
        .ok()?;

    if !response.status().is_success() {
        log::warn!(
            "User lookup by email failed: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
        return None;
    }

    let data: serde_json::Value = response.json().await.ok()?;
    data.get("result")
        .and_then(|r| r.as_array())
        .and_then(|users| users.first())
        .and_then(|user| user.get("id"))
        .and_then(|id| id.as_str())
        .map(String::from)
}
