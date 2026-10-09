use super::*;

/// `GET /api/collab/conflicts?resource_type=&resource_id=` — unresolved
/// concurrent ops awaiting a resolution decision.
pub async fn list_conflicts(
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<AuthenticatedUser>,
    Query(params): Query<ConflictQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    if !sanitize_resource(&params.resource_type, &params.resource_id) {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid resource_type/resource_id"));
    }
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        id: String,
        #[diesel(sql_type = Text)]
        actor_id: String,
        #[diesel(sql_type = Text)]
        actor_name: String,
        #[diesel(sql_type = Text)]
        actor_type: String,
        #[diesel(sql_type = Text)]
        op_type: String,
        #[diesel(sql_type = BigInt)]
        base_version: i64,
        #[diesel(sql_type = BigInt)]
        lamport_ts: i64,
        #[diesel(sql_type = Text)]
        payload: String,
        #[diesel(sql_type = Timestamptz)]
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let rows = diesel::sql_query(
        "SELECT id::text, actor_id, actor_name, actor_type, op_type, base_version, \
                lamport_ts, payload, created_at \
         FROM collab_ops \
         WHERE resource_type = $1 AND resource_id = $2 \
           AND conflict = TRUE AND resolved = FALSE \
         ORDER BY lamport_ts ASC",
    )
    .bind::<Text, _>(&params.resource_type)
    .bind::<Text, _>(&params.resource_id)
    .load::<Row>(&mut conn)
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("list conflicts: {e}")))?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            let payload = serde_json::from_str::<serde_json::Value>(&r.payload).unwrap_or(serde_json::Value::Null);
            serde_json::json!({
                "id": r.id,
                "actor_id": r.actor_id,
                "actor_name": r.actor_name,
                "actor_type": r.actor_type,
                "op_type": r.op_type,
                "base_version": r.base_version,
                "lamport": r.lamport_ts,
                "payload": payload,
                "created_at": r.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(items))
}
/// `POST /api/collab/conflicts/:id/resolve` — resolve a concurrent op.
/// `accept-server` discards the editor change; `accept-client` rebases it on
/// top of the converged state (advancing the resource version).
pub async fn resolve_conflict(
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<AuthenticatedUser>,
    Path(id): Path<uuid::Uuid>,
    Json(body): Json<ResolveBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let resolution = match body.resolution.as_str() {
        "accept-server" => "rejected",
        "accept-client" => "accepted",
        _ => return Err(err(StatusCode::BAD_REQUEST, "resolution must be accept-server or accept-client")),
    };

    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    // Fetch the op's Lamport timestamp and resource address first.
    #[derive(QueryableByName)]
    struct OpRef {
        #[diesel(sql_type = Text)]
        resource_type: String,
        #[diesel(sql_type = Text)]
        resource_id: String,
        #[diesel(sql_type = Text)]
        actor_id: String,
        #[diesel(sql_type = BigInt)]
        lamport_ts: i64,
    }
    let op = diesel::sql_query(
        "SELECT resource_type, resource_id, actor_id, lamport_ts FROM collab_ops WHERE id = $1",
    )
    .bind::<SqlUuid, _>(id)
    .get_result::<OpRef>(&mut conn)
    .optional()
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("load op: {e}")))?;

    let Some(op) = op else {
        return Err(err(StatusCode::NOT_FOUND, "Operation not found"));
    };

    let updated = diesel::sql_query(
        "UPDATE collab_ops SET resolved = TRUE, resolution = $2 \
         WHERE id = $1 AND conflict = TRUE AND resolved = FALSE",
    )
    .bind::<SqlUuid, _>(id)
    .bind::<Text, _>(resolution)
    .execute(&mut conn)
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("resolve op: {e}")))?;

    if updated == 0 {
        return Err(err(StatusCode::CONFLICT, "Operation is not an unresolved conflict"));
    }

    // accept-client advances the converged version past the applied change and
    // records the rebased op in the version vector.
    if resolution == "accepted" {
        let current = read_current_version(&mut conn, &op.resource_type, &op.resource_id)
            .unwrap_or(0);
        let new_version = current.max(op.lamport_ts);
        let mut vector = read_vector(&mut conn, &op.resource_type, &op.resource_id).unwrap_or_default();
        vector.insert(op.actor_id.clone(), op.lamport_ts);
        let vector_json = serde_json::to_string(&vector).unwrap_or_else(|_| "{}".to_string());
        if let Err(e) = upsert_state(&mut conn, &op.resource_type, &op.resource_id, new_version, &vector_json) {
            warn!("resolve op: state upsert failed: {e}");
        }
    }

    Ok(Json(serde_json::json!({ "success": true, "resolution": resolution })))
}
/// `GET /api/collab/ops/actors?resource_type=&resource_id=` — distinct actors
/// grouped by `actor_type` (human vs llm) with their last Lamport timestamp and
/// op count, so a client can render who (or which AI) edited concurrently.
pub async fn list_actors(
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<AuthenticatedUser>,
    Query(params): Query<ConflictQuery>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<serde_json::Value>)> {
    if !sanitize_resource(&params.resource_type, &params.resource_id) {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid resource_type/resource_id"));
    }
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Text)]
        actor_id: String,
        #[diesel(sql_type = Text)]
        actor_name: String,
        #[diesel(sql_type = Text)]
        actor_type: String,
        #[diesel(sql_type = BigInt)]
        last_ts: i64,
        #[diesel(sql_type = BigInt)]
        op_count: i64,
    }

    let rows = diesel::sql_query(
        "SELECT actor_id, MAX(actor_name) AS actor_name, actor_type, \
                MAX(lamport_ts) AS last_ts, COUNT(*) AS op_count \
         FROM collab_ops \
         WHERE resource_type = $1 AND resource_id = $2 \
         GROUP BY actor_id, actor_type \
         ORDER BY last_ts DESC",
    )
    .bind::<Text, _>(&params.resource_type)
    .bind::<Text, _>(&params.resource_id)
    .load::<Row>(&mut conn)
    .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("list actors: {e}")))?;

    let items: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| {
            serde_json::json!({
                "actor_id": r.actor_id,
                "actor_name": r.actor_name,
                "actor_type": r.actor_type,
                "last_lamport": r.last_ts,
                "op_count": r.op_count,
            })
        })
        .collect();

    Ok(Json(items))
}
/// `GET /api/collab/ops/state?resource_type=&resource_id=` — the converged
/// version + version vector, so a client can base its next op correctly.
pub async fn ops_state(
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<AuthenticatedUser>,
    Query(params): Query<ConflictQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !sanitize_resource(&params.resource_type, &params.resource_id) {
        return Err(err(StatusCode::BAD_REQUEST, "Invalid resource_type/resource_id"));
    }
    let mut conn = state
        .conn
        .get()
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &format!("db pool: {e}")))?;

    let current = read_current_version(&mut conn, &params.resource_type, &params.resource_id)
        .unwrap_or(0);
    let vector = read_vector(&mut conn, &params.resource_type, &params.resource_id)
        .unwrap_or_default();

    Ok(Json(serde_json::json!({
        "resource_type": params.resource_type,
        "resource_id": params.resource_id,
        "current_version": current,
        "version_vector": vector,
    })))
}
pub fn configure_collab_ops_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/collab/ops", post(submit_op).get(list_ops))
        .route("/api/collab/ops/state", get(ops_state))
        .route("/api/collab/ops/actors", get(list_actors))
        .route("/api/collab/conflicts", get(list_conflicts))
        .route("/api/collab/conflicts/:id/resolve", post(resolve_conflict))
}
