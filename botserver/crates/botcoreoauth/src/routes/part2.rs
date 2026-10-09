use super::*;

pub(super) async fn create_or_get_oauth_user(
    state: &OAuthState_,
    user_info: &OAuthUserInfo,
) -> anyhow::Result<Uuid> {
    let conn = state.conn.clone();
    let provider_id = user_info.provider_id.clone();
    let provider = user_info.provider.to_string().to_lowercase();
    let user_email = user_info.email.clone();
    let display_name = user_info.name.clone().unwrap_or_else(|| "OAuth User".to_string());

    tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().map_err(|e| anyhow!("DB connection error: {}", e))?;

        use diesel::prelude::*;
        use crate::schema::users;

        let existing_user: Option<Uuid> = if let Some(ref email_addr) = user_email {
            users::dsl::users
                .filter(users::dsl::email.eq(email_addr))
                .select(users::dsl::id)
                .first(&mut db_conn)
                .optional()
                .map_err(|e| anyhow!("DB error: {}", e))?
        } else {
            let oauth_username = format!("{}_{}", provider, provider_id);
            users::dsl::users
                .filter(users::dsl::username.eq(&oauth_username))
                .select(users::dsl::id)
                .first(&mut db_conn)
                .optional()
                .map_err(|e| anyhow!("DB error: {}", e))?
        };

        if let Some(user_id) = existing_user {
            return Ok(user_id);
        }

        let new_user_id = Uuid::new_v4();

        let sanitized_name: String = display_name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .take(20)
            .collect();

        let oauth_username = if sanitized_name.is_empty() {
            format!("{}_{}", provider, &provider_id[..8.min(provider_id.len())])
        } else {
            format!("{}_{}", sanitized_name, &provider_id[..6.min(provider_id.len())])
        };

        let user_email_value = user_email.unwrap_or_else(|| format!("{}@oauth.local", oauth_username));

        diesel::insert_into(users::dsl::users)
            .values((
                users::dsl::id.eq(new_user_id),
                users::dsl::username.eq(&oauth_username),
                users::dsl::email.eq(&user_email_value),
                users::dsl::password_hash.eq("OAUTH_USER_NO_PASSWORD"),
                users::dsl::is_active.eq(true),
                users::dsl::is_admin.eq(false),
                users::dsl::created_at.eq(diesel::dsl::now),
                users::dsl::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut db_conn)
            .map_err(|e| anyhow!("Failed to create user: {}", e))?;

        debug!("Created OAuth user: {} ({}) for provider {}", oauth_username, user_email_value, provider);

        Ok(new_user_id)
    })
    .await
    .map_err(|e| anyhow!("Task error: {}", e))?
}
pub(super) async fn create_user_session(state: &OAuthState_, user_id: Uuid) -> anyhow::Result<String> {
    let conn = state.conn.clone();
    let (bot_id, branch_id) = tokio::task::spawn_blocking(move || {
        let mut db_conn = conn.get().ok()?;
        use diesel::prelude::*;
        use crate::schema::bots;
        bots::dsl::bots
            .filter(bots::dsl::is_active.eq(true))
            .select((bots::dsl::id, bots::dsl::branch_id))
            .first::<(Uuid, Uuid)>(&mut db_conn)
            .optional()
            .ok()?
    })
    .await
    .ok()
    .flatten()
    .unwrap_or((Uuid::nil(), Uuid::nil()));

    let session_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    let conn = state.conn.clone();
    tokio::task::spawn_blocking(move || {
        use diesel::prelude::*;
        use crate::schema::user_sessions;
        diesel::insert_into(user_sessions::dsl::user_sessions)
            .values((
                user_sessions::dsl::id.eq(session_id),
                user_sessions::dsl::branch_id.eq(branch_id),
                user_sessions::dsl::bot_id.eq(bot_id),
                user_sessions::dsl::session_id.eq(session_id.to_string()),
                user_sessions::dsl::user_id.eq(user_id.to_string()),
                user_sessions::dsl::created_at.eq(now),
                user_sessions::dsl::updated_at.eq(now),
            ))
            .execute(&mut conn.get().map_err(|e| anyhow!("DB error: {}", e))?)
            .map_err(|e| anyhow!("Failed to create session: {}", e))?;
        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|e| anyhow!("Task error: {}", e))??;

    Ok(session_id.to_string())
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
