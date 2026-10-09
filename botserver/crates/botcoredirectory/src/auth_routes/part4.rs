use super::*;

pub fn configure() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/me", get(get_current_user))
        .route("/refresh", post(refresh_token))
        .route("/2fa/verify", post(verify_2fa))
        .route("/2fa/resend", post(resend_2fa))
        .route("/bootstrap", post(bootstrap_admin))
        // #1546 — Profile/Security link to the directory: the Settings UI
        // reads the configured issuer so it can open the Zitadel console.
        .route("/directory-config", get(get_directory_config))

}
/// `GET /api/auth/directory-config` — public, non-sensitive: returns only the
/// configured directory API base so the suite UI can deep-link the Zitadel
/// console. No credentials, tokens or internal-only addresses are returned.
pub(super) async fn get_directory_config(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let api_url = state
        .auth_service
        .as_ref()
        .map(|svc| svc.try_lock().map(|svc| svc.api_url()).unwrap_or_default())
        .unwrap_or_default();
    Json(serde_json::json!({
        "issuer_url": api_url,
        "provider": "zitadel",
    }))
}
