use super::*;

/// `PATCH /api/player/playlists/{id}` — rename or change visibility.
pub async fn update_playlist(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= MAX_TITLE_LEN);
    let visibility = body.get("visibility").and_then(|v| v.as_str());

    if name.is_none() && visibility.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Provide name and/or visibility" })),
        ));
    }

    let pool = state.conn.clone();
    let name = name.map(str::to_string);
    let visibility = visibility.map(str::to_string);

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        if let Some(name) = &name {
            diesel::sql_query(
                "UPDATE player_playlists SET name = $2, updated_at = NOW() WHERE id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .bind::<diesel::sql_types::Text, _>(name)
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {e}"))?;
        }
        if let Some(visibility) = &visibility {
            diesel::sql_query(
                "UPDATE player_playlists SET visibility = $2, updated_at = NOW() WHERE id = $1",
            )
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .bind::<diesel::sql_types::Text, _>(visibility)
            .execute(&mut conn)
            .map_err(|e| format!("Update error: {e}"))?;
        }
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({ "success": true }))),
        Ok(Err(e)) => Err(db_err(e)),
        Err(e) => Err(db_err(format!("Task join error: {e}"))),
    }
}
/// `POST /api/player/playlists/{id}/items` — add a media item to a playlist.
pub async fn add_item(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
    Json(body): Json<AddItemRequest>,
) -> Result<Json<PlaylistItem>, (StatusCode, Json<serde_json::Value>)> {
    let media_path = body.media_path.trim().to_string();
    let title = if body.title.trim().is_empty() {
        media_path
            .rsplit('/')
            .next()
            .unwrap_or(&media_path)
            .to_string()
    } else {
        body.title.trim().to_string()
    };

    if media_path.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "media_path is required" })),
        ));
    }

    let pool = state.conn.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

        let count: i64 = diesel::sql_query(
            "SELECT COUNT(*)::bigint AS count FROM player_playlist_items WHERE playlist_id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(playlist_id)
        .get_result::<CountRow>(&mut conn)
        .map_err(|e| format!("Count error: {e}"))?
        .count;

        if count >= MAX_PLAYLIST_ITEMS as i64 {
            return Err(format!("Playlist limit of {MAX_PLAYLIST_ITEMS} items reached"));
        }

        let position = count as i32;
        let id = Uuid::new_v4();
        diesel::sql_query(
            "INSERT INTO player_playlist_items (id, playlist_id, media_path, title, position)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(playlist_id)
        .bind::<diesel::sql_types::Text, _>(&media_path)
        .bind::<diesel::sql_types::Text, _>(&title)
        .bind::<diesel::sql_types::Int4, _>(position)
        .execute(&mut conn)
        .map_err(|e| format!("Insert error: {e}"))?;

        diesel::sql_query("UPDATE player_playlists SET updated_at = NOW() WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(playlist_id)
            .execute(&mut conn)
            .map_err(|e| format!("Touch error: {e}"))?;

        Ok::<_, String>(PlaylistItem {
            id,
            media_path,
            title,
            position,
        })
    })
    .await;

    match result {
        Ok(Ok(item)) => Ok(Json(item)),
        Ok(Err(e)) => {
            let status = if e.contains("limit") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            Err((status, Json(serde_json::json!({ "error": e }))))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Join error: {e}") })),
        )),
    }
}
