use super::*;

impl IntentCompiler {
    pub fn new(
        state: Arc<dyn AutoTaskState>,
        config_ops: Arc<dyn ConfigOps>,
        llm_ops: Arc<dyn LlmProviderOps>,
    ) -> Self {
        let safety = SafetyLayer::new(state.db_pool().clone());
        Self { state, config_ops, llm_ops, safety }
    }

    pub fn state(&self) -> &Arc<dyn AutoTaskState> {
        &self.state
    }

    pub fn config_ops(&self) -> &Arc<dyn ConfigOps> {
        &self.config_ops
    }

    pub fn llm_ops(&self) -> &Arc<dyn LlmProviderOps> {
        &self.llm_ops
    }

    /// `bot_id` selects the model scope: the bot being automated is the one
    /// whose `llm-model` applies, not the nil (global) one.
    pub async fn compile(
        &self,
        bot_id: Uuid,
        intent: &str,
        execution_mode: Option<ExecutionMode>,
        priority: Option<TaskPriority>,
    ) -> Result<CompiledIntent, Box<dyn std::error::Error + Send + Sync>> {
        info!("Compiling intent: {}", botlib::text::truncate_at_char_boundary(intent, 100));
        let prompt = self.build_compile_prompt(intent);
        let response = self.call_llm(bot_id, &prompt).await?;
        self.parse_compile_response(&response, intent, execution_mode, priority)
    }

    pub async fn compile_from_classification(
        &mut self,
        bot_id: Uuid,
        classification: &ClassifiedIntent,
        execution_mode: Option<ExecutionMode>,
        priority: Option<TaskPriority>,
    ) -> Result<CompiledIntent, Box<dyn std::error::Error + Send + Sync>> {
        // Load this bot's constraints up front. `load_constraints` propagates a
        // database error instead of defaulting to an empty list, so a broken
        // or unmigrated `safety_constraints` table fails the compile rather
        // than silently compiling with nothing enforced.
        self.safety.load_constraints(&bot_id)?;

        let intent_text = &classification.original_text;
        let mut compiled = self.compile(bot_id, intent_text, execution_mode, priority).await?;
        compiled.intent_type = classification.intent_type;
        if let Some(ref name) = classification.suggested_name {
            compiled.plan_name = name.clone();
        }
        Ok(compiled)
    }

    pub async fn simulate(
        &self,
        compiled: &CompiledIntent,
        session: &UserSession,
    ) -> Result<SimulationResult, Box<dyn std::error::Error + Send + Sync>> {
        self.safety.simulate_execution(&compiled.id, session)
    }

    pub(super) fn build_compile_prompt(&self, intent: &str) -> String {
        compile_prompt(intent, &self.basic_reference_text())
    }

    /// BASIC reference for the prompt. The host owns the keyword catalog
    /// (`basic::keywords::get_all_keywords`); when it is unavailable the
    /// compiler still asks for a program, and the deterministic fallback
    /// catches a missing one.
    pub(super) fn basic_reference_text(&self) -> String {
        match self.state.basic_reference() {
            Some(reference) if !reference.trim().is_empty() => reference,
            _ => DEGRADED_BASIC_REFERENCE.to_string(),
        }
    }

    #[cfg(feature = "llm")]
    async fn call_llm(
        &self,
        bot_id: Uuid,
        prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let model = self.compile_model(bot_id);
        let key = self.resolved_key(bot_id);
        // Generous budget: a reasoning model spends tokens before the object.
        if model.is_empty() {
            return Err(format!(
                "no llm-model configured for bot {bot_id} (checked the bot scope, then the global one)"
            )
            .into());
        }
        info!(
            "[autotask] compiling with model={model} key_len={} bot={bot_id}",
            key.len()
        );
        let config = serde_json::json!({"temperature": 0.3, "max_tokens": 4000});
        // Bounded — a stalled model must fail the compile step, not hang it.
        crate::types::collect_llm_stream(self.llm_ops.as_ref(), prompt, &config, &model, &key, None)
            .await
    }

