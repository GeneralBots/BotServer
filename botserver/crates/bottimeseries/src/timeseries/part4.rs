use super::*;

impl Metrics {
    pub fn message(bot_id: &str, channel: &str, direction: &str) -> MetricPoint {
        MetricPoint::new("messages")
            .tag("bot_id", bot_id)
            .tag("channel", channel)
            .tag("direction", direction)
            .field_i64("count", 1)
            .at(Utc::now())
    }

    pub fn response_time(bot_id: &str, duration_ms: f64) -> MetricPoint {
        MetricPoint::new("response_time")
            .tag("bot_id", bot_id)
            .field_f64("duration_ms", duration_ms)
            .at(Utc::now())
    }

    pub fn llm_tokens(
        bot_id: &str,
        model: &str,
        prompt_tokens: i64,
        completion_tokens: i64,
    ) -> MetricPoint {
        MetricPoint::new("llm_tokens")
            .tag("bot_id", bot_id)
            .tag("model", model)
            .field_i64("prompt_tokens", prompt_tokens)
            .field_i64("completion_tokens", completion_tokens)
            .field_i64("total_tokens", prompt_tokens + completion_tokens)
            .at(Utc::now())
    }

    pub fn active_sessions(bot_id: &str, count: i64) -> MetricPoint {
        MetricPoint::new("active_sessions")
            .tag("bot_id", bot_id)
            .field_i64("count", count)
            .at(Utc::now())
    }

    pub fn error(bot_id: &str, error_type: &str, message: &str) -> MetricPoint {
        MetricPoint::new("errors")
            .tag("bot_id", bot_id)
            .tag("error_type", error_type)
            .field_i64("count", 1)
            .field_str("message", message)
            .at(Utc::now())
    }

    pub fn storage_usage(bot_id: &str, bytes_used: u64, file_count: u64) -> MetricPoint {
        MetricPoint::new("storage_usage")
            .tag("bot_id", bot_id)
            .field_u64("bytes_used", bytes_used)
            .field_u64("file_count", file_count)
            .at(Utc::now())
    }

    pub fn api_request(
        endpoint: &str,
        method: &str,
        status_code: i64,
        duration_ms: f64,
    ) -> MetricPoint {
        MetricPoint::new("api_requests")
            .tag("endpoint", endpoint)
            .tag("method", method)
            .field_i64("status_code", status_code)
            .field_f64("duration_ms", duration_ms)
            .field_i64("count", 1)
            .at(Utc::now())
    }

    pub fn system(cpu_percent: f64, memory_percent: f64, disk_percent: f64) -> MetricPoint {
        MetricPoint::new("system_metrics")
            .field_f64("cpu_percent", cpu_percent)
            .field_f64("memory_percent", memory_percent)
            .field_f64("disk_percent", disk_percent)
            .at(Utc::now())
    }
}
