use super::*;

pub fn configure_itsm_routes() -> Router<Arc<TicketsState>> {
    Router::new()
        .route("/api/tickets/cis", get(list_cis).post(create_ci))
        .route("/api/tickets/cis/:id", get(get_ci).put(update_ci).delete(delete_ci))
        .route("/api/tickets/kb", get(list_kb_articles).post(create_kb_article))
        .route("/api/tickets/kb/:id", get(get_kb_article).put(update_kb_article).delete(delete_kb_article))
}
pub(super) fn resolve_branch_id(state: &Arc<TicketsState>) -> Uuid {
    let Ok(mut conn) = state.pool.get() else {
        return Uuid::nil();
    };
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BranchRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: Uuid,
    }
    diesel::sql_query(
        "SELECT branch_id FROM bots WHERE is_default_for_branch = TRUE ORDER BY created_at ASC LIMIT 1",
    )
    .get_result::<BranchRow>(&mut conn)
    .map(|r| r.branch_id)
    .unwrap_or(Uuid::nil())
}
pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub(super) async fn list_cis(State(state): State<Arc<TicketsState>>) -> impl IntoResponse {
    let branch_id = resolve_branch_id(&state);
    let conn = state.pool.clone();

    let rows: Vec<CiRow> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return Vec::new();
            }
        };

        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            ci_type: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            description: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
        }

        diesel::sql_query(
            "SELECT id, name, ci_type, description, status FROM ticket_cis WHERE branch_id = $1 ORDER BY name ASC LIMIT 100",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<Row>(&mut db_conn)
        .map(|rows| {
            rows.into_iter()
                .map(|r| CiRow {
                    id: r.id,
                    name: r.name,
                    ci_type: r.ci_type,
                    description: r.description,
                    status: r.status,
                })
                .collect()
        })
        .unwrap_or_else(|e| {
            log::error!("Failed to list CIs: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    Json(rows)
}
pub(super) async fn create_ci(
    State(state): State<Arc<TicketsState>>,
    Json(payload): Json<NewCiRequest>,
) -> Result<Json<CiRow>, (axum::http::StatusCode, String)> {
    if payload.name.trim().is_empty() {
        return Err((axum::http::StatusCode::BAD_REQUEST, "name is required".to_string()));
    }

    let branch_id = resolve_branch_id(&state);
    let conn = state.pool.clone();
    let id = Uuid::new_v4();
    let name = payload.name.clone();
    let ci_type = payload.ci_type.clone();
    let description = payload.description.clone();
    let status = payload.status.clone().unwrap_or_else(|| "operational".to_string());

    let (name_c, ci_type_c, description_c, status_c) = (
        name.clone(),
        ci_type.clone(),
        description.clone(),
        status.clone(),
    );

    tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return;
            }
        };
        let _ = diesel::sql_query(
            "INSERT INTO ticket_cis (id, org_id, branch_id, name, ci_type, description, status) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind::<diesel::sql_types::Uuid, _>(&id)
        .bind::<diesel::sql_types::Uuid, _>(&Uuid::nil())
        .bind::<diesel::sql_types::Uuid, _>(&branch_id)
        .bind::<diesel::sql_types::Text, _>(&name_c)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&ci_type_c)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&description_c)
        .bind::<diesel::sql_types::Text, _>(&status_c)
        .execute(&mut db_conn);
    })
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Task error: {e}")))?;

    Ok(Json(CiRow {
        id,
        name,
        ci_type,
        description,
        status,
    }))
}
pub(super) async fn get_ci(State(state): State<Arc<TicketsState>>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.pool.clone();

    let row: Option<CiRow> = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return None;
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            ci_type: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            description: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
        }
        diesel::sql_query("SELECT id, name, ci_type, description, status FROM ticket_cis WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(&id)
            .get_result::<Row>(&mut db_conn)
            .ok()
            .map(|r| CiRow {
                id: r.id,
                name: r.name,
                ci_type: r.ci_type,
                description: r.description,
                status: r.status,
            })
    })
    .await
    .unwrap_or(None);

    match row {
        Some(r) => Json(r).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "CI not found").into_response(),
    }
}
pub(super) async fn update_ci(
    State(state): State<Arc<TicketsState>>,
    Path(id): Path<Uuid>,
    Json(payload): Json<NewCiRequest>,
) -> Result<Json<CiRow>, (axum::http::StatusCode, String)> {
    let conn = state.pool.clone();
    let name = payload.name.clone();
    let ci_type = payload.ci_type.clone();
    let description = payload.description.clone();
    let status = payload.status.clone().unwrap_or_else(|| "operational".to_string());

    let (name_c, ci_type_c, description_c, status_c) = (
        name.clone(),
        ci_type.clone(),
        description.clone(),
        status.clone(),
    );

    tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return;
            }
        };
        let _ = diesel::sql_query(
            "UPDATE ticket_cis SET name = $1, ci_type = $2, description = $3, status = $4, updated_at = NOW() WHERE id = $5",
        )
        .bind::<diesel::sql_types::Text, _>(&name_c)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&ci_type_c)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&description_c)
        .bind::<diesel::sql_types::Text, _>(&status_c)
        .bind::<diesel::sql_types::Uuid, _>(&id)
        .execute(&mut db_conn);
    })
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Task error: {e}")))?;

    Ok(Json(CiRow {
        id,
        name,
        ci_type,
        description,
        status,
    }))
}
pub(super) async fn delete_ci(State(state): State<Arc<TicketsState>>, Path(id): Path<Uuid>) -> impl IntoResponse {
    let conn = state.pool.clone();
    match tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                log::error!("DB connection error: {e}");
                return;
            }
        };
        let _ = diesel::sql_query("DELETE FROM ticket_cis WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(&id)
            .execute(&mut db_conn);
    })
    .await
    {
        Ok(()) => (),
        Err(e) => log::error!("Delete CI task error: {e}"),
    }

    axum::http::StatusCode::NO_CONTENT.into_response()
}