    /// Resolves a setting for the bot being automated, then the global one.
    ///
    /// Only the nil bot used to be consulted, and its Vault folder carries no
    /// `llm-model`, so the lookup fell through to a hardcoded `"gpt-4"`: every
    /// AutoTask compile then died with `404 model_not_found` on a gateway that
    /// serves something else. An empty result is now an explicit error instead
    /// of a vendor guess.
    #[cfg(feature = "llm")]
    pub(super) fn setting_for_bot(&self, bot_id: &Uuid, key: &str) -> String {
        for id in [*bot_id, Uuid::nil()] {
            if let Ok(value) = self.config_ops.get_config(&id, key, None) {
                let trimmed = value.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
        String::new()
    }

    /// The model used to write BASIC, with an `autotask-llm-model` override.
    ///
    /// Code generation is a long streaming call: on a reasoning model the
    /// gateway dropped the connection mid-stream ("error sending request"),
    /// which is why the feature gets its own setting and can be pointed at a
    /// fast model without changing the chat model.
    #[cfg(feature = "llm")]
    pub(super) fn compile_model(&self, bot_id: Uuid) -> String {
        for id in [bot_id, Uuid::nil()] {
            if let Ok(value) = self.config_ops.get_config(&id, "autotask-llm-model", None) {
                let trimmed = value.trim().to_string();
                if !trimmed.is_empty() {
                    return trimmed;
                }
            }
        }
        self.resolved_model(bot_id)
    }

    #[cfg(feature = "llm")]
    pub(super) fn resolved_model(&self, bot_id: Uuid) -> String {
        self.setting_for_bot(&bot_id, "llm-model")
    }

    #[cfg(feature = "llm")]
    pub(super) fn resolved_key(&self, bot_id: Uuid) -> String {
        self.setting_for_bot(&bot_id, "llm-key")
    }

    #[cfg(not(feature = "llm"))]
    async fn call_llm(
        &self,
        _bot_id: Uuid,
        _prompt: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        warn!("LLM feature not enabled for intent compilation");
        Ok("{}".to_string())
    }

    pub(super) fn parse_compile_response(
        &self,
        response: &str,
        original_intent: &str,
        execution_mode: Option<ExecutionMode>,
        priority: Option<TaskPriority>,
    ) -> Result<CompiledIntent, Box<dyn std::error::Error + Send + Sync>> {
        let cleaned = response.trim()
            .trim_start_matches("```json").trim_start_matches("```JSON")
            .trim_start_matches("```").trim_end_matches("```").trim();
        let json_span = extract_json_object(cleaned);

        #[derive(Deserialize)]
        struct CompileResponse {
            plan_name: String,
            plan_description: String,
            steps: Vec<StepResponse>,
            alternatives: Vec<AltResponse>,
            confidence: f64,
            risk_level: String,
            estimated_duration_minutes: i32,
            estimated_cost: f64,
            resource_estimate: ResourceEstResponse,
            basic_program: Option<String>,
            requires_approval: bool,
            mcp_servers: Vec<String>,
            external_apis: Vec<String>,
            risks: Vec<RiskResponse>,
        }
        #[derive(Deserialize)]
        struct StepResponse {
            name: String,
            description: String,
            keywords: Option<Vec<String>>,
            priority: Option<String>,
            risk_level: Option<String>,
            estimated_minutes: Option<i32>,
            requires_approval: Option<bool>,
        }
        #[derive(Deserialize)]
        struct AltResponse {
            description: String,
            confidence: f64,
            pros: Option<Vec<String>>,
            cons: Option<Vec<String>>,
            estimated_cost: Option<f64>,
            estimated_time_hours: Option<f64>,
        }
        #[derive(Deserialize)]
        struct ResourceEstResponse {
            compute_hours: f64,
            storage_gb: f64,
            api_calls: i32,
            llm_tokens: i32,
            estimated_cost_usd: f64,
        }
        #[derive(Deserialize)]
        struct RiskResponse {
            category: String,
            description: String,
            probability: f64,
            impact: String,
        }

        match serde_json::from_str::<CompileResponse>(json_span) {
            Ok(resp) => {
                let _ = (execution_mode, priority);
                let steps = resp.steps.into_iter().enumerate().map(|(i, s)| PlanStep {
                    id: Uuid::new_v4().to_string(),
                    order: i as i32 + 1,
                    name: s.name,
                    description: s.description,
                    keywords: s.keywords.unwrap_or_default(),
                    priority: s.priority.unwrap_or_else(|| "medium".to_string()),
                    risk_level: s.risk_level.unwrap_or_else(|| "low".to_string()),
                    estimated_minutes: s.estimated_minutes.unwrap_or(5),
                    requires_approval: s.requires_approval.unwrap_or(false),
                }).collect();
                let alternatives = resp.alternatives.into_iter().map(|a| Alternative {
                    id: Uuid::new_v4().to_string(),
                    description: a.description,
                    confidence: a.confidence,
                    pros: a.pros.unwrap_or_default(),
                    cons: a.cons.unwrap_or_default(),
                    estimated_cost: a.estimated_cost,
                    estimated_time_hours: a.estimated_time_hours,
                }).collect();
                let risks = resp.risks.into_iter().map(|r| Risk {
                    id: Uuid::new_v4().to_string(),
                    category: r.category,
                    description: r.description,
                    probability: r.probability,
                    impact: r.impact,
                }).collect();
                let plan_name = resp.plan_name;
                Ok(CompiledIntent {
                    id: Uuid::new_v4().to_string(),
                    intent_type: IntentType::Unknown,
                    plan_name: plan_name.clone(),
                    plan_description: resp.plan_description,
                    steps,
                    alternatives,
                    confidence: resp.confidence,
                    risk_level: resp.risk_level,
                    estimated_duration_minutes: resp.estimated_duration_minutes,
                    estimated_cost: resp.estimated_cost,
                    resource_estimate: ResourceEstimate {
                        compute_hours: resp.resource_estimate.compute_hours,
                        storage_gb: resp.resource_estimate.storage_gb,
                        api_calls: resp.resource_estimate.api_calls,
                        llm_tokens: resp.resource_estimate.llm_tokens,
                        estimated_cost_usd: resp.resource_estimate.estimated_cost_usd,
                    },
                    basic_program: resp.basic_program
                        .or_else(|| Some(fallback_basic_program(&plan_name, original_intent))),
                    requires_approval: resp.requires_approval,
                    mcp_servers: resp.mcp_servers,
                    external_apis: resp.external_apis,
                    risks,
                })
            }
            Err(e) => {
                // A malformed completion must fail the create, not persist the
                // minimal stub as if an automation existed: the stub only TALKs,
                // so reporting it as created lies to the user (the pipeline's
                // core honesty rule). The error text reaches the reply verbatim.
                warn!("Failed to parse compile response: {e}");
                Err(format!("the model returned a plan that is not valid JSON ({e}); no automation was created — try again").into())
            }
        }
    }
}
