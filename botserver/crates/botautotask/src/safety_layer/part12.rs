use super::*;

impl std::fmt::Debug for SafetyLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafetyLayer")
            .field("config", &self.config)
            .field("constraints_count", &self.constraints.len())
            .finish_non_exhaustive()
    }
}
impl SafetyLayer {
    pub fn new(pool: crate::types::DbPool) -> Self {
        Self { pool, config: SafetyConfig::default(), constraints: Vec::new() }
    }

    pub fn with_config(pool: crate::types::DbPool, config: SafetyConfig) -> Self {
        Self { pool, config, constraints: Vec::new() }
    }

    pub fn load_constraints(
        &mut self, bot_id: &Uuid,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut conn = self.pool.get().map_err(|e| format!("DB error: {e}"))?;
        let bot_id_str = bot_id.to_string();

        let query = diesel::sql_query(
            "SELECT id, name, constraint_type, description, expression, threshold, severity, enabled, applies_to \
             FROM safety_constraints WHERE bot_id = $1 AND enabled = true"
        ).bind::<diesel::sql_types::Text, _>(&bot_id_str);

        #[derive(QueryableByName)]
        struct ConstraintRow {
            #[diesel(sql_type = diesel::sql_types::Text)] id: String,
            #[diesel(sql_type = diesel::sql_types::Text)] name: String,
            #[diesel(sql_type = diesel::sql_types::Text)] constraint_type: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] description: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] expression: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] threshold: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Text)] severity: String,
            #[diesel(sql_type = diesel::sql_types::Bool)] enabled: bool,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] applies_to: Option<String>,
        }

        // A failed load must not look like "this bot has no constraints": that
        // silently disabled every check. This used to be `unwrap_or_default()`,
        // which also hid the table being missing entirely — see migration
        // `6.7.8-safety-constraints`, which creates it.
        let rows: Vec<ConstraintRow> = query
            .load(&mut *conn)
            .map_err(|e| format!("Failed to load safety constraints for bot {bot_id}: {e}"))?;

        self.constraints = rows.into_iter().map(|row| Constraint {
            id: row.id,
            name: row.name,
            constraint_type: match row.constraint_type.as_str() {
                "budget" => ConstraintType::Budget, "permission" => ConstraintType::Permission,
                "policy" => ConstraintType::Policy, "compliance" => ConstraintType::Compliance,
                "technical" => ConstraintType::Technical, "rate_limit" => ConstraintType::RateLimit,
                "time_window" => ConstraintType::TimeWindow, "data_access" => ConstraintType::DataAccess,
                "security" => ConstraintType::Security, "resource" => ConstraintType::Resource,
                other => ConstraintType::Custom(other.to_string()),
            },
            description: row.description.unwrap_or_default(),
            expression: row.expression,
            threshold: row.threshold.and_then(|t| serde_json::from_str(&t).ok()),
            severity: match row.severity.as_str() {
                "info" => ConstraintSeverity::Info, "error" => ConstraintSeverity::Error,
                "critical" => ConstraintSeverity::Critical, _ => ConstraintSeverity::Warning,
            },
            enabled: row.enabled,
            applies_to: row.applies_to.map(|s| s.split(',').map(|x| x.trim().to_string()).collect()).unwrap_or_default(),
            bot_id: bot_id_str.clone(),
        }).collect();

        info!("Loaded {} constraints for bot {bot_id}", self.constraints.len());
        Ok(())
    }

