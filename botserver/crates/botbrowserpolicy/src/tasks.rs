//! Browser task lifecycle orchestration.
//!
//! The execution driver is out of scope for this crate; an external driver
//! service calls `advance_task` with each completed step. Policy and budget
//! validation happen BEFORE a step is recorded, and violations mark the task
//! as failed with the reason persisted in `error`.

use crate::budget::{BudgetExceeded, BudgetTracker};
use crate::memory::{self, MemoryError};
use crate::models::{BrowserTask, NewBrowserTask, ProgressDoc, ProgressStep};
use crate::policy::{self, PolicyConfig};
use chrono::Utc;
use diesel::prelude::*;
use std::fmt;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

pub const STATUS_QUEUED: &str = "queued";
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_PAUSED: &str = "paused";
pub const STATUS_FINISHED: &str = "finished";
pub const STATUS_FAILED: &str = "failed";
pub const STATUS_CANCELLED: &str = "cancelled";

#[derive(Debug, Clone, PartialEq)]
pub enum TaskError {
    NotFound,
    InvalidState(String),
    Db(String),
    Policy(String),
    Budget(BudgetExceeded),
    Memory(MemoryError),
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "browser task not found"),
            Self::InvalidState(m) => write!(f, "invalid browser task state: {m}"),
            Self::Db(m) => write!(f, "browser task database error: {m}"),
            Self::Policy(m) => write!(f, "policy violation: {m}"),
            Self::Budget(e) => write!(f, "{e}"),
            Self::Memory(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TaskError {}

/// Driver-submitted result of one browsing step.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub action: String,
    pub url: String,
    pub ok: bool,
    pub note: String,
    pub cost_milli: u64,
    pub title: Option<String>,
    pub facts: Option<serde_json::Value>,
}

fn terminal(status: &str) -> bool {
    matches!(
        status,
        STATUS_FINISHED | STATUS_FAILED | STATUS_CANCELLED
    )
}

fn load_task(pool: &DbPool, task_id: Uuid) -> Result<BrowserTask, TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    bt::browser_tasks
        .find(task_id)
        .first::<BrowserTask>(&mut conn)
        .optional()
        .map_err(|e| TaskError::Db(e.to_string()))?
        .ok_or(TaskError::NotFound)
}

fn set_status(
    pool: &DbPool,
    task_id: Uuid,
    status: &str,
    stamp_started: bool,
) -> Result<BrowserTask, TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    let target = bt::browser_tasks.find(task_id);
    if terminal(status) {
        diesel::update(target)
            .set((
                bt::status.eq(status.to_string()),
                bt::finished_at.eq(diesel::dsl::now),
            ))
            .get_result::<BrowserTask>(&mut conn)
    } else if stamp_started {
        diesel::update(target)
            .set((
                bt::status.eq(status.to_string()),
                bt::started_at.eq(diesel::dsl::now),
            ))
            .get_result::<BrowserTask>(&mut conn)
    } else {
        diesel::update(target)
            .set(bt::status.eq(status.to_string()))
            .get_result::<BrowserTask>(&mut conn)
    }
    .optional()
    .map_err(|e| TaskError::Db(e.to_string()))?
    .ok_or(TaskError::NotFound)
}

/// Creates a queued task. When `policy` is provided it is embedded in the
/// `plan` JSONB under key `policy`; otherwise defaults apply at advance time.
/// `task_kind` (`default` | `checkout`) is stored alongside it and drives the
/// credential-phishing guard.
pub fn create_task(
    pool: &DbPool,
    user_id: Uuid,
    org_id: Option<Uuid>,
    bot_id: Option<Uuid>,
    goal: String,
    domains: Vec<String>,
    budget_steps: i32,
    policy_cfg: Option<PolicyConfig>,
    task_kind: &str,
) -> Result<BrowserTask, TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let plan = policy_cfg.map(|p| serde_json::json!({ "policy": p, "kind": task_kind }));
    let new_task = NewBrowserTask {
        id: Uuid::new_v4(),
        user_id,
        org_id,
        bot_id,
        goal,
        plan,
        domains: serde_json::to_value(&domains).unwrap_or_else(|_| serde_json::json!([])),
        budget_steps: budget_steps.clamp(1, 10_000),
        status: STATUS_QUEUED.to_string(),
    };
    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    diesel::insert_into(bt::browser_tasks)
        .values(&new_task)
        .returning(BrowserTask::as_returning())
        .get_result::<BrowserTask>(&mut conn)
        .map_err(|e| TaskError::Db(e.to_string()))
}

