//! HTTP surface for browser tasks, browsing memory and the admin policy.
//!
//! Routes (BRIEF contract): `POST /api/browser/tasks`,
//! `GET /api/browser/tasks/:id/events` (SSE),
//! `POST /api/browser/tasks/:id/pause|resume|cancel`,
//! `GET /api/browser/memory`. Additions: `GET /api/browser/tasks` (owner
//! listing), `POST /api/browser/tasks/:id/advance` and
//! `POST /api/browser/tasks/:id/finish` for the external execution driver,
//! `GET/PUT /api/browser/policy` (admin-only).

use crate::models::ProgressDoc;
use crate::policy::{PolicyConfig, TASK_KIND_CHECKOUT, TASK_KIND_DEFAULT};
use crate::tasks::{self, StepResult, TaskError};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    Json,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

const SSE_POLL_INTERVAL_MS: u64 = 1000;
const SSE_MAX_TICKS: u64 = 3600;
const POLICY_PREFERENCE_KEY: &str = "browser_policy";

/// Platform-global sentinel scope used to store the admin policy in the
/// existing `user_preferences` table without a dedicated migration.
const GLOBAL_SCOPE_USER: Uuid = Uuid::nil();

pub type SharedService = Arc<crate::BrowserPolicyService>;

fn api_error(context: &str, e: &TaskError) -> (StatusCode, String) {
    tracing::error!("browser policy {context}: {e}");
    match e {
        TaskError::NotFound => (StatusCode::NOT_FOUND, "Browser task not found".to_string()),
        TaskError::InvalidState(_) | TaskError::Policy(_) | TaskError::Budget(_) => {
            (StatusCode::CONFLICT, "Task state rejected".to_string())
        }
        TaskError::Db(_) | TaskError::Memory(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure".to_string())
        }
    }
}

fn resolve_user_id(headers: &HeaderMap) -> Result<Uuid, (StatusCode, String)> {
    let payload = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .and_then(jwt_payload);
    raw_subject_to_uuid(payload.as_ref())
}

/// SSE transport: EventSource cannot set headers, so the suite passes the
/// Bearer token as `?token=`. Header wins when both are present.
fn resolve_user_id_sse(
    headers: &HeaderMap,
    token_param: &str,
) -> Result<Uuid, (StatusCode, String)> {
    let header_token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or_default();
    let token = if header_token.is_empty() { token_param } else { header_token };
    if token.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string()));
    }
    raw_subject_to_uuid(jwt_payload(token).as_ref())
}

fn raw_subject_to_uuid(payload: Option<&serde_json::Value>) -> Result<Uuid, (StatusCode, String)> {
    let raw = payload
        .and_then(|p| p.get("uid").or_else(|| p.get("sub")))
        .and_then(|v| v.as_str());
    let Some(raw) = raw else {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required".to_string()));
    };
    Uuid::parse_str(raw).map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token subject".to_string()))
}

fn jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let decoded = base64_url_decode(parts[1]).ok()?;
    serde_json::from_slice::<serde_json::Value>(&decoded).ok()
}

fn base64_url_decode(input: &str) -> Result<Vec<u8>, String> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for ch in input.bytes() {
        if ch == b'=' {
            break;
        }
        let val = ALPHABET
            .iter()
            .position(|c| *c == ch)
            .ok_or_else(|| "invalid base64url character".to_string())? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
        }
    }
    Ok(out)
}

