use super::*;

/// `DELETE /api/player/playlists/{id}/items/{item_id}` — remove an item.
pub async fn remove_item(
    State(state): State<Arc<AppState>>,
    Path((playlist_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        diesel::sql_query("DELETE FROM player_playlist_items WHERE id = $1 AND playlist_id = $2")
            .bind::<diesel::sql_types::Uuid, _>(item_id)
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .execute(&mut conn)
            .map_err(|e| format!("Delete error: {e}"))?;
        diesel::sql_query("UPDATE player_playlists SET updated_at = NOW() WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .execute(&mut conn)
            .map_err(|e| format!("Touch error: {e}"))?;
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
/// `PUT /api/player/playlists/{id}/reorder` — apply a new item order.
pub async fn reorder_items(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
    Json(body): Json<ReorderRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let item_ids = body.item_ids.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        for (index, item_id) in item_ids.iter().enumerate() {
            diesel::sql_query(
                "UPDATE player_playlist_items SET position = $3
                 WHERE id = $1 AND playlist_id = $2",
            )
            .bind::<diesel::sql_types::Uuid, _>(item_id)
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .bind::<diesel::sql_types::Int4, _>(index as i32)
            .execute(&mut conn)
            .map_err(|e| format!("Reorder error: {e}"))?;
        }
        diesel::sql_query("UPDATE player_playlists SET updated_at = NOW() WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .execute(&mut conn)
            .map_err(|e| format!("Touch error: {e}"))?;
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
/// `DELETE /api/player/playlists/{id}` — delete a playlist.
pub async fn delete_playlist(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        diesel::sql_query("DELETE FROM player_playlists WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .execute(&mut conn)
            .map_err(|e| format!("Delete error: {e}"))?;
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
/// `POST /api/player/playbacks` — record a playback event for analytics.
pub async fn record_playback(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PlaybackEventRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let id = Uuid::new_v4();
    let position_seconds = body.position_seconds.unwrap_or(0);
    let event_type = match body.event_type.as_str() {
        "play" | "pause" | "resume" | "complete" => body.event_type.clone(),
        _ => "play".to_string(),
    };

    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        diesel::sql_query(
            "INSERT INTO player_playback_events
             (id, playlist_id, item_id, media_path, event_type, position_seconds, created_at)
             VALUES ($1, NULL, $2, $3, $4, $5, NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Uuid>, _>(body.item_id)
        .bind::<diesel::sql_types::Text, _>(&body.media_path)
        .bind::<diesel::sql_types::Text, _>(&event_type)
        .bind::<diesel::sql_types::Int4, _>(position_seconds)
        .execute(&mut conn)
        .map_err(|e| format!("Insert error: {e}"))?;
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
/// `GET /api/player/playlists/{id}/analytics` — per-item plays/completion.
pub async fn playlist_analytics(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        let rows: Vec<AnalyticsRow> = diesel::sql_query(
            "SELECT media_path,
                    COUNT(*) FILTER (WHERE event_type = 'play')::bigint AS plays,
                    COUNT(*) FILTER (WHERE event_type = 'complete')::bigint AS completions,
                    COALESCE(SUM(position_seconds) FILTER (WHERE event_type = 'play'), 0)::bigint AS watch_seconds
             FROM player_playback_events
             WHERE playlist_id = $1
             GROUP BY media_path
             ORDER BY plays DESC",
        )
        .bind::<diesel::sql_types::Uuid, _>(playlist_id)
        .load::<AnalyticsRow>(&mut conn)
        .map_err(|e| format!("Query error: {e}"))?;

        let items: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|r| {
                serde_json::json!({
                    "media_path": r.media_path,
                    "plays": r.plays,
                    "completions": r.completions,
                    "watch_seconds": r.watch_seconds,
                })
            })
            .collect();

        Ok::<_, String>(serde_json::json!({ "items": items }))
    })
    .await;

    match result {
        Ok(Ok(value)) => Ok(Json(value)),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
