use super::*;

pub(super) fn classifier_for(api: &Arc<AutoTaskApi>) -> IntentClassifier {
    IntentClassifier::new(
        api.state().db_pool().clone(),
        api.config_ops().clone(),
        api.llm_ops().clone(),
        api.state().clone(),
    )
}
pub(super) fn compiler_for(api: &Arc<AutoTaskApi>) -> IntentCompiler {
    IntentCompiler::new(
        api.state().clone(),
        api.config_ops().clone(),
        api.llm_ops().clone(),
    )
}
pub(crate) fn err_msg(context: &str, e: &dyn std::error::Error) -> String {
    let msg = format!("{context} failed: {e}");
    warn!("AutoTask API: {msg}");
    msg
}
pub async fn classify_intent(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<ClassifyIntentRequest>,
) -> impl IntoResponse {
    info!(
        "API classify intent: {}",
        botlib::text::truncate_at_char_boundary(&req.intent, 50)
    );
    let bot_id = canonical_bot_id(req.bot_id.clone());
    let effective_bot_id = resolve_effective_bot_id(api.state().db_pool(), bot_id);
    match classifier_for(&api).classify_api(&req.intent, bot_id).await {
        Ok(c) => {
            let result = if req.auto_process == Some(true) {
                auto_process_classification(&api, effective_bot_id, &c).await
            } else {
                None
            };
            Json(ClassifyIntentResponse {
                success: true,
                intent_type: c.intent_type.to_string(),
                confidence: c.confidence,
                suggested_name: c.suggested_name.clone(),
                requires_clarification: c.requires_clarification,
                clarification_question: c.clarification_question.clone(),
                result,
                error: None,
            })
        }
        Err(e) => Json(ClassifyIntentResponse {
            success: false,
            intent_type: "UNKNOWN".to_string(),
            confidence: 0.0,
            suggested_name: None,
            requires_clarification: true,
            clarification_question: None,
            result: None,
            error: Some(err_msg("classify", &*e)),
        }),
    }
}
/// Auto-process pipeline for chat-driven intents (vibe chat): classify →
/// compile → persist the generated `.bas` to the bot's Drive bucket so
/// DriveMonitor registers the automation. Returns the result payload the
/// frontend renders as task nodes and progress messages.
pub(super) async fn auto_process_classification(
    api: &Arc<AutoTaskApi>,
    bot_id: Uuid,
    classification: &ClassifiedIntent,
) -> Option<crate::api::IntentResultResponse> {
    let compiled = match compiler_for(api)
        .compile_from_classification(bot_id, classification, None, None)
        .await
    {
        Ok(c) => Some(c),
        Err(e) => {
            warn!("auto_process LLM compile failed ({e}); using offline BASIC fallback");
            None
        }
    };
    let (relative_path, body) = script_for(classification, compiled.as_ref());
    match persist_script(api, bot_id, &relative_path, &body) {
        Ok((bucket, key)) => Some(crate::api::IntentResultResponse {
            success: true,
            message: format!("Automation created and registered: {bucket}/{key}"),
            app_url: None,
            task_id: Some(classification.id.clone()),
            schedule_id: None,
            tool_triggers: Vec::new(),
            created_resources: vec![crate::api::CreatedResourceResponse {
                resource_type: classification.intent_type.to_string().to_lowercase(),
                name: classification
                    .suggested_name
                    .clone()
                    .unwrap_or_else(|| "autotask".to_string()),
                path: Some(key.clone()),
            }],
            next_steps: Vec::new(),
        }),
        Err(e) => {
            warn!("auto_process persist failed: {e}");
            Some(crate::api::IntentResultResponse {
                success: false,
                message: e,
                app_url: None,
                task_id: Some(classification.id.clone()),
                schedule_id: None,
                tool_triggers: Vec::new(),
                created_resources: Vec::new(),
                next_steps: Vec::new(),
            })
        }
    }
}
pub async fn compile_intent(
    State(api): State<Arc<AutoTaskApi>>,
    Json(req): Json<CompileIntentRequest>,
) -> Json<CompileIntentResponse> {
    info!(
        "API compile intent: {}",
        botlib::text::truncate_at_char_boundary(&req.intent, 50)
    );
    let bot_id = canonical_bot_id(req.bot_id.clone());
    let classification = match classifier_for(&api).classify_api(&req.intent, bot_id).await {
        Ok(c) => c,
        Err(e) => return error_compile(&req.intent, &*e),
    };
    let fallback_body = script_for(&classification, None).1;
    let compiled = match compiler_for(&api)
        .compile_from_classification(bot_id, &classification, None, None)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            warn!("LLM compile failed ({e}); returning offline BASIC fallback");
            return Json(CompileIntentResponse {
                success: true,
                plan_id: Some(classification.id.clone()),
                plan_name: classification.suggested_name.clone(),
                plan_description: Some(classification.original_text.clone()),
                steps: Vec::new(),
                alternatives: Vec::new(),
                confidence: classification.confidence,
                risk_level: "medium".to_string(),
                estimated_duration_minutes: 0,
                estimated_cost: 0.0,
                resource_estimate: ResourceEstimateResponse {
                    compute_hours: 0.0, storage_gb: 0.0, api_calls: 0, llm_tokens: 0, estimated_cost_usd: 0.0,
                },
                basic_program: Some(fallback_body),
                requires_approval: classification.requires_clarification,
                mcp_servers: Vec::new(),
                external_apis: Vec::new(),
                risks: Vec::new(),
                error: None,
            })
        }
    };
    let basic_program = compiled
        .basic_program
        .clone()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or(fallback_body);
    Json(CompileIntentResponse {
        success: true,
        plan_id: Some(compiled.id.clone()),
        plan_name: Some(compiled.plan_name.clone()),
        plan_description: Some(compiled.plan_description.clone()),
        steps: compiled
            .steps
            .iter()
            .map(|s| PlanStepResponse {
                id: s.id.clone(),
                order: s.order,
                name: s.name.clone(),
                description: s.description.clone(),
                keywords: s.keywords.clone(),
                priority: s.priority.clone(),
                risk_level: s.risk_level.clone(),
                estimated_minutes: s.estimated_minutes,
                requires_approval: s.requires_approval,
            })
            .collect(),
        alternatives: Vec::new(),
        confidence: compiled.confidence,
        risk_level: compiled.risk_level.clone(),
        estimated_duration_minutes: compiled.estimated_duration_minutes,
        estimated_cost: compiled.estimated_cost,
        resource_estimate: ResourceEstimateResponse {
            compute_hours: compiled.resource_estimate.compute_hours,
            storage_gb: compiled.resource_estimate.storage_gb,
            api_calls: compiled.resource_estimate.api_calls,
            llm_tokens: compiled.resource_estimate.llm_tokens,
            estimated_cost_usd: compiled.resource_estimate.estimated_cost_usd,
        },
        basic_program: Some(basic_program),
        requires_approval: compiled.requires_approval,
        mcp_servers: compiled.mcp_servers.clone(),
        external_apis: compiled.external_apis.clone(),
        risks: compiled
            .risks
            .iter()
            .map(|r| RiskResponse {
                id: r.id.clone(),
                category: r.category.clone(),
                description: r.description.clone(),
                probability: r.probability,
                impact: r.impact.clone(),
            })
            .collect(),
        error: None,
    })
}
pub(super) fn error_compile(intent: &str, e: &dyn std::error::Error) -> Json<CompileIntentResponse> {
    let msg = err_msg("compile", e);
    warn!("compile failed for intent: {intent}");
    Json(CompileIntentResponse {
        success: false,
        plan_id: None,
        plan_name: None,
        plan_description: None,
        steps: Vec::new(),
        alternatives: Vec::new(),
        confidence: 0.0,
        risk_level: "unknown".to_string(),
        estimated_duration_minutes: 0,
        estimated_cost: 0.0,
        resource_estimate: ResourceEstimateResponse {
            compute_hours: 0.0, storage_gb: 0.0, api_calls: 0, llm_tokens: 0, estimated_cost_usd: 0.0,
        },
        basic_program: None,
        requires_approval: false,
        mcp_servers: Vec::new(),
        external_apis: Vec::new(),
        risks: Vec::new(),
        error: Some(msg),
    })
}
