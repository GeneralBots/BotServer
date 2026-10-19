//! Extended workflow DAG primitives (IF/BRANCH/PARALLEL/MERGE/ON ERROR) that
//! complement the basic STEP keyword registered in `orchestration.rs`.
//!
//! These keywords write structured markers into the workflow state JSON so
//! the runtime can rebuild the DAG. The actual DAG executor lives in
//! `botorchestration` (or the workflow runner); this module is the
//! declarative BASIC-side.

use botbasic_types::{BasicRuntime, UserSession};
use botschema::workflow_executions;
use diesel::prelude::*;
use diesel::PgConnection;
use rhai::{Dynamic, Engine, EvalAltResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug)]
pub enum DagError {
    Database(String),
    NotFound(String),
    Serialization(String),
    Invalid(String),
}

impl fmt::Display for DagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DagError::Database(m) => write!(f, "database: {m}"),
            DagError::NotFound(m) => write!(f, "not found: {m}"),
            DagError::Serialization(m) => write!(f, "serialization: {m}"),
            DagError::Invalid(m) => write!(f, "invalid: {m}"),
        }
    }
}

impl std::error::Error for DagError {}

impl From<diesel::result::Error> for DagError {
    fn from(e: diesel::result::Error) -> Self {
        DagError::Database(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for DagError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        DagError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for DagError {
    fn from(e: serde_json::Error) -> Self {
        DagError::Serialization(e.to_string())
    }
}

impl From<uuid::Error> for DagError {
    fn from(e: uuid::Error) -> Self {
        DagError::Serialization(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    #[default]
    Step,
    Branch,
    Parallel,
    Merge,
    ErrorHandler,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DagNode {
    pub id: Uuid,
    pub kind: NodeKind,
    pub label: String,
    pub condition: Option<String>,
    pub branches: Vec<String>,
    pub handler: Option<String>,
    pub depends_on: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DagState {
    pub nodes: HashMap<String, DagNode>,
    pub status: String,
    pub next_step: i32,
    pub current_branch: Option<String>,
    pub completed: Vec<String>,
    pub failed: Vec<String>,
}

pub fn register_workflow_dag_keywords(
    state: Arc<dyn BasicRuntime>,
    user: UserSession,
    engine: &mut Engine,
) {
    register_branch_keyword(state.clone(), user.clone(), engine);
    register_parallel_start(state.clone(), user.clone(), engine);
    register_merge_keyword(state.clone(), user.clone(), engine);
    register_error_handler(state, user, engine);
}

fn runtime_error(message: impl Into<String>) -> Box<EvalAltResult> {
    Box::new(EvalAltResult::ErrorRuntime(message.into().into(), rhai::Position::NONE))
}

fn eval_string(context: &mut rhai::EvalContext, input: &rhai::Expression) -> Result<String, Box<EvalAltResult>> {
    Ok(context.eval_expression_tree(input)?.to_string())
}

/// `IF "condition" THEN "label"`: declares a branch in the workflow DAG. The
/// condition is stored as a free-form string and evaluated at runtime.
fn register_branch_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;
    let registered = engine
        .register_custom_syntax(
            ["IF", "$expr$", "THEN", "$expr$"],
            false,
            move |context, inputs| {
                let condition = eval_string(context, &inputs[0])?;
                let label = eval_string(context, &inputs[1])?;
                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = std::thread::Builder::new()
                        .name("branch-dag".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                                    format!("runtime: {e}").into()
                                })?;
                            rt.block_on(upsert_dag_node(
                                &state_for_task,
                                &user_for_task,
                                NodeKind::Branch,
                                &label,
                                Some(&condition),
                                None,
                                None,
                            ))
                        });
                    let outcome = match result {
                        Ok(handle) => match handle.join() {
                            Ok(res) => res,
                            Err(_) => Err("branch thread panicked".into()),
                        },
                        Err(e) => Err(format!("spawn: {e}").into()),
                    };
                    let _ = tx.send(outcome);
                });
                match rx.recv_timeout(std::time::Duration::from_secs(15)) {
                    Ok(Ok(())) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => Err(runtime_error(e.to_string())),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(runtime_error("branch timeout")),
                    Err(e) => Err(runtime_error(format!("branch failed: {e}"))),
                }
            },
        );
    if let Err(e) = registered {
        log::error!("Failed to register IF/THEN DAG syntax: {e}");
    }
}

/// `PARALLEL "name" WITH "label1" AND "label2"`: declares a parallel section
/// that fans out to multiple branches.
fn register_parallel_start(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;
    let registered = engine
        .register_custom_syntax(
            ["PARALLEL", "$expr$", "WITH", "$expr$", "AND", "$expr$"],
            false,
            move |context, inputs| {
                let name = eval_string(context, &inputs[0])?;
                let first = eval_string(context, &inputs[1])?;
                let second = eval_string(context, &inputs[2])?;
                let branches = vec![first, second];
                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = std::thread::Builder::new()
                        .name("parallel-dag".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                                    format!("runtime: {e}").into()
                                })?;
                            rt.block_on(upsert_dag_node(
                                &state_for_task,
                                &user_for_task,
                                NodeKind::Parallel,
                                &name,
                                None,
                                Some(branches),
                                None,
                            ))
                        });
                    let outcome = match result {
                        Ok(handle) => match handle.join() {
                            Ok(res) => res,
                            Err(_) => Err("parallel thread panicked".into()),
                        },
                        Err(e) => Err(format!("spawn: {e}").into()),
                    };
                    let _ = tx.send(outcome);
                });
                match rx.recv_timeout(std::time::Duration::from_secs(15)) {
                    Ok(Ok(())) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => Err(runtime_error(e.to_string())),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(runtime_error("parallel timeout")),
                    Err(e) => Err(runtime_error(format!("parallel failed: {e}"))),
                }
            },
        );
    if let Err(e) = registered {
        log::error!("Failed to register PARALLEL WITH AND syntax: {e}");
    }
}

