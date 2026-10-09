use super::*;

pub async fn check_bot_access(
    state: &Arc<botcore::shared::state::AppState>,
    bot_name: &str,
    user_id: Uuid,
) -> Result<(), String> {
    check_bot_access_typed(state, bot_name, user_id)
        .await
        .map_err(|denial| match denial {
            BotAccessDenial::NotFound => "Bot not found".to_string(),
            BotAccessDenial::AuthRequired => "Access denied".to_string(),
            BotAccessDenial::Internal(e) => e,
        })
}
/// Typed variant of [`check_bot_access`] used by the WebSocket gateway so the
/// handshake can answer 404 (unknown bot) / 401 (private bot, sign-in
/// required) instead of a blanket 403.
pub async fn check_bot_access_typed(
    state: &Arc<botcore::shared::state::AppState>,
    bot_name: &str,
    user_id: Uuid,
) -> Result<(), BotAccessDenial> {
    use botcore::shared::schema::bots::dsl as bots_dsl;
    use botcore::shared::schema::user_organizations::dsl as uo_dsl;

    let mut conn = state
        .conn
        .get()
        .map_err(|e| BotAccessDenial::Internal(format!("DB connection error: {}", e)))?;

    // #1289 — the URL/launcher identifies bots by SLUG ("helper-bot") while
    // older rows carry a display name with spaces ("Helper Bot"). Accept
    // either: name first, then slug.
    let bot_record = bots_dsl::bots
        .filter(
            bots_dsl::name
                .eq(bot_name)
                .or(bots_dsl::slug.eq(bot_name)),
        )
        .select((bots_dsl::is_public, bots_dsl::org_id))
        .first::<(bool, Uuid)>(&mut *conn)
        .optional()
        .map_err(|e| BotAccessDenial::Internal(format!("DB query error: {}", e)))?;

    let (is_public, org_id) = match bot_record {
        Some(record) => record,
        None => return Err(BotAccessDenial::NotFound),
    };

    if is_public {
        return Ok(());
    }

    {
        let is_member = uo_dsl::user_organizations
            .filter(uo_dsl::user_id.eq(user_id))
            .filter(uo_dsl::org_id.eq(org_id))
            .count()
            .get_result::<i64>(&mut *conn)
            .map_err(|e| BotAccessDenial::Internal(format!("DB query error: {}", e)))? > 0;

        if is_member {
            return Ok(());
        }
    }

    Err(BotAccessDenial::AuthRequired)
}
pub async fn check_access_handler(
    axum::extract::State(state): axum::extract::State<Arc<botcore::shared::state::AppState>>,
    axum::extract::Path(bot_name): axum::extract::Path<String>,
    req: axum::extract::Request,
) -> impl axum::response::IntoResponse {        let is_public = {
        use botcore::shared::schema::bots::dsl as bots_dsl;
        if let Ok(mut conn) = state.conn.get() {
            // #1289 — accept slug as well as name (launcher/URL use slugs).
            bots_dsl::bots
                .filter(
                    bots_dsl::name
                        .eq(&bot_name)
                        .or(bots_dsl::slug.eq(&bot_name)),
                )
                .select(bots_dsl::is_public)
                .first::<bool>(&mut *conn)
                .unwrap_or(false)
        } else {
            false
        }
    };

    if is_public || bot_name == "default" {
        return axum::http::StatusCode::OK.into_response();
    }

    let user = req.extensions().get::<crate::security::auth_api::types::AuthenticatedUser>();
    let user_id = match user {
        Some(u) if u.is_authenticated() => u.user_id,
        _ => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
    };

    match check_bot_access(&state, &bot_name, user_id).await {
        Ok(_) => axum::http::StatusCode::OK.into_response(),
        Err(_) => axum::http::StatusCode::FORBIDDEN.into_response(),
    }
}
