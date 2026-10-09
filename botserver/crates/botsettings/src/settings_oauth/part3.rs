use super::*;

/// Redirects back to the Settings page (accounts section) preserving the
/// result via query params.
pub(super) fn redirect_to_settings(query: &str) -> Response {
    let base = base_url();
    Redirect::temporary(&format!("{base}/settings?section=accounts&{query}"))
        .into_response()
}
/// Persists the OAuth access/refresh tokens in Vault under
/// `secret/gbo/oauth/{user_id}/{provider}`. Only non-sensitive metadata is
/// kept in the database.
pub(super) fn store_tokens_in_vault(
    user_id: &uuid::Uuid,
    provider: OAuthProvider,
    token: &botcoreoauth::OAuthTokenResponse,
) -> anyhow::Result<()> {
    let manager = botcoresecrets::SecretsManager::get_clone()?;
    if !manager.is_enabled() {
        return Ok(());
    }
    let path = format!(
        "gbo/oauth/{}/{}",
        user_id,
        provider.to_string().to_lowercase()
    );
    let mut data = HashMap::new();
    data.insert("access_token".to_string(), token.access_token.clone());
    data.insert("token_type".to_string(), token.token_type.clone());
    data.insert(
        "expires_in".to_string(),
        token.expires_in.unwrap_or(0).to_string(),
    );
    if let Some(refresh) = &token.refresh_token {
        data.insert("refresh_token".to_string(), refresh.clone());
    }
    if let Some(scope) = &token.scope {
        data.insert("scope".to_string(), scope.clone());
    }
    let manager = manager.clone();
    let path_clone = path.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(manager.put_secret(&path_clone, data))
        } else {
            Err(anyhow::anyhow!("failed to create runtime"))
        };
        let _ = tx.send(result);
    });
    match rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::anyhow!("Vault write channel closed: {e}")),
    }
}
/// Removes the stored tokens for a linked account (unlink flow).
pub(super) fn delete_tokens_from_vault(
    user_id: &uuid::Uuid,
    provider: OAuthProvider,
) -> anyhow::Result<()> {
    let manager = botcoresecrets::SecretsManager::get_clone()?;
    if !manager.is_enabled() {
        return Ok(());
    }
    let path = format!(
        "gbo/oauth/{}/{}",
        user_id,
        provider.to_string().to_lowercase()
    );
    let manager = manager.clone();
    let path_clone = path.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(manager.delete_secret(&path_clone))
        } else {
            Err(anyhow::anyhow!("failed to create runtime"))
        };
        let _ = tx.send(result);
    });
    match rx.recv() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(anyhow::anyhow!("Vault delete channel closed: {e}")),
    }
}
/// Upserts the account link row (one link per user/provider).
pub(super) fn upsert_account_link(
    state: &Arc<AppState>,
    user_id: uuid::Uuid,
    provider: OAuthProvider,
    user_info: &botcoreoauth::OAuthUserInfo,
) -> anyhow::Result<()> {
    let mut conn = get_conn(state)
        .ok_or_else(|| anyhow::anyhow!("database unavailable"))?;
    let provider_lower = provider.to_string().to_lowercase();
    diesel::sql_query(
        "INSERT INTO oauth_account_links \
         (id, user_id, provider, provider_user_id, email, display_name, avatar_url, linked_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW()) \
         ON CONFLICT (user_id, provider) \
         DO UPDATE SET provider_user_id = EXCLUDED.provider_user_id, \
                       email = EXCLUDED.email, display_name = EXCLUDED.display_name, \
                       avatar_url = EXCLUDED.avatar_url, updated_at = NOW()",
    )
    .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(user_id)
    .bind::<diesel::sql_types::Text, _>(&provider_lower)
    .bind::<diesel::sql_types::Text, _>(&user_info.provider_id)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(user_info.email.clone())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(user_info.name.clone())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(user_info.avatar_url.clone())
    .execute(&mut conn)
    .map_err(|e| anyhow::anyhow!("upsert oauth link: {e}"))?;
    Ok(())
}