/// `MERGE "name"`: marks a join point that waits for all branches declared
/// in the parallel section named `name` to complete.
fn register_merge_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;
    let registered = engine
        .register_custom_syntax(
            ["MERGE", "$expr$"],
            false,
            move |context, inputs| {
                let name = eval_string(context, &inputs[0])?;
                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = std::thread::Builder::new()
                        .name("merge-dag".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                                    format!("runtime: {e}").into()
                                })?;
                            rt.block_on(upsert_dag_node(
                                &state_for_task,
                                &user_for_task,
                                NodeKind::Merge,
                                &name,
                                None,
                                None,
                                None,
                            ))
                        });
                    let outcome = match result {
                        Ok(handle) => match handle.join() {
                            Ok(res) => res,
                            Err(_) => Err("merge thread panicked".into()),
                        },
                        Err(e) => Err(format!("spawn: {e}").into()),
                    };
                    let _ = tx.send(outcome);
                });
                match rx.recv_timeout(std::time::Duration::from_secs(15)) {
                    Ok(Ok(())) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => Err(runtime_error(e.to_string())),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(runtime_error("merge timeout")),
                    Err(e) => Err(runtime_error(format!("merge failed: {e}"))),
                }
            },
        );
    if let Err(e) = registered {
        log::error!("Failed to register MERGE syntax: {e}");
    }
}

/// `ON ERROR CALL "handler_tool"`: attaches an error handler to the most
/// recently declared node. The handler label is stored for the runtime to
/// invoke when the previous node fails.
fn register_error_handler(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;
    let registered = engine
        .register_custom_syntax(
            ["ON", "ERROR", "CALL", "$expr$"],
            false,
            move |context, inputs| {
                let handler = eval_string(context, &inputs[0])?;
                let state_for_task = Arc::clone(&state_clone);
                let user_for_task = user_clone.clone();
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = std::thread::Builder::new()
                        .name("on-error-dag".into())
                        .spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                                    format!("runtime: {e}").into()
                                })?;
                            rt.block_on(upsert_dag_node(
                                &state_for_task,
                                &user_for_task,
                                NodeKind::ErrorHandler,
                                &handler,
                                None,
                                None,
                                Some(&handler),
                            ))
                        });
                    let outcome = match result {
                        Ok(handle) => match handle.join() {
                            Ok(res) => res,
                            Err(_) => Err("on error thread panicked".into()),
                        },
                        Err(e) => Err(format!("spawn: {e}").into()),
                    };
                    let _ = tx.send(outcome);
                });
                match rx.recv_timeout(std::time::Duration::from_secs(15)) {
                    Ok(Ok(())) => Ok(Dynamic::UNIT),
                    Ok(Err(e)) => Err(runtime_error(e.to_string())),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(runtime_error("on error timeout")),
                    Err(e) => Err(runtime_error(format!("on error failed: {e}"))),
                }
            },
        );
    if let Err(e) = registered {
        log::error!("Failed to register ON ERROR CALL syntax: {e}");
    }
}

async fn upsert_dag_node(
    state: &Arc<dyn BasicRuntime>,
    user: &UserSession,
    kind: NodeKind,
    label: &str,
    condition: Option<&str>,
    branches: Option<Vec<String>>,
    handler: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = state.db_pool().get()?;
    let bot_uuid = Uuid::parse_str(&user.bot_id.to_string())?;

    let workflow_id: Uuid = {
        #[derive(QueryableByName)]
        struct IdRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
        }
        diesel::sql_query(
            "SELECT id FROM workflow_executions WHERE bot_id = $1 AND status = 'running' ORDER BY created_at DESC LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_uuid)
        .get_result::<IdRow>(&mut conn)
        .optional()?
        .ok_or("no running workflow found")?
        .id
    };

    let state_json: Option<serde_json::Value> = diesel::sql_query(
        "SELECT state_json FROM workflow_executions WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(workflow_id)
    .get_result::<StateJsonRow>(&mut conn)
    .optional()?
    .and_then(|r| r.state_json)
    .and_then(|s| serde_json::from_str(&s).ok());

    let mut dag: DagState = match state_json {
        Some(v) => serde_json::from_value(v)?,
        None => DagState {
            status: "running".into(),
            next_step: 1,
            ..Default::default()
        },
    };

    let node = DagNode {
        id: Uuid::new_v4(),
        kind: kind.clone(),
        label: label.to_string(),
        condition: condition.map(|s| s.to_string()),
        branches: branches.unwrap_or_default(),
        handler: handler.map(|s| s.to_string()),
        depends_on: Vec::new(),
    };
    dag.nodes.insert(label.to_string(), node);
    dag.next_step += 1;
    match kind {
        NodeKind::Parallel => dag.current_branch = Some(label.to_string()),
        NodeKind::Merge => {
            dag.current_branch = None;
            dag.completed.push(label.to_string());
        }
        _ => {}
    }

    save_dag_state(workflow_id, &dag, &mut conn)?;

    Ok(())
}

#[derive(QueryableByName)]
struct StateJsonRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    state_json: Option<String>,
}

fn save_dag_state(
    workflow_id: Uuid,
    dag: &DagState,
    conn: &mut PgConnection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state_json = serde_json::to_string(dag)?;
    diesel::update(workflow_executions::table.filter(workflow_executions::id.eq(workflow_id)))
        .set((
            workflow_executions::state_json.eq(state_json),
            workflow_executions::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)?;
    Ok(())
}


#[cfg(test)]
#[path = "workflow_dag_tests.rs"]
mod workflow_dag_tests;
