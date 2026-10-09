use super::*;

/// Resolves the authenticated user from the bearer session (fix #830).
///
/// The token is looked up in the persisted `login_sessions` table (the same
/// store the auth middleware rehydrates from), and the stored `user_id` is
/// mapped to a stable UUID the same way `resolve_user_role` does — never the
/// first row of the `users` table.
/// Resolves the SaaS JWT secret used to sign cloud-login tokens — the same
/// chain the minter applies (`directory_setup::resolve_saas_jwt_secret` is
/// `pub(crate)` to the botserver crate, so replicate it here): persisted
/// `directory_config.json` → env `SAAS_JWT_SECRET` → env `JWT_SECRET` →
/// development default.
pub(super) fn saas_jwt_secret() -> String {
    let stack = std::env::var("BOTSERVER_STACK_PATH")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| std::env::var("GBO_STACK_PATH").ok().filter(|p| !p.trim().is_empty()))
        .unwrap_or_else(|| "/opt/gbo".to_string());
    let config_path = format!("{stack}/conf/system/directory_config.json");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(secret) = json
                .get("saas_jwt_secret")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                return secret.to_string();
            }
        }
    }
    std::env::var("SAAS_JWT_SECRET")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("JWT_SECRET").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_else(|| "dev-secret-key-change-in-production-minimum-32-chars".to_string())
}
/// #1457 — verifies a stateless SaaS cloud JWT and returns the caller's
/// stable user id. Cloud login mints claim-light tokens (`sub`/`email`/
/// `org_id`/`branch_id`, issue #736); the `sub` claim maps to a UUID the
/// same way `SaasJwtAuthProvider` and `resolve_user_role` do.
pub(super) fn user_id_from_saas_jwt(token: &str) -> Option<uuid::Uuid> {
    if token.split('.').count() != 3 {
        return None;
    }
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.required_spec_claims = HashSet::from(["exp".to_string()]);
    let data = decode::<serde_json::Value>(
        token,
        &DecodingKey::from_secret(saas_jwt_secret().as_bytes()),
        &validation,
    )
    .ok()?;
    let sub = data
        .claims
        .get("sub")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if sub.is_empty() {
        return None;
    }
    Some(uuid::Uuid::parse_str(sub).unwrap_or_else(|_| {
        uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, format!("zitadel:{sub}").as_bytes())
    }))
}
/// Fallback user resolution for fragment handlers without a request context.
/// Returns the first user row — these are read-only UI fragments (storage,
/// 2FA status) where the session may be anonymous.
pub fn first_user(conn: &mut diesel::PgConnection) -> Option<uuid::Uuid> {
    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct IdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
    }
    diesel::sql_query("SELECT id FROM users ORDER BY created_at LIMIT 1")
        .get_result::<IdRow>(conn)
        .ok()
        .map(|r| r.id)
}
pub fn resolve_user_id(
    state: &Arc<AppState>,
    headers: &axum::http::HeaderMap,
) -> Result<uuid::Uuid, (StatusCode, Html<String>)> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|a| a.strip_prefix("Bearer "))
        .map(|s| s.to_string());
    let Some(token) = token else {
        return Err((StatusCode::UNAUTHORIZED, Html("Missing bearer token".to_string())));
    };

    let mut conn = get_conn(state)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())))?;

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct SessionRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        user_data: String,
    }

    let row: Option<SessionRow> = diesel::sql_query(
        // user_data is jsonb: cast to text so the Text decode always succeeds
        // (a raw jsonb result would silently fail and reject valid sessions).
        "SELECT user_data::text AS user_data FROM login_sessions WHERE token = $1 LIMIT 1",
    )
    .bind::<diesel::sql_types::Text, _>(&token)
    .get_result(&mut conn)
    .ok();

    let Some(row) = row else {
        // #1457 — cloud-login tokens are stateless SaaS JWTs (HS256, signed
        // with the persisted `saas_jwt_secret`) and never have a
        // `login_sessions` row, so every settings endpoint rejected them with
        // 401 while the same token authenticated fine against /api/crm/*.
        // Validate the JWT signature directly before rejecting.
        if let Some(user_id) = user_id_from_saas_jwt(&token) {
            return Ok(user_id);
        }
        return Err((StatusCode::UNAUTHORIZED, Html("No valid session for token".to_string())));
    };

    let parsed: serde_json::Value = serde_json::from_str(&row.user_data).unwrap_or_default();
    let uid = parsed.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    if uid.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, Html("Invalid session user".to_string())));
    }

    match uuid::Uuid::parse_str(uid) {
        Ok(u) => Ok(u),
        Err(_) => Ok(uuid::Uuid::new_v5(
            &uuid::Uuid::NAMESPACE_DNS,
            format!("zitadel:{uid}").as_bytes(),
        )),
    }
}
/// Whether the resolved user holds an admin group (RBAC).
pub(super) fn is_admin_user(state: &Arc<AppState>, user_id: uuid::Uuid) -> bool {
    let Ok(mut conn) = state.conn.get() else {
        return false;
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct GroupName {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let names: Vec<GroupName> = diesel::sql_query(
        "SELECT g.name FROM rbac_groups g \
         JOIN rbac_user_groups ug ON ug.group_id = g.id \
         WHERE ug.user_id = $1 AND g.is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load(&mut conn)
    .unwrap_or_default();
    names.iter().any(|g| g.name.to_lowercase().contains("admin"))
}
pub fn configure_settings_api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/user/profile", get(settings_profile::user_profile_get).put(settings_profile::user_profile_put))
        .route("/api/user/desktops", get(settings_profile::user_desktops_get).put(settings_profile::user_desktops_put))
        .route("/api/user/password", post(settings_profile::user_password))
        .route("/api/user/api-keys", get(settings_credentials::api_keys_list).post(settings_credentials::api_keys_create))
        .route("/api/user/api-keys/:key_id", axum::routing::delete(settings_credentials::api_keys_delete))
        .route("/api/user/api-keys/:key_id/rotate", post(settings_credentials::api_keys_rotate))
        .route("/api/user/webhooks", get(settings_webhooks::webhooks_list).post(settings_webhooks::webhooks_create))
        .route("/api/user/webhooks/:webhook_id", axum::routing::delete(settings_webhooks::webhooks_delete))
        .route("/api/user/webhooks/:webhook_id/test", post(settings_webhooks::webhooks_test))
        .route("/api/user/webhooks/:webhook_id/deliveries", get(settings_webhooks::webhook_deliveries_list))
        .route("/api/user/billing/plan", get(settings_billing::billing_plan))
        .route("/api/user/billing/invoices", get(settings_billing::billing_invoices))
        .route("/api/user/billing/payment-methods", get(settings_billing::billing_payment_methods))
        .route("/api/user/data/export", post(settings_billing::data_export))
        .route("/api/user/notifications/preferences", get(notif_prefs_get).put(notif_prefs_put))
        .route("/api/oauth/:provider/connect", post(oauth_connect_handler))
        .route("/api/groups/create", post(groups_create))
        .route("/api/users/create", post(users_create))
}
pub fn get_conn(
    state: &Arc<AppState>,
) -> Option<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::PgConnection>>> {
    state.conn.get().ok()
}
