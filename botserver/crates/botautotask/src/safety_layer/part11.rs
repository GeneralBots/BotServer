use super::*;

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_constraints: true,
            simulate_impact: true,
            audit_enabled: true,
            approval_threshold: RiskLevel::High,
            max_auto_execute_risk: RiskLevel::Low,
            default_budget_limit: 100.0,
            rate_limit_per_minute: 60,
            circuit_breaker_threshold: 5,
            audit_retention_days: 90,
            require_simulation_for: vec![
                "DELETE".to_string(), "UPDATE".to_string(), "RUN_PYTHON".to_string(),
                "RUN_BASH".to_string(), "POST".to_string(), "PUT".to_string(), "PATCH".to_string(),
            ],
        }
    }
}
