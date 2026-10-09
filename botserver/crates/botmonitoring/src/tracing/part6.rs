use super::*;

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            strategy: SamplingStrategy::Probabilistic,
            rate: 0.1,
            max_traces_per_second: 100,
            service_overrides: HashMap::new(),
            operation_overrides: HashMap::new(),
        }
    }
}
