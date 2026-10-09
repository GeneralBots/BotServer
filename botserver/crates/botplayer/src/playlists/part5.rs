use super::*;

impl PlaylistSummaryRow {
    pub(super) fn into_summary(self) -> PlaylistSummary {
        PlaylistSummary {
            id: self.id,
            name: self.name,
            visibility: self.visibility,
            item_count: self.item_count,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
/// `POST /api/player/playlists` — create a playlist.
pub async fn create_playlist(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreatePlaylistRequest>,
) -> Result<Json<PlaylistSummary>, (StatusCode, Json<serde_json::Value>)> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > MAX_TITLE_LEN {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Playlist name is required (max 255 chars)" })),
        ));
    }
    let visibility = match body.visibility.as_str() {
        "private" | "org" | "public" => body.visibility.clone(),
        _ => "private".to_string(),
    };

    let id = Uuid::new_v4();
    let now = Utc::now();
    let pool = state.conn.clone();
    let branch_id = resolve_branch(&state, None);
    let resp_name = name.clone();
    let resp_visibility = visibility.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
        diesel::sql_query(
            "INSERT INTO player_playlists (id, branch_id, name, visibility, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .bind::<diesel::sql_types::Text, _>(&name)
        .bind::<diesel::sql_types::Text, _>(&visibility)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(&mut conn)
        .map_err(|e| format!("Insert error: {e}"))?;
        Ok::<_, String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => Ok(Json(PlaylistSummary {
            id,
            name: resp_name,
            visibility: resp_visibility,
            item_count: 0,
            created_at: now,
            updated_at: now,
        })),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Database error: {e}") })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("Task join error: {e}") })),
        )),
    }
}
/// `GET /api/player/playlists/{id}` — playlist detail with ordered items.
pub async fn get_playlist(
    State(state): State<Arc<AppState>>,
    Path(playlist_id): Path<Uuid>,
) -> Result<Json<PlaylistDetail>, (StatusCode, Json<serde_json::Value>)> {
    let pool = state.conn.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

        #[derive(diesel::QueryableByName)]
        struct PlaylistRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            visibility: String,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: chrono::DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            updated_at: chrono::DateTime<Utc>,
        }
        let row: Option<PlaylistRow> = diesel::sql_query(
            "SELECT id, name, visibility, created_at, updated_at
             FROM player_playlists WHERE id = $1",
        )
        .bind::<diesel::sql_types::Uuid, _>(playlist_id)
        .get_result(&mut conn)
        .optional()
        .map_err(|e| format!("Query error: {e}"))?;

        let row = row.ok_or_else(|| "Playlist not found".to_string())?;
        let id = row.id;
        let name = row.name;
        let visibility = row.visibility;
        let created_at = row.created_at;
        let updated_at = row.updated_at;

        let items: Vec<PlaylistItem> = diesel::sql_query(
            "SELECT id, media_path, title, position
             FROM player_playlist_items
             WHERE playlist_id = $1
             ORDER BY position ASC",
        )
        .bind::<diesel::sql_types::Uuid, _>(playlist_id)
        .load::<PlaylistItemRow>(&mut conn)
        .map_err(|e| format!("Query error: {e}"))?
        .into_iter()
        .map(|r| PlaylistItem {
            id: r.id,
            media_path: r.media_path,
            title: r.title,
            position: r.position,
        })
        .collect();

        Ok::<_, String>(PlaylistDetail {
            id,
            name,
            visibility,
            items,
            created_at,
            updated_at,
        })
    })
    .await;

    match result {
        Ok(Ok(detail)) => Ok(Json(detail)),
        Ok(Err(e)) => {
            let status = if e == "Playlist not found" {
                StatusCode::NOT_FOUND
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