pub fn start_task(pool: &DbPool, task_id: Uuid) -> Result<BrowserTask, TaskError> {
    let t = load_task(pool, task_id)?;
    if t.status != STATUS_QUEUED && t.status != STATUS_PAUSED {
        return Err(TaskError::InvalidState(format!("cannot start from '{}'", t.status)));
    }
    set_status(pool, task_id, STATUS_RUNNING, t.started_at.is_none())
}

pub fn pause_task(pool: &DbPool, task_id: Uuid) -> Result<BrowserTask, TaskError> {
    let t = load_task(pool, task_id)?;
    if t.status != STATUS_RUNNING {
        return Err(TaskError::InvalidState("only running tasks can be paused".to_string()));
    }
    set_status(pool, task_id, STATUS_PAUSED, false)
}

pub fn resume_task(pool: &DbPool, task_id: Uuid) -> Result<BrowserTask, TaskError> {
    let t = load_task(pool, task_id)?;
    if t.status != STATUS_PAUSED {
        return Err(TaskError::InvalidState("only paused tasks can be resumed".to_string()));
    }
    set_status(pool, task_id, STATUS_RUNNING, false)
}

pub fn cancel_task(pool: &DbPool, task_id: Uuid) -> Result<BrowserTask, TaskError> {
    let t = load_task(pool, task_id)?;
    if terminal(&t.status) {
        return Err(TaskError::InvalidState("task already terminal".to_string()));
    }
    set_status(pool, task_id, STATUS_CANCELLED, false)
}

pub fn finish_task(
    pool: &DbPool,
    task_id: Uuid,
    result: serde_json::Value,
    citations: Option<serde_json::Value>,
) -> Result<BrowserTask, TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let t = load_task(pool, task_id)?;
    if terminal(&t.status) {
        return Err(TaskError::InvalidState("task already terminal".to_string()));
    }
    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    diesel::update(bt::browser_tasks.find(task_id))
        .set((
            bt::status.eq(STATUS_FINISHED.to_string()),
            bt::result.eq(Some(result)),
            bt::citations.eq(citations),
            bt::finished_at.eq(diesel::dsl::now),
        ))
        .get_result::<BrowserTask>(&mut conn)
        .map_err(|e| TaskError::Db(e.to_string()))
}

fn fail_task(pool: &DbPool, task_id: Uuid, reason: &str) -> Result<(), TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    diesel::update(bt::browser_tasks.find(task_id))
        .set((
            bt::status.eq(STATUS_FAILED.to_string()),
            bt::error.eq(Some(reason.to_string())),
            bt::finished_at.eq(diesel::dsl::now),
        ))
        .execute(&mut conn)
        .map_err(|e| TaskError::Db(e.to_string()))?;
    Ok(())
}

fn persist_progress(
    pool: &DbPool,
    task_id: Uuid,
    progress: &ProgressDoc,
) -> Result<(), TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    diesel::update(bt::browser_tasks.find(task_id))
        .set(bt::progress.eq(Some(progress.to_value())))
        .execute(&mut conn)
        .map_err(|e| TaskError::Db(e.to_string()))?;
    Ok(())
}

