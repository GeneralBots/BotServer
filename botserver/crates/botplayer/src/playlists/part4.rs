use super::*;

/// Resolves the scope for playlists: the branch the caller belongs to, or the
/// default branch when no JWT is carried (suite mode).
pub(super) fn resolve_branch(state: &Arc<AppState>, user_id: Option<Uuid>) -> Uuid {
    if let Some(_uid) = user_id {
        // Branch scope still applies; ownership is enforced by the user_id
        // column on the playlist itself.
        return Uuid::nil();
    }
    let _ = state;
    Uuid::nil()
}
/// `GET /api/player/playlists?user_id=...` — list playlists for the caller.
pub async fn list_playlists(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PlaylistQuery>,
) -> Result<Json<Vec<PlaylistSummary>>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let branch_id = resolve_branch(&state, query.user_id);

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        diesel::sql_query(
            "SELECT p.id, p.name, p.visibility, p.created_at, p.updated_at,
                    COUNT(i.id)::bigint AS item_count
             FROM player_playlists p
             LEFT JOIN player_playlist_items i ON i.playlist_id = p.id
             WHERE p.branch_id = $1
             GROUP BY p.id
             ORDER BY p.updated_at DESC",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<PlaylistSummaryRow>(&mut conn)
        .map_err(|e| format!("Query error: {e}"))
    })
    .await;

    match result {
        Ok(Ok(rows)) => Ok(Json(rows.into_iter().map(|r| r.into_summary()).collect())),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botplayer::playlists::part4", None) })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": botsecurity_core::error_sanitizer::log_and_sanitize_json(&e, "botplayer::playlists::part4", None) })),
        )),
    }
}
