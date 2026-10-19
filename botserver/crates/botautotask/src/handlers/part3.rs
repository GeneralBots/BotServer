use super::*;

pub(super) fn load_plan(api: &AutoTaskApi, plan_id: Uuid) -> Result<Option<PlanRow>, String> {
    let mut conn = api.state().db_pool().get().map_err(|e| format!("db pool: {e}"))?;
    sql_query(
        "SELECT id, bot_id, original_text, intent_type, confidence, suggested_name \
         FROM intent_classifications WHERE id = $1",
    )
    .bind::<DieselUuid, _>(plan_id)
    .get_result::<PlanRow>(&mut conn)
    .optional()
    .map_err(|e| format!("load plan: {e}"))
}
/// Execute a previously classified plan for real.
///
/// The plan id identifies a persisted `intent_classifications` row; the row is
/// rebuilt into a `ClassifiedIntent`, the offline pipeline regenerates the
/// BASIC, and the script is uploaded to the bot's Drive `gbdialog` folder —
/// exactly the path `create_and_execute` takes, so DriveMonitor compiles and
/// runs the automation. The response reports the real outcome; it never
/// fabricates a `scheduled` status with no side effect.
pub async fn execute_plan(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<ExecutePlanRequest>,
) -> impl IntoResponse {
    info!(
        "API execute plan: {} (mode: {:?}, priority: {:?})",
        req.plan_id, req.execution_mode, req.priority
    );
    let plan_uuid = match Uuid::parse_str(&req.plan_id) {
        Ok(u) => u,
        Err(_) => {
            return Json(ExecutePlanResponse {
                success: false,
                task_id: None,
                status: Some("invalid".to_string()),
                error: Some("Invalid plan id".to_string()),
            });
        }
    };

    let row = match load_plan(&api, plan_uuid) {
        Ok(Some(row)) => row,
        Ok(None) => {
            return Json(ExecutePlanResponse {
                success: false,
                task_id: Some(req.plan_id.clone()),
                status: Some("not_found".to_string()),
                error: Some("Plan not found".to_string()),
            });
        }
        Err(e) => {
            warn!("execute_plan lookup failed for {}: {e}", req.plan_id);
            return Json(ExecutePlanResponse {
                success: false,
                task_id: Some(req.plan_id.clone()),
                status: Some("failed".to_string()),
                error: Some(e),
            });
        }
    };

    let plan_id = row.id.to_string();
    let classification = ClassifiedIntent {
        id: plan_id.clone(),
        original_text: row.original_text,
        intent_type: IntentType::from(row.intent_type.as_str()),
        confidence: row.confidence,
        entities: ClassifiedEntities::default(),
        suggested_name: row.suggested_name,
        requires_clarification: false,
        clarification_question: None,
        alternative_types: Vec::new(),
        classified_at: chrono::Utc::now(),
    };
    let (relative_path, body) = script_for(&classification, None);
    match persist_script(&api, row.bot_id, &relative_path, &body) {
        Ok((bucket, key)) => {
            info!("execute_plan persisted automation to {bucket}/{key}");
            Json(ExecutePlanResponse {
                success: true,
                task_id: Some(plan_id),
                status: Some("created".to_string()),
                error: None,
            })
        }
        Err(e) => {
            warn!("execute_plan persist failed: {e}");
            Json(ExecutePlanResponse {
                success: false,
                task_id: Some(plan_id),
                status: Some("failed".to_string()),
                error: Some(e),
            })
        }
    }
}
/// BASIC-only pipeline: classify → compile → persist `.bas` to the bot's
/// Drive bucket. DriveMonitor picks the file up, DriveCompiler registers the
/// automation (basic_tools + system_automations), auto_service runs it.
pub async fn create_and_execute(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<CreateAndExecuteRequest>,
) -> Json<CreateAndExecuteResponse> {
    let bot_id = canonical_bot_id(req.bot_id.clone());
    Json(create_and_execute_for(&api, &req.intent, bot_id).await)
}
/// The whole create-and-execute flow, callable from outside the HTTP layer.
///
/// The chat command executor needs the same pipeline: `tasks.autotask.create`
/// was advertised in the catalog but had no execution arm, so a user asking
/// for an automation in chat received a deep link and nothing else.
pub async fn create_and_execute_for(
    api: &Arc<AutoTaskApi>,
    intent: &str,
    bot_id: Uuid,
) -> CreateAndExecuteResponse {
    info!("API create and execute: {}", botlib::text::truncate_at_char_boundary(intent, 50));
    let intent = intent.to_string();
    let req = CreateAndExecuteRequest { intent: intent.clone(), bot_id: None };

    // Shipped-template fast path: intents that name an implementation the
    // product already ships (e.g. media classification/filing) persist the
    // vetted template verbatim — deterministic, no LLM, no compile stall.
    if let Some(template) = match_shipped_template(&req.intent) {
        return persist_shipped_template(api, bot_id, &req.intent, template).await.0;
    }

    let classification = match classifier_for(api).classify_api(&req.intent, bot_id).await {
        Ok(c) => c,
        Err(e) => return error_create(&req.intent, &*e).0,
    };
    let mut compiler = compiler_for(api);
    let compiled = match compiler
        .compile_from_classification(bot_id, &classification, None, None)
        .await
    {
        Ok(c) => c,
        Err(e) => return error_create(&req.intent, &*e).0,
    };
    let (relative_path, body) = script_for(&classification, Some(&compiled));
    match persist_script(api, bot_id, &relative_path, &body) {
        Ok((bucket, key)) => CreateAndExecuteResponse {
            success: true,
            task_id: classification.id.clone(),
            status: "created".to_string(),
            message: format!("Automation created and registered: {bucket}/{key}"),
            app_url: None,
            created_resources: vec![crate::api::CreatedResourceResponse {
                resource_type: classification.intent_type.to_string().to_lowercase(),
                name: classification
                    .suggested_name
                    .clone()
                    .unwrap_or_else(|| "autotask".to_string()),
                path: Some(key.clone()),
            }],
            pending_items: Vec::new(),
            error: None,
        },
        Err(e) => CreateAndExecuteResponse {
            success: false,
            task_id: classification.id.clone(),
            status: "failed".to_string(),
            message: e.clone(),
            app_url: None,
            created_resources: Vec::new(),
            pending_items: Vec::new(),
            error: Some(e),
        },
    }
}
/// Upload the generated `.bas` to `{bot}.gbai/{bot}.gbdialog/{path}` via the
/// AutoTask Drive facade (no local filesystem writes).
pub(super) fn persist_script(
    api: &AutoTaskApi,
    bot_id: Uuid,
    relative_path: &str,
    body: &str,
) -> Result<(String, String), String> {
    let info = resolve_bot_info(api.state().db_pool(), bot_id)?
        .ok_or_else(|| "bot not found for classification".to_string())?;
    let bucket = info.bucket_name();
    // Reform #1505 — a git-owned bot's `.gbdialog` is its repository: commit the
    // generated tool there so the pull monitor compiles it and the task leaves a
    // real artifact. Drive is the legacy fallback for bots without a project.
    if let Some(result) = crate::source_persist::persist_to_git(
        api.state().as_ref(),
        &info,
        bot_id,
        relative_path.trim_start_matches('/'),
        body,
        None,
        "autotask: add generated tool",
    ) {
        let persisted = result?;
        info!(
            "Committed generated BASIC to ALM: {}/{}",
            persisted.bucket, persisted.tool_key
        );
        record_auto_task(
            api.state().db_pool(),
            bot_id,
            relative_path.trim_start_matches('/'),
            "generated tool",
            "completed",
            Some(&persisted.tool_key),
        );
        return Ok((persisted.bucket, persisted.tool_key));
    }
    let ops = api
        .state()
        .file_ops()
        .ok_or_else(|| "Drive ops not available — cannot persist generated BASIC".to_string())?;
    let key = format!("{}/{}", info.dialog_folder(), relative_path.trim_start_matches('/'));
    ops.put_object(&bucket, &key, body.as_bytes().to_vec(), "text/plain")
        .map_err(|e| format!("drive put failed: {e}"))?;
    info!("Saved BASIC file to Drive: {bucket}/{key}");
    Ok((bucket, key))
}
