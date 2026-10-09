use super::*;

impl Default for CostImpact {
    fn default() -> Self {
        Self {
            api_costs: 0.0,
            compute_costs: 0.0,
            storage_costs: 0.0,
            total_estimated_cost: 0.0,
            currency: "USD".to_string(),
            exceeds_budget: false,
            budget_remaining: None,
        }
    }
}
