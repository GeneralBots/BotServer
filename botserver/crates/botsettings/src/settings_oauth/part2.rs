use super::*;

/// GET /api/oauth/accounts
///
/// Lists the OAuth accounts linked to the authenticated user (HTML fragment
/// for the Settings page).
pub async fn oauth_accounts_list(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html(String::new())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct LinkRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        provider: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        email: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        display_name: Option<String>,
    }
    let rows: Vec<LinkRow> = diesel::sql_query(
        "SELECT provider, email, display_name \
         FROM oauth_account_links WHERE user_id = $1 ORDER BY provider",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .load::<LinkRow>(&mut conn)
    .unwrap_or_default();

    if rows.is_empty() {
        return (
            StatusCode::OK,
            Html(r#"<div class="empty-state"><p>No accounts linked yet.</p></div>"#.to_string()),
        );
    }

    let mut html = String::from(r#"<div class="oauth-accounts-list">"#);
    for row in rows {
        let display = row
            .display_name
            .clone()
            .unwrap_or_else(|| row.provider.clone());
        let email = row.email.unwrap_or_default();
        html.push_str(&format!(
            r#"<div class="oauth-account-item">
                <span class="oauth-account-name">{display}</span>
                <span class="oauth-account-email">{email}</span>
                <span class="oauth-account-status">Linked</span>
                <button class="btn-secondary btn-sm" hx-post="/api/oauth/{}/unlink" hx-target="closest .oauth-account-item" hx-swap="outerHTML">Unlink</button>
            </div>"#,
            row.provider
        ));
    }
    html.push_str("</div>");
    (StatusCode::OK, Html(html))
}
/// POST /api/oauth/{provider}/unlink
///
/// Disconnects the provider: revokes the Vault token (best-effort),
/// deletes the link row, and audit-logs the action.
pub async fn oauth_unlink(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(provider_name): Path<String>,
) -> impl IntoResponse {
    let provider_name_lower = provider_name.to_lowercase();
    let Some(provider) = OAuthProvider::parse(&provider_name_lower) else {
        return (
            StatusCode::BAD_REQUEST,
            Html(format!(
                "Unsupported OAuth provider: {}",
                provider_name.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
            )),
        );
    };

    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string())),
    };
    let user_id = match resolve_user_id(&state, &headers) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Best-effort token revocation from Vault.
    let _ = delete_tokens_from_vault(&user_id, provider);

    let affected = diesel::sql_query(
        "DELETE FROM oauth_account_links WHERE user_id = $1 AND provider = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&provider_name_lower)
    .execute(&mut conn)
    .unwrap_or(0);

    record_audit_event(crate::audit_log::AuditEvent {
        state: &state,
        event_type: "oauth.unlink",
        actor_id: user_id,
        action: "oauth.unlink",
        target_type: Some("oauth_account_links"),
        target_id: None,
        success: affected > 0,
        details: Some(&format!("Unlinked {} account", provider)),
    });

    if affected == 0 {
        return (
            StatusCode::NOT_FOUND,
            Html(format!("No linked {} account found", provider)),
        );
    }
    (StatusCode::OK, Html(format!("{} account unlinked", provider)))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
