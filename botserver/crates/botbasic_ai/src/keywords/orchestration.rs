use botschema::workflow_executions;
use rhai::{Dynamic, Engine};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug)]
pub enum WorkflowError {
    Database(String),
    NotFound(String),
    Serialization(String),
    Execution(String),
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkflowError::Database(msg) => write!(f, "Database error: {msg}"),
            WorkflowError::NotFound(msg) => write!(f, "Not found: {msg}"),
            WorkflowError::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            WorkflowError::Execution(msg) => write!(f, "Execution error: {msg}"),
        }
    }
}

impl std::error::Error for WorkflowError {}

impl From<diesel::result::Error> for WorkflowError {
    fn from(e: diesel::result::Error) -> Self {
        WorkflowError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for WorkflowError {
    fn from(e: serde_json::Error) -> Self {
        WorkflowError::Serialization(e.to_string())
    }
}

impl From<uuid::Error> for WorkflowError {
    fn from(e: uuid::Error) -> Self {
        WorkflowError::Serialization(e.to_string())
    }
}

impl From<diesel::r2d2::PoolError> for WorkflowError {
    fn from(e: diesel::r2d2::PoolError) -> Self {
        WorkflowError::Database(e.to_string())
    }
}

impl From<rhai::EvalAltResult> for WorkflowError {
    fn from(e: rhai::EvalAltResult) -> Self {
        WorkflowError::Execution(e.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowState {
    #[serde(default)]
    pub current_step: i32,
    #[serde(default)]
    pub variables: std::collections::HashMap<String, String>,
    #[serde(default = "default_running")]
    pub status: String,
    #[serde(default)]
    pub parallel_branches: Vec<String>,
    #[serde(default)]
    pub parallel_results: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub on_error_strategy: String,
}

fn default_running() -> String {
    "running".to_string()
}

pub fn register_orchestrate_workflow(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine.register_custom_syntax(
        ["ORCHESTRATE", "WORKFLOW", "$string$"],
        false,
        move |context, inputs| {
            let workflow_name = context.eval_expression_tree(&inputs[0])?.to_string();
            let state_for_spawn = Arc::clone(&state_clone);
            let user_clone_spawn = user_clone.clone();

            tokio::spawn(async move {
                if let Err(e) = create_workflow(&state_for_spawn, &user_clone_spawn, &workflow_name).await {
                    log::error!("Failed to create workflow {workflow_name}: {e}");
                }
            });

            Ok(Dynamic::UNIT)
        },
    ) {
        log::warn!("Failed to register ORCHESTRATE WORKFLOW syntax: {e}");
    }
}

async fn create_workflow(
    state: &Arc<dyn BasicRuntime>,
    user: &UserSession,
    workflow_name: &str,
) -> Result<Uuid, WorkflowError> {
    let mut conn = state.db_pool().get()
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    let bot_uuid = Uuid::parse_str(&user.bot_id.to_string())?;

    let initial_state = WorkflowState {
        current_step: 1,
        variables: std::collections::HashMap::new(),
        status: "running".to_string(),
        parallel_branches: Vec::new(),
        parallel_results: std::collections::HashMap::new(),
        on_error_strategy: "abort".to_string(),
    };

    let state_json = serde_json::to_string(&initial_state)?;
    let workflow_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let branch_id: Uuid = botschema::bots::table
        .filter(botschema::bots::id.eq(bot_uuid))
        .select(botschema::bots::branch_id)
        .first(&mut conn)?;

    diesel::sql_query(
        "INSERT INTO workflow_executions (id, bot_id, workflow_name, current_step, state_json, status, created_at, updated_at, branch_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
        .bind::<diesel::sql_types::Uuid, _>(&workflow_id)
        .bind::<diesel::sql_types::Uuid, _>(&bot_uuid)
        .bind::<diesel::sql_types::Text, _>(workflow_name)
        .bind::<diesel::sql_types::Int4, _>(1i32)
        .bind::<diesel::sql_types::Text, _>(&state_json)
        .bind::<diesel::sql_types::Text, _>("running")
        .bind::<diesel::sql_types::Timestamptz, _>(&now)
        .bind::<diesel::sql_types::Timestamptz, _>(&now)
        .bind::<diesel::sql_types::Uuid, _>(&branch_id)
        .execute(&mut conn)
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    Ok(workflow_id)
}

pub fn register_step_keyword(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    register_step_basic(state.clone(), user.clone(), engine);
    register_step_parallel(state.clone(), user.clone(), engine);
    register_step_on_error(state.clone(), user.clone(), engine);
}

fn register_step_basic(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine.register_custom_syntax(
        ["STEP", "$int$", ":", "BOT", "$string$", "$string$"],
        false,
        move |context, inputs| {
            let step_number = context.eval_expression_tree(&inputs[0])?.as_int()?;
            let bot_name = context.eval_expression_tree(&inputs[1])?.to_string();
            let action = context.eval_expression_tree(&inputs[2])?.to_string();

            let state_for_spawn = Arc::clone(&state_clone);
            let user_clone_spawn = user_clone.clone();

            tokio::spawn(async move {
                if let Err(e) = execute_workflow_step(
                    &state_for_spawn,
                    &user_clone_spawn,
                    step_number as i32,
                    &bot_name,
                    &action,
                    None,
                    None,
                ).await {
                    log::error!("Failed to execute workflow step {step_number}: {e}");
                }
            });

            Ok(Dynamic::UNIT)
        },
    ) {
        log::warn!("Failed to register STEP syntax: {e}");
    }
}

fn register_step_parallel(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine.register_custom_syntax(
        ["STEP", "$int$", ":", "BOT", "$string$", "$string$", "PARALLEL"],
        false,
        move |context, inputs| {
            let step_number = context.eval_expression_tree(&inputs[0])?.as_int()?;
            let bot_name = context.eval_expression_tree(&inputs[1])?.to_string();
            let action = context.eval_expression_tree(&inputs[2])?.to_string();

            let state_for_spawn = Arc::clone(&state_clone);
            let user_clone_spawn = user_clone.clone();

            tokio::spawn(async move {
                if let Err(e) = execute_workflow_step(
                    &state_for_spawn,
                    &user_clone_spawn,
                    step_number as i32,
                    &bot_name,
                    &action,
                    Some(true),
                    None,
                ).await {
                    log::error!("Failed to execute parallel workflow step {step_number}: {e}");
                }
            });

            Ok(Dynamic::UNIT)
        },
    ) {
        log::warn!("Failed to register PARALLEL STEP syntax: {e}");
    }
}

fn register_step_on_error(state: Arc<dyn BasicRuntime>, user: UserSession, engine: &mut Engine) {
    let state_clone = Arc::clone(&state);
    let user_clone = user;

    if let Err(e) = engine.register_custom_syntax(
        ["STEP", "$int$", ":", "BOT", "$string$", "$string$", "ON", "ERROR", "$string$"],
        false,
        move |context, inputs| {
            let step_number = context.eval_expression_tree(&inputs[0])?.as_int()?;
            let bot_name = context.eval_expression_tree(&inputs[1])?.to_string();
            let action = context.eval_expression_tree(&inputs[2])?.to_string();
            let error_strategy = context.eval_expression_tree(&inputs[3])?.to_string();

            let allowed_strategies = ["retry", "skip", "abort"];
            if !allowed_strategies.contains(&error_strategy.as_str()) {
                return Err(format!("Invalid ON ERROR strategy: {error_strategy}. Must be: retry, skip, or abort").into());
            }

            let state_for_spawn = Arc::clone(&state_clone);
            let user_clone_spawn = user_clone.clone();

            tokio::spawn(async move {
                if let Err(e) = execute_workflow_step(
                    &state_for_spawn,
                    &user_clone_spawn,
                    step_number as i32,
                    &bot_name,
                    &action,
                    None,
                    Some(error_strategy),
                ).await {
                    log::error!("Failed to execute workflow step {step_number}: {e}");
                }
            });

            Ok(Dynamic::UNIT)
        },
    ) {
        log::warn!("Failed to register ON ERROR STEP syntax: {e}");
    }
}

async fn execute_workflow_step(
    state: &Arc<dyn BasicRuntime>,
    user: &UserSession,
    step_number: i32,
    bot_name: &str,
    action: &str,
    parallel: Option<bool>,
    on_error: Option<String>,
) -> Result<(), WorkflowError> {
    let mut conn = state.db_pool().get()
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    let bot_uuid = Uuid::parse_str(&user.bot_id.to_string())?;

    #[derive(QueryableByName)]
    struct WorkflowRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        state_json: Option<String>,
    }

    let workflow = diesel::sql_query(
        "SELECT id, state_json FROM workflow_executions WHERE bot_id = $1 AND status = 'running' ORDER BY created_at DESC LIMIT 1"
    )
        .bind::<diesel::sql_types::Uuid, _>(bot_uuid)
        .get_result::<WorkflowRow>(&mut conn)
        .optional()
        .map_err(|e| WorkflowError::Database(e.to_string()))?
        .ok_or_else(|| WorkflowError::NotFound("No running workflow found".to_string()))?;

    let workflow_state: WorkflowState = match &workflow.state_json {
        Some(v) => serde_json::from_str(v)
            .map_err(|e| WorkflowError::Serialization(e.to_string()))?,
        None => WorkflowState::default(),
    };

    let error_strategy = on_error.unwrap_or_else(|| workflow_state.on_error_strategy.clone());

    if workflow_state.current_step == step_number || parallel.is_some() {
        let mut new_state = workflow_state.clone();
        new_state.current_step = step_number + 1;
        new_state.variables.insert("last_bot".to_string(), bot_name.to_string());
        new_state.variables.insert("last_action".to_string(), action.to_string());

        if parallel.is_some() {
            new_state.parallel_branches.push(format!("{bot_name}:{action}"));
        }

        if let Err(e) = save_workflow_state(workflow.id, &new_state, &mut conn) {
            match error_strategy.as_str() {
                "retry" => {
                    log::warn!("Workflow step {step_number} failed, retrying: {e}");
                    save_workflow_state(workflow.id, &new_state, &mut conn)?;
                }
                "skip" => {
                    log::warn!("Workflow step {step_number} failed, skipping: {e}");
                    new_state.status = "running".to_string();
                    save_workflow_state(workflow.id, &new_state, &mut conn)?;
                }
                _ => {
                    log::error!("Workflow step {step_number} failed, aborting: {e}");
                    new_state.status = "aborted".to_string();
                    save_workflow_state(workflow.id, &new_state, &mut conn)?;
                    return Err(e);
                }
            }
        }
    }

    Ok(())
}

fn save_workflow_state(
    workflow_id: Uuid,
    state: &WorkflowState,
    conn: &mut PgConnection,
) -> Result<(), WorkflowError> {
    let state_json = serde_json::to_string(state)
        .map_err(|e| WorkflowError::Serialization(e.to_string()))?;

    diesel::update(workflow_executions::table.filter(workflow_executions::id.eq(workflow_id)))
        .set((
            workflow_executions::state_json.eq(state_json),
            workflow_executions::current_step.eq(state.current_step),
            workflow_executions::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(conn)
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    Ok(())
}

pub async fn resume_workflows_on_startup(
    state: Arc<dyn BasicRuntime>,
) -> Result<(), WorkflowError> {
    let mut conn = state.db_pool().get()
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    #[derive(QueryableByName)]
    struct WorkflowRowList {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        state_json: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        workflow_name: Option<String>,
    }

    let running_workflows: Vec<WorkflowRowList> = diesel::sql_query(
        "SELECT id, state_json, workflow_name FROM workflow_executions WHERE status = 'running'"
    )
        .load(&mut conn)
        .map_err(|e| WorkflowError::Database(e.to_string()))?;

    for workflow in running_workflows {
        let workflow_state: WorkflowState = match &workflow.state_json {
            Some(v) => serde_json::from_str(v)
                .map_err(|e| WorkflowError::Serialization(e.to_string()))?,
            None => {
                log::warn!("Workflow {} has no state JSON, skipping", workflow.id);
                continue;
            }
        };

        log::info!("Resuming workflow {} ({}) at step {}",
            workflow.id,
            workflow.workflow_name.as_deref().unwrap_or("unknown"),
            workflow_state.current_step
        );

        let state_clone = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = execute_resumed_workflow(workflow.id, workflow_state, state_clone).await {
                log::error!("Failed to resume workflow {}: {e}", workflow.id);
            }
        });
    }

    Ok(())
}

async fn execute_resumed_workflow(
    workflow_id: Uuid,
    _state: WorkflowState,
    _app_state: Arc<dyn BasicRuntime>,
) -> Result<(), WorkflowError> {
    log::info!("Executing resumed workflow {workflow_id}");
    Ok(())
}

use botbasic_types::BasicRuntime;
use botbasic_types::UserSession;
use diesel::prelude::*;
use diesel::PgConnection;
use std::sync::Arc;
use uuid::Uuid;
