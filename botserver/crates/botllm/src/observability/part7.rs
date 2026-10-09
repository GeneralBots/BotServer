use super::*;

impl LLMRequestMetrics {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("request_id".into(), self.request_id.to_string().into());
        map.insert("session_id".into(), self.session_id.to_string().into());
        map.insert("bot_id".into(), self.bot_id.to_string().into());
        map.insert("model".into(), self.model.clone().into());
        map.insert("request_type".into(), self.request_type.to_string().into());
        map.insert("input_tokens".into(), (self.input_tokens as i64).into());
        map.insert("output_tokens".into(), (self.output_tokens as i64).into());
        map.insert("total_tokens".into(), (self.total_tokens as i64).into());
        map.insert("latency_ms".into(), (self.latency_ms as i64).into());
        map.insert("cached".into(), self.cached.into());
        map.insert("success".into(), self.success.into());
        map.insert("estimated_cost".into(), self.estimated_cost.into());
        map.insert("timestamp".into(), self.timestamp.to_rfc3339().into());

        if let Some(ttft) = self.ttft_ms {
            map.insert("ttft_ms".into(), (ttft as i64).into());
        }

        if let Some(ref error) = self.error {
            map.insert("error".into(), error.clone().into());
        }

        Dynamic::from(map)
    }
}
impl AggregatedMetrics {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("period_start".into(), self.period_start.to_rfc3339().into());
        map.insert("period_end".into(), self.period_end.to_rfc3339().into());
        map.insert("total_requests".into(), (self.total_requests as i64).into());
        map.insert(
            "successful_requests".into(),
            (self.successful_requests as i64).into(),
        );
        map.insert(
            "failed_requests".into(),
            (self.failed_requests as i64).into(),
        );
        map.insert("cache_hits".into(), (self.cache_hits as i64).into());
        map.insert("cache_misses".into(), (self.cache_misses as i64).into());
        map.insert(
            "total_input_tokens".into(),
            (self.total_input_tokens as i64).into(),
        );
        map.insert(
            "total_output_tokens".into(),
            (self.total_output_tokens as i64).into(),
        );
        map.insert("total_tokens".into(), (self.total_tokens as i64).into());
        map.insert("total_cost".into(), self.total_cost.into());
        map.insert("avg_latency_ms".into(), self.avg_latency_ms.into());
        map.insert("p50_latency_ms".into(), self.p50_latency_ms.into());
        map.insert("p95_latency_ms".into(), self.p95_latency_ms.into());
        map.insert("p99_latency_ms".into(), self.p99_latency_ms.into());
        map.insert("max_latency_ms".into(), (self.max_latency_ms as i64).into());
        map.insert("min_latency_ms".into(), (self.min_latency_ms as i64).into());

        Dynamic::from(map)
    }
}
impl BudgetStatus {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();

        map.insert("daily_limit".into(), self.daily_limit.into());
        map.insert("daily_spend".into(), self.daily_spend.into());
        map.insert("daily_remaining".into(), self.daily_remaining.into());
        map.insert("daily_percentage".into(), self.daily_percentage.into());
        map.insert("monthly_limit".into(), self.monthly_limit.into());
        map.insert("monthly_spend".into(), self.monthly_spend.into());
        map.insert("monthly_remaining".into(), self.monthly_remaining.into());
        map.insert("monthly_percentage".into(), self.monthly_percentage.into());
        map.insert("daily_exceeded".into(), self.daily_exceeded.into());
        map.insert("monthly_exceeded".into(), self.monthly_exceeded.into());
        map.insert("near_daily_limit".into(), self.near_daily_limit.into());
        map.insert("near_monthly_limit".into(), self.near_monthly_limit.into());

        Dynamic::from(map)
    }
}
pub fn register_observability_keywords(engine: &mut Engine) {
    engine.register_fn("metrics_total_requests", |metrics: Map| -> i64 {
        metrics
            .get("total_requests")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(0)
    });

    engine.register_fn("metrics_cache_hit_rate", |metrics: Map| -> f64 {
        let hits = metrics
            .get("cache_hits")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(0) as f64;
        let misses = metrics
            .get("cache_misses")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(0) as f64;
        if hits + misses > 0.0 {
            hits / (hits + misses)
        } else {
            0.0
        }
    });

    engine.register_fn("metrics_error_rate", |metrics: Map| -> f64 {
        let failed = metrics
            .get("failed_requests")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(0) as f64;
        let total = metrics
            .get("total_requests")
            .and_then(|v| v.clone().try_cast::<i64>())
            .unwrap_or(0) as f64;
        if total > 0.0 {
            failed / total
        } else {
            0.0
        }
    });

    engine.register_fn("budget_is_exceeded", |status: Map| -> bool {
        status
            .get("daily_exceeded")
            .and_then(|v| v.clone().try_cast::<bool>())
            .unwrap_or(false)
            || status
                .get("monthly_exceeded")
                .and_then(|v| v.clone().try_cast::<bool>())
                .unwrap_or(false)
    });

    engine.register_fn("budget_remaining_daily", |status: Map| -> f64 {
        status
            .get("daily_remaining")
            .and_then(|v| v.clone().try_cast::<f64>())
            .unwrap_or(0.0)
    });

    engine.register_fn("budget_remaining_monthly", |status: Map| -> f64 {
        status
            .get("monthly_remaining")
            .and_then(|v| v.clone().try_cast::<f64>())
            .unwrap_or(0.0)
    });

    info!("Observability keywords registered");
}