fn is_super_admin(svc: &SharedService, headers: &HeaderMap) -> Result<bool, (StatusCode, String)> {
    // Canonical platform-admin check (#1387) — same root-org membership
    // model as drive tenant isolation and botcloud admin gates.
    let mut conn = svc.pool().get().map_err(|e| {
        tracing::error!("browser policy admin check: pool unavailable: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure".to_string())
    })?;
    botsecurity_auth::platform_admin::is_platform_admin(&mut conn, headers).map_err(|e| {
        tracing::error!("browser policy admin check query failed: {e}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure".to_string())
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskBody {
    pub goal: String,
    #[serde(default)]
    pub domains: Vec<String>,
    pub bot_id: Option<Uuid>,
    pub org_id: Option<Uuid>,
    pub budget_steps: Option<i32>,
    pub policy: Option<PolicyConfig>,
    pub task_kind: Option<String>,
}

pub async fn create_task_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Json(body): Json<CreateTaskBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_id = resolve_user_id(&headers)?;
    if body.goal.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Goal must not be empty".to_string()));
    }
    let task_kind = match body.task_kind.as_deref() {
        None | Some(TASK_KIND_DEFAULT) => TASK_KIND_DEFAULT,
        Some(TASK_KIND_CHECKOUT) => TASK_KIND_CHECKOUT,
        Some(other) => {
            return Err((StatusCode::BAD_REQUEST, format!("Unknown task kind '{other}'")))
        }
    };
    let t = tasks::create_task(
        svc.pool(),
        tasks::NewTaskRequest {
            user_id,
            org_id: body.org_id,
            bot_id: body.bot_id,
            goal: body.goal,
            domains: body.domains,
            budget_steps: body.budget_steps.unwrap_or(60),
            policy_cfg: body.policy,
            task_kind,
        },
    )
    .map_err(|e| api_error("create task", &e))?;
    Ok(Json(serde_json::to_value(&t).unwrap_or_else(|_| serde_json::json!({}))))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    limit: Option<i64>,
}

pub async fn list_tasks_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_id = resolve_user_id(&headers)?;
    let items = tasks::list_user_tasks(svc.pool(), user_id, q.limit.unwrap_or(50))
        .map_err(|e| api_error("list tasks", &e))?;
    Ok(Json(serde_json::json!({ "items": items })))
}

async fn control(
    svc: SharedService,
    headers: HeaderMap,
    task_id: Uuid,
    f: fn(&tasks::DbPool, Uuid) -> Result<crate::models::BrowserTask, TaskError>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    resolve_user_id(&headers)?;
    let t = f(svc.pool(), task_id).map_err(|e| api_error("task control", &e))?;
    Ok(Json(serde_json::json!({ "status": t.status })))
}

pub async fn pause_handler(State(svc): State<SharedService>, headers: HeaderMap, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    control(svc, headers, id, tasks::pause_task).await
}

pub async fn resume_handler(State(svc): State<SharedService>, headers: HeaderMap, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    control(svc, headers, id, tasks::resume_task).await
}

pub async fn cancel_handler(State(svc): State<SharedService>, headers: HeaderMap, Path(id): Path<Uuid>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    control(svc, headers, id, tasks::cancel_task).await
}

#[derive(Debug, Deserialize)]
pub struct AdvanceBody {
    pub action: String,
    pub url: String,
    #[serde(default = "default_true")]
    pub ok: bool,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub cost_milli: u64,
    pub title: Option<String>,
    pub facts: Option<serde_json::Value>,
}

fn default_true() -> bool {
    true
}

/// Driver-facing endpoint invoking `tasks::advance_task`.
pub async fn advance_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<AdvanceBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    resolve_user_id(&headers)?;
    let (_, recorded) = tasks::advance_task(
        svc.pool(),
        id,
        StepResult {
            action: body.action,
            url: body.url,
            ok: body.ok,
            note: body.note,
            cost_milli: body.cost_milli,
            title: body.title,
            facts: body.facts,
        },
    )
    .await
    .map_err(|e| api_error("advance", &e))?;
    Ok(Json(serde_json::to_value(&recorded).unwrap_or_else(|_| serde_json::json!({}))))
}

#[derive(Debug, Deserialize)]
pub struct FinishBody {
    pub result: serde_json::Value,
    pub citations: Option<serde_json::Value>,
}

pub async fn finish_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(body): Json<FinishBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    resolve_user_id(&headers)?;
    let t = tasks::finish_task(svc.pool(), id, body.result, body.citations)
        .map_err(|e| api_error("finish", &e))?;
    Ok(Json(serde_json::json!({ "status": t.status })))
}

struct SseTick {
    task: crate::models::BrowserTask,
    new_steps: Vec<serde_json::Value>,
}

fn poll_tick(svc: &SharedService, task_id: Uuid, emitted: usize) -> Option<SseTick> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = svc.pool().get().ok()?;
    let task = bt::browser_tasks
        .find(task_id)
        .first::<crate::models::BrowserTask>(&mut conn)
        .optional()
        .ok()??;
    let progress = ProgressDoc::parse(task.progress.as_ref());
    let new_steps = progress.steps.iter().skip(emitted).filter_map(|s| serde_json::to_value(s).ok()).collect();
    Some(SseTick { task, new_steps })
}

