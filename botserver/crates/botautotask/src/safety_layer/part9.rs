use super::*;

impl Default for PredictedResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_mb: 0.0,
            network_kbps: 0.0,
            disk_io_kbps: 0.0,
            api_calls: 0,
            db_queries: 0,
            llm_tokens: 0,
        }
    }
}