/// Records one driver-reported step after validating policy and budget.
///
/// Async entry point required by the driver contract: validates policy and
/// budget BEFORE recording, then delegates to the blocking implementation.
/// The external execution driver service calls this per completed step.
pub async fn advance_task(
    pool: &DbPool,
    task_id: Uuid,
    step: StepResult,
) -> Result<(BrowserTask, ProgressStep), TaskError> {
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || record_step(&pool, task_id, step))
        .await
        .map_err(|e| TaskError::Db(format!("driver step join failure: {e}")))?
}

/// Records one driver-reported step after validating policy and budget.
///
/// Order of operations: load task → reject non-running state → resolve policy
/// (per-task `plan.policy`, else defaults) → validate URL + credential guard →
/// charge budget rehydrated from the persisted progress → append the step to
/// the progress document → upsert page facts on success. A policy or budget
/// violation fails the task and is returned as `Err` carrying the reason.
fn record_step(
    pool: &DbPool,
    task_id: Uuid,
    step: StepResult,
) -> Result<(BrowserTask, ProgressStep), TaskError> {
    let t = load_task(pool, task_id)?;
    if t.status != STATUS_RUNNING {
        return Err(TaskError::InvalidState(format!(
            "task not running (status '{}')",
            t.status
        )));
    }

    let cfg = PolicyConfig::from_plan(t.plan.as_ref());
    if let Err(reason) = policy::step_allowed(&cfg, &step.url, kind_of(t.plan.as_ref())) {
        fail_task(pool, task_id, &reason)?;
        return Err(TaskError::Policy(reason));
    }

    let mut progress = ProgressDoc::parse(t.progress.as_ref());
    let steps_used = u32::try_from(progress.steps.len())
        .map_err(|_| TaskError::InvalidState("progress step count overflow".to_string()))?;
    let tracker = BudgetTracker::with_usage(
        crate::budget::BudgetCaps::from_policy(&cfg),
        steps_used,
        progress.cost_milli,
    );
    if let Err(e) = tracker.charge_step() {
        let reason = e.to_string();
        fail_task(pool, task_id, &reason)?;
        return Err(TaskError::Budget(e));
    }
    if step.cost_milli > 0 {
        if let Err(e) = tracker.charge_cost(step.cost_milli) {
            let reason = e.to_string();
            fail_task(pool, task_id, &reason)?;
            return Err(TaskError::Budget(e));
        }
        progress.cost_milli += step.cost_milli;
    }

    let recorded = ProgressStep {
        n: progress.steps.len() as i32 + 1,
        action: step.action.clone(),
        url: step.url.clone(),
        ok: step.ok,
        note: step.note.clone(),
        ts: Utc::now(),
    };
    progress.steps.push(recorded.clone());
    persist_progress(pool, task_id, &progress)?;

    if step.ok {
        if let Some(facts) = step.facts.clone() {
            memory::upsert_page_fact(pool, t.user_id, step.url, step.title, facts)
                .map_err(TaskError::Memory)?;
        }
    }

    let updated = load_task(pool, task_id)?;
    Ok((updated, recorded))
}

fn kind_of(plan: Option<&serde_json::Value>) -> &'static str {
    match plan.and_then(|p| p.get("kind")).and_then(|k| k.as_str()) {
        Some(policy::TASK_KIND_CHECKOUT) => policy::TASK_KIND_CHECKOUT,
        _ => policy::TASK_KIND_DEFAULT,
    }
}

/// Lists recent tasks of a user, newest first.
pub fn list_user_tasks(
    pool: &DbPool,
    user_id: Uuid,
    limit: i64,
) -> Result<Vec<BrowserTask>, TaskError> {
    use crate::schema::browser_tasks::dsl as bt;

    let mut conn = pool.get().map_err(|e| TaskError::Db(e.to_string()))?;
    bt::browser_tasks
        .filter(bt::user_id.eq(user_id))
        .order(bt::created_at.desc())
        .limit(limit.clamp(1, 200))
        .load::<BrowserTask>(&mut conn)
        .map_err(|e| TaskError::Db(e.to_string()))
}
