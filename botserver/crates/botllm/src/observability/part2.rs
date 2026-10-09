use super::*;

impl Default for ModelPricing {
    fn default() -> Self {
        Self {
            model: "default".to_string(),
            input_cost_per_1k: 0.0,
            output_cost_per_1k: 0.0,
            cost_per_request: 0.0,
            is_local: true,
        }
    }
}