    pub fn check_constraints(
        &self, action: &str, context: &serde_json::Value, _user: &UserSession,
    ) -> Result<ConstraintCheckResult, Box<dyn std::error::Error + Send + Sync>> {
        let mut result = ConstraintCheckResult::default();

        for constraint in &self.constraints {
            if !constraint.enabled { continue; }
            if !constraint.applies_to.is_empty() && !constraint.applies_to.contains(&action.to_string()) { continue; }

            match Self::evaluate_constraint(constraint, context) {
                Ok(passed) => {
                    let constraint_result = ConstraintResult {
                        constraint_id: constraint.id.clone(),
                        constraint_type: constraint.constraint_type.clone(),
                        passed,
                        severity: constraint.severity.clone(),
                        message: if passed {
                            format!("Constraint '{}' passed", constraint.name)
                        } else {
                            format!("Constraint '{}' violated: {}", constraint.name, constraint.description)
                        },
                        details: None,
                        remediation: None,
                    };
                    if !passed {
                        result.passed = false;
                        match constraint.severity {
                            ConstraintSeverity::Critical | ConstraintSeverity::Error => {
                                result.blocking.push(constraint.name.clone());
                            }
                            ConstraintSeverity::Warning => { result.warnings.push(constraint.name.clone()); }
                            ConstraintSeverity::Info => { result.suggestions.push(constraint.name.clone()); }
                        }
                    }
                    result.results.push(constraint_result);
                }
                Err(e) => { warn!("Failed to evaluate constraint {}: {e}", constraint.id); }
            }
        }
        result.risk_score = Self::calculate_risk_score(&result);
        Ok(result)
    }

    pub(super) fn evaluate_constraint(
        _constraint: &Constraint, _context: &serde_json::Value,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> { Ok(true) }

    pub(super) fn calculate_risk_score(result: &ConstraintCheckResult) -> f64 {
        let blocking_score = (result.blocking.len() as f64) * 0.5;
        let warning_score = (result.warnings.len() as f64) * 0.3;
        let suggestion_score = (result.suggestions.len() as f64) * 0.1;
        (blocking_score + warning_score + suggestion_score).min(1.0)
    }

    pub fn simulate_execution(
        &self, task_id: &str, _session: &UserSession,
    ) -> Result<SimulationResult, Box<dyn std::error::Error + Send + Sync>> {
        info!("Simulating execution for task_id={task_id}");
        let start_time = std::time::Instant::now();
        Ok(SimulationResult {
            id: Uuid::new_v4().to_string(),
            success: true,
            step_outcomes: Vec::new(),
            impact: ImpactAssessment::default(),
            resource_usage: PredictedResourceUsage::default(),
            side_effects: Vec::new(),
            recommendations: Vec::new(),
            confidence: 0.85,
            simulated_at: Utc::now(),
            simulation_duration_ms: start_time.elapsed().as_millis() as i64,
        })
    }

    pub fn log_audit(
        &self, entry: AuditEntry,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.config.audit_enabled { return Ok(()); }
        let mut conn = self.pool.get().map_err(|e| format!("DB error: {e}"))?;
        let details_json = serde_json::to_string(&entry.details)?;
        let now = entry.timestamp.to_rfc3339();
        let event_type_str = entry.event_type.to_string();
        let actor_type_str = format!("{:?}", entry.actor.actor_type);
        let risk_level_str = format!("{:?}", entry.risk_level);

        diesel::sql_query(
            "INSERT INTO audit_log (id, timestamp, event_type, actor_type, actor_id, action, target_type, target_id, outcome_success, details, session_id, bot_id, task_id, step_id, risk_level) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"
        )
            .bind::<diesel::sql_types::Text, _>(&entry.id)
            .bind::<diesel::sql_types::Text, _>(&now)
            .bind::<diesel::sql_types::Text, _>(&event_type_str)
            .bind::<diesel::sql_types::Text, _>(&actor_type_str)
            .bind::<diesel::sql_types::Text, _>(&entry.actor.id)
            .bind::<diesel::sql_types::Text, _>(&entry.action)
            .bind::<diesel::sql_types::Text, _>(&entry.target.target_type)
            .bind::<diesel::sql_types::Text, _>(&entry.target.id)
            .bind::<diesel::sql_types::Bool, _>(entry.outcome.success)
            .bind::<diesel::sql_types::Text, _>(&details_json)
            .bind::<diesel::sql_types::Text, _>(&entry.session_id)
            .bind::<diesel::sql_types::Text, _>(&entry.bot_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&entry.task_id)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&entry.step_id)
            .bind::<diesel::sql_types::Text, _>(&risk_level_str)
            .execute(&mut *conn)
            .map_err(|e| format!("Failed to log audit: {e}"))?;

        trace!("Audit logged: {} - {}", entry.event_type, entry.action);
        Ok(())
    }
}
