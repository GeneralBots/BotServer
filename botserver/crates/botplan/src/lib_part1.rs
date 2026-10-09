use super::*;

pub fn get_store() -> &'static PlanStore {
    STORE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}
/// Attach the database pool so mutations persist to `plan_snapshots`.
pub fn set_pool(pool: DbPool) {
    let _ = POOL.set(Some(pool));
}
/// Hydrate the in-memory store from any persisted plan snapshots. Fire-and-
/// forget: spawns a task so the route mount stays synchronous.
pub fn load_from_db() {
    let pool = match POOL.get().cloned().flatten() {
        Some(p) => p,
        None => return,
    };
    tokio::spawn(async move {
        #[derive(QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            plan_id: String,
            #[diesel(sql_type = diesel::sql_types::Jsonb)]
            payload: serde_json::Value,
        }
        let loaded = tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| e.to_string())?;
            diesel::sql_query("SELECT plan_id, payload FROM plan_snapshots")
                .load::<Row>(&mut conn)
                .map_err(|e| e.to_string())
        })
        .await;

        let rows = match loaded {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                log::warn!("plan load failed: {e}");
                return;
            }
            Err(e) => {
                log::warn!("plan load join failed: {e}");
                return;
            }
        };

        let mut store = get_store().write().await;
        for row in rows {
            match serde_json::from_value::<Plan>(row.payload) {
                Ok(plan) => {
                    store.entry(row.plan_id).or_insert(plan);
                }
                Err(e) => log::warn!("plan snapshot parse failed: {e}"),
            }
        }
    });
}
/// Persist every plan to `plan_snapshots` (one JSONB row per plan). Called
/// after each mutation; failures are logged, never fatal.
pub async fn persist_all() {
    let pool = match POOL.get().cloned().flatten() {
        Some(p) => p,
        None => return,
    };
    let snapshot: Vec<(String, serde_json::Value)> = {
        let store = get_store().read().await;
        store
            .iter()
            .filter_map(|(id, plan)| serde_json::to_value(plan).ok().map(|v| (id.clone(), v)))
            .collect()
    };
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let mut conn = pool.get().map_err(|e| e.to_string())?;
        for (plan_id, payload) in snapshot {
            diesel::sql_query(
                "INSERT INTO plan_snapshots (plan_id, payload, updated_at) \
                 VALUES ($1, $2, NOW()) \
                 ON CONFLICT (plan_id) DO UPDATE SET payload = EXCLUDED.payload, updated_at = NOW()",
            )
            .bind::<diesel::sql_types::Text, _>(&plan_id)
            .bind::<diesel::sql_types::Jsonb, _>(payload)
            .execute(&mut conn)
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    })
    .await;

    if let Err(e) = res {
        log::warn!("plan persist_all failed: {e}");
    }
}
pub fn configure_plan_routes() -> Router {
    Router::new()
        .route("/ws/plan/:plan_id", get(handle_plan_websocket))
        .route(
            "/api/plan/:plan_id/collaborators",
            get(handle_get_plan_collaborators),
        )
        .route(
            "/api/plan/:plan_id/presence",
            get(handle_get_plan_presence),
        )
        .route("/api/plan/:plan_id/typing", get(handle_get_plan_typing))
        .route("/api/plan/:plan_id", get(handle_get_plan))
        .route(
            "/api/plan/:plan_id/members",
            post(handle_add_member).delete(handle_remove_member),
        )
        .route("/api/plan/task", post(handle_create_task))
        .route("/api/plan/task/update", post(handle_update_task))
        .route("/api/plan/task/delete", post(handle_delete_task))
        .route("/api/plan/task/depends", post(handle_set_dependencies))
        .route("/api/plan/ai/assist", post(handle_plan_ai_assist))
}
pub async fn handle_get_plan(Path(plan_id): Path<String>) -> impl IntoResponse {
    let existing = {
        let store = get_store().read().await;
        store.get(&plan_id).cloned()
    };
    if let Some(plan) = existing {
        return Json(serde_json::json!({ "ok": true, "plan": plan }));
    }

    let plan = Plan {
        id: plan_id.clone(),
        title: "Novo Plano".to_string(),
        description: None,
        owner_id: "anonymous".to_string(),
        members: Vec::new(),
        tasks: HashMap::new(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    {
        let mut w = get_store().write().await;
        w.entry(plan_id).or_insert_with(|| plan.clone());
    }
    persist_all().await;
    Json(serde_json::json!({ "ok": true, "plan": plan, "created": true }))
}
pub async fn handle_create_task(Json(req): Json<CreateTaskRequest>) -> impl IntoResponse {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let task = Task {
        id: id.clone(),
        title: req.title,
        description: None,
        status: req.status.unwrap_or_else(|| "todo".to_string()),
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        assignee: req.assignee,
        start: req.start,
        end: req.end,
        progress: 0,
        tags: req.tags.unwrap_or_default(),
        depends_on: Vec::new(),
        created_at: now,
        updated_at: now,
        created_by: None,
    };
    {
        let mut store = get_store().write().await;
        let plan = store.entry(req.plan_id.clone()).or_insert_with(|| Plan {
            id: req.plan_id.clone(),
            title: "Plano".to_string(),
            description: None,
            owner_id: "anonymous".to_string(),
            members: Vec::new(),
            tasks: HashMap::new(),
            created_at: now,
            updated_at: now,
        });
        plan.tasks.insert(id.clone(), task.clone());
        plan.updated_at = now;
    }
    persist_all().await;
    Json(serde_json::json!({ "ok": true, "task": task }))
}
pub async fn handle_update_task(Json(req): Json<UpdateTaskRequest>) -> impl IntoResponse {
    let updated = {
        let mut store = get_store().write().await;
        if let Some(plan) = store.get_mut(&req.plan_id) {
            if let Some(t) = plan.tasks.get_mut(&req.task_id) {
                if let Some(v) = req.title { t.title = v; }
                if let Some(v) = req.status { t.status = v; }
                if let Some(v) = req.priority { t.priority = v; }
                if let Some(v) = req.assignee { t.assignee = Some(v); }
                if let Some(v) = req.start { t.start = Some(v); }
                if let Some(v) = req.end { t.end = Some(v); }
                if let Some(v) = req.progress { t.progress = v.min(100); }
                if let Some(v) = req.tags { t.tags = v; }
                if let Some(v) = req.depends_on { t.depends_on = v; }
                t.updated_at = Utc::now();
                plan.updated_at = Utc::now();
                Some(t.clone())
            } else {
                None
            }
        } else {
            None
        }
    };
    persist_all().await;
    match updated {
        Some(task) => Json(serde_json::json!({ "ok": true, "task": task })),
        None => Json(serde_json::json!({ "ok": false, "error": "task not found" })),
    }
}
pub async fn handle_add_member(
    Path(plan_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    let found = {
        let mut store = get_store().write().await;
        if let Some(plan) = store.get_mut(&plan_id) {
            if !plan.members.iter().any(|m| m == &req.user_id) {
                plan.members.push(req.user_id);
            }
            plan.updated_at = Utc::now();
            Some(plan.clone())
        } else {
            None
        }
    };
    persist_all().await;
    match found {
        Some(plan) => Json(serde_json::json!({ "ok": true, "plan": plan })),
        None => Json(serde_json::json!({ "ok": false, "error": "plan not found" })),
    }
}
pub async fn handle_remove_member(
    Path(plan_id): Path<String>,
    Json(req): Json<AddMemberRequest>,
) -> impl IntoResponse {
    let found = {
        let mut store = get_store().write().await;
        if let Some(plan) = store.get_mut(&plan_id) {
            plan.members.retain(|m| m != &req.user_id);
            plan.updated_at = Utc::now();
            Some(plan.clone())
        } else {
            None
        }
    };
    persist_all().await;
    match found {
        Some(plan) => Json(serde_json::json!({ "ok": true, "plan": plan })),
        None => Json(serde_json::json!({ "ok": false, "error": "plan not found" })),
    }
}
pub async fn handle_set_dependencies(Json(req): Json<SetDependenciesRequest>) -> impl IntoResponse {
    let updated = {
        let mut store = get_store().write().await;
        if let Some(plan) = store.get_mut(&req.plan_id) {
            if let Some(t) = plan.tasks.get_mut(&req.task_id) {
                t.depends_on = req.depends_on;
                t.updated_at = Utc::now();
                plan.updated_at = Utc::now();
                Some(t.clone())
            } else {
                None
            }
        } else {
            None
        }
    };
    persist_all().await;
    match updated {
        Some(task) => Json(serde_json::json!({ "ok": true, "task": task })),
        None => Json(serde_json::json!({ "ok": false, "error": "task not found" })),
    }
}
pub async fn handle_delete_task(Json(req): Json<serde_json::Value>) -> impl IntoResponse {
    let plan_id = req
        .get("plan_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let task_id = req
        .get("task_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let removed = {
        let mut store = get_store().write().await;
        if let Some(plan) = store.get_mut(&plan_id) {
            plan.tasks.remove(&task_id);
            plan.updated_at = Utc::now();
            true
        } else {
            false
        }
    };
    persist_all().await;
    if removed {
        Json(serde_json::json!({ "ok": true }))
    } else {
        Json(serde_json::json!({ "ok": false, "error": "plan not found" }))
    }
}
