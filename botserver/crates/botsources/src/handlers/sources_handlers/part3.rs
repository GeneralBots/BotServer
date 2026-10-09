use super::*;

pub async fn handle_install_skill(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<InstallSkillRequest>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let skill_id = payload.name.unwrap_or_default();
    let bot_id = payload
        .bot_id
        .unwrap_or_else(|| "default".to_string());

    if skill_id.trim().is_empty() {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "ok": false, "error": "skill name required" })),
        );
    }

    let pool = state.conn.clone();
    let skill_id_c = skill_id.clone();
    let bot_id_c = bot_id.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return Err("database unavailable".to_string());
            }
        };
        let _ = diesel::sql_query(
            "INSERT INTO bot_skills (bot_id, skill_id, name, installed_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (bot_id, skill_id) DO NOTHING",
        )
        .bind::<diesel::sql_types::Text, _>(&bot_id_c)
        .bind::<diesel::sql_types::Text, _>(&skill_id_c)
        .bind::<diesel::sql_types::Text, _>(&skill_id_c)
        .execute(&mut conn);
        Ok::<(), String>(())
    })
    .await;

    match result {
        Ok(Ok(())) => (
            axum::http::StatusCode::OK,
            Json(serde_json::json!({
                "ok": true,
                "skill": skill_id,
                "bot_id": bot_id,
                "message": "Skill installed successfully",
            })),
        ),
        _ => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "error": "skill install failed" })),
        ),
    }
}