/// `GET /api/browser/tasks/:id/events` — Server-Sent Events stream polling the
/// persisted progress document every second; emits each new step once and a
/// terminal `task_status` event before closing.
pub async fn task_events(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>, (StatusCode, String)>
{
    resolve_user_id_sse(
        &headers,
        query.get("token").map(|s| s.as_str()).unwrap_or_default(),
    )?;
    let stream = async_stream::stream! {
        let mut emitted: usize = 0;
        for _ in 0..SSE_MAX_TICKS {
            if let Some(tick) = poll_tick(&svc, task_id, emitted) {
                for step in tick.new_steps {
                    emitted += 1;
                    yield Ok(Event::default().event("step").data(step.to_string()));
                }
                if matches!(tick.task.status.as_str(), tasks::STATUS_FINISHED | tasks::STATUS_FAILED | tasks::STATUS_CANCELLED) {
                    if let Ok(data) = serde_json::to_string(&tick.task) {
                        yield Ok(Event::default().event("task_status").data(data));
                    }
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(SSE_POLL_INTERVAL_MS)).await;
        }
    };
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

#[derive(Debug, Deserialize)]
pub struct MemoryQuery {
    limit: Option<i64>,
}

pub async fn memory_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Query(q): Query<MemoryQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_id = resolve_user_id(&headers)?;
    let items = crate::memory::top_facts(svc.pool(), user_id, q.limit.unwrap_or(50))
        .map_err(|e| api_error("memory", &TaskError::Memory(e)))?;
    Ok(Json(serde_json::json!({ "items": items })))
}

pub async fn delete_memory_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let user_id = resolve_user_id(&headers)?;
    let removed = crate::memory::purge_user(svc.pool(), user_id)
        .map_err(|e| api_error("purge memory", &TaskError::Memory(e)))?;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

fn read_global_policy(svc: &SharedService) -> PolicyConfig {
    use crate::schema::user_preferences::dsl as up;

    let Ok(mut conn) = svc.pool().get() else {
        return PolicyConfig::default();
    };
    up::user_preferences
        .filter(up::user_id.eq(GLOBAL_SCOPE_USER))
        .filter(up::preference_key.eq(POLICY_PREFERENCE_KEY))
        .select(up::preference_value)
        .first::<serde_json::Value>(&mut conn)
        .optional()
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_value::<PolicyConfig>(v).ok())
        .unwrap_or_default()
}

pub async fn get_policy_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !is_super_admin(&svc, &headers)? {
        return Err((StatusCode::FORBIDDEN, "Administrator access required".to_string()));
    }
    let cfg = read_global_policy(&svc);
    Ok(Json(serde_json::to_value(&cfg).unwrap_or_else(|_| serde_json::json!({}))))
}

pub async fn put_policy_handler(
    State(svc): State<SharedService>,
    headers: HeaderMap,
    Json(cfg): Json<PolicyConfig>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !is_super_admin(&svc, &headers)? {
        return Err((StatusCode::FORBIDDEN, "Administrator access required".to_string()));
    }
    use crate::schema::user_preferences::dsl as up;

    let value = serde_json::to_value(&cfg)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid policy document".to_string()))?;
    let mut conn = svc
        .pool()
        .get()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure".to_string()))?;
    diesel::insert_into(up::user_preferences)
        .values((
            up::id.eq(Uuid::new_v4()),
            up::user_id.eq(GLOBAL_SCOPE_USER),
            up::preference_key.eq(POLICY_PREFERENCE_KEY),
            up::preference_value.eq(value.clone()),
        ))
        .on_conflict((up::user_id, up::preference_key))
        .do_update()
        .set((up::preference_value.eq(value.clone()), up::updated_at.eq(diesel::dsl::now)))
        .execute(&mut conn)
        .map_err(|e| {
            tracing::error!("browser policy persist failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Storage failure".to_string())
        })?;
    Ok(Json(value))
}

/// Router fragment merged by the integrator under the authenticated scope.
pub fn configure_routes() -> axum::Router<SharedService> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/api/browser/tasks", get(list_tasks_handler).post(create_task_handler))
        .route("/api/browser/tasks/:id/events", get(task_events))
        .route("/api/browser/tasks/:id/pause", post(pause_handler))
        .route("/api/browser/tasks/:id/resume", post(resume_handler))
        .route("/api/browser/tasks/:id/cancel", post(cancel_handler))
        .route("/api/browser/tasks/:id/advance", post(advance_handler))
        .route("/api/browser/tasks/:id/finish", post(finish_handler))
        .route("/api/browser/memory", get(memory_handler).delete(delete_memory_handler))
        .route("/api/browser/policy", get(get_policy_handler).put(put_policy_handler))
}
