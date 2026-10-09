use super::*;

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_interval: 60,
            cost_tracking: true,
            trace_enabled: true,
            trace_sample_rate: 1.0,
            budget_daily: 100.0,
            budget_monthly: 2000.0,
            alert_threshold: 0.8,
            model_pricing: default_model_pricing(),
        }
    }
}
pub(super) fn default_model_pricing() -> HashMap<String, ModelPricing> {
    let mut pricing = HashMap::new();

    pricing.insert(
        "gpt-4".to_string(),
        ModelPricing {
            model: "gpt-4".to_string(),
            input_cost_per_1k: 0.03,
            output_cost_per_1k: 0.06,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "gpt-4-turbo".to_string(),
        ModelPricing {
            model: "gpt-4-turbo".to_string(),
            input_cost_per_1k: 0.01,
            output_cost_per_1k: 0.03,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "gpt-3.5-turbo".to_string(),
        ModelPricing {
            model: "gpt-3.5-turbo".to_string(),
            input_cost_per_1k: 0.0005,
            output_cost_per_1k: 0.0015,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "claude-3-opus".to_string(),
        ModelPricing {
            model: "claude-3-opus".to_string(),
            input_cost_per_1k: 0.015,
            output_cost_per_1k: 0.075,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "claude-3-sonnet".to_string(),
        ModelPricing {
            model: "claude-3-sonnet".to_string(),
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "mixtral-8x7b-32768".to_string(),
        ModelPricing {
            model: "mixtral-8x7b-32768".to_string(),
            input_cost_per_1k: 0.00027,
            output_cost_per_1k: 0.00027,
            cost_per_request: 0.0,
            is_local: false,
        },
    );

    pricing.insert(
        "local".to_string(),
        ModelPricing {
            model: "local".to_string(),
            input_cost_per_1k: 0.0,
            output_cost_per_1k: 0.0,
            cost_per_request: 0.0,
            is_local: true,
        },
    );

    pricing
}
