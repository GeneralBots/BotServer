use super::*;

impl ObservabilityManager {
    pub fn new(config: ObservabilityConfig) -> Self {
        let budget = Budget {
            daily_limit: config.budget_daily,
            monthly_limit: config.budget_monthly,
            alert_threshold: config.alert_threshold,
            ..Default::default()
        };

        Self {
            config,
            metrics_buffer: Arc::new(RwLock::new(Vec::new())),
            current_metrics: Arc::new(RwLock::new(AggregatedMetrics::default())),
            budget: Arc::new(RwLock::new(budget)),
            trace_buffer: Arc::new(RwLock::new(Vec::new())),
            request_count: AtomicU64::new(0),
            token_count: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    pub fn from_config(config_map: &HashMap<String, String>) -> Self {
        let config = ObservabilityConfig {
            enabled: config_map
                .get("observability-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            metrics_interval: config_map
                .get("observability-metrics-interval")
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            cost_tracking: config_map
                .get("observability-cost-tracking")
                .map(|v| v == "true")
                .unwrap_or(true),
            trace_enabled: config_map
                .get("observability-trace-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            trace_sample_rate: config_map
                .get("observability-trace-sample-rate")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0),
            budget_daily: config_map
                .get("observability-budget-daily")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100.0),
            budget_monthly: config_map
                .get("observability-budget-monthly")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2000.0),
            alert_threshold: config_map
                .get("observability-alert-threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.8),
            model_pricing: default_model_pricing(),
        };
        Self::new(config)
    }

    pub(super) fn calculate_budget_status(budget: &Budget) -> BudgetStatus {
        BudgetStatus {
            daily_limit: budget.daily_limit,
            daily_spend: budget.daily_spend,
            daily_remaining: budget.daily_limit - budget.daily_spend,
            daily_percentage: (budget.daily_spend / budget.daily_limit) * 100.0,
            monthly_limit: budget.monthly_limit,
            monthly_spend: budget.monthly_spend,
            monthly_remaining: budget.monthly_limit - budget.monthly_spend,
            monthly_percentage: (budget.monthly_spend / budget.monthly_limit) * 100.0,
            daily_exceeded: budget.daily_spend > budget.daily_limit,
            monthly_exceeded: budget.monthly_spend > budget.monthly_limit,
            near_daily_limit: budget.daily_spend >= budget.daily_limit * budget.alert_threshold,
            near_monthly_limit: budget.monthly_spend
                >= budget.monthly_limit * budget.alert_threshold,
        }
    }

    pub async fn record_request(&self, metrics: LLMRequestMetrics) {
        if !self.config.enabled {
            return;
        }

        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.token_count
            .fetch_add(metrics.total_tokens, Ordering::Relaxed);

        if metrics.cached {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        if !metrics.success {
            self.error_count.fetch_add(1, Ordering::Relaxed);
        }

        if self.config.cost_tracking && metrics.estimated_cost > 0.0 {
            let mut budget = self.budget.write().await;
            budget.daily_spend += metrics.estimated_cost;
            budget.monthly_spend += metrics.estimated_cost;
        }

        let mut buffer = self.metrics_buffer.write().await;
        buffer.push(metrics);

        if buffer.len() > 10000 {
            buffer.drain(0..1000);
        }
    }

    pub fn calculate_cost(&self, model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        if let Some(pricing) = self.config.model_pricing.get(model) {
            if pricing.is_local {
                return 0.0;
            }
            let input_cost = (input_tokens as f64 / 1000.0) * pricing.input_cost_per_1k;
            let output_cost = (output_tokens as f64 / 1000.0) * pricing.output_cost_per_1k;
            input_cost + output_cost + pricing.cost_per_request
        } else {
            0.0
        }
    }

    pub async fn get_budget_status(&self) -> BudgetStatus {
        let budget = self.budget.read().await;
        Self::calculate_budget_status(&budget)
    }

    pub(super) fn check_budget_internal(budget: &Budget, estimated_cost: f64) -> BudgetCheckResult {
        let daily_after = budget.daily_spend + estimated_cost;
        let monthly_after = budget.monthly_spend + estimated_cost;

        if daily_after > budget.daily_limit {
            return BudgetCheckResult::DailyExceeded;
        }

        if monthly_after > budget.monthly_limit {
            return BudgetCheckResult::MonthlyExceeded;
        }

        if daily_after >= budget.daily_limit * budget.alert_threshold {
            return BudgetCheckResult::NearDailyLimit;
        }

        if monthly_after >= budget.monthly_limit * budget.alert_threshold {
            return BudgetCheckResult::NearMonthlyLimit;
        }

        BudgetCheckResult::Ok
    }

    pub async fn check_budget(&self, estimated_cost: f64) -> BudgetCheckResult {
        let budget = self.budget.read().await;
        Self::check_budget_internal(&budget, estimated_cost)
    }

    pub async fn reset_daily_budget(&self) {
        let mut budget = self.budget.write().await;
        budget.daily_spend = 0.0;
        budget.daily_reset_date = Utc::now();
        budget.daily_alert_sent = false;
    }

    pub async fn reset_monthly_budget(&self) {
        let mut budget = self.budget.write().await;
        budget.monthly_spend = 0.0;
        budget.monthly_reset_date = Utc::now();
        budget.monthly_alert_sent = false;
    }

    pub async fn record_trace(&self, event: TraceEvent) {
        if !self.config.enabled || !self.config.trace_enabled {
            return;
        }

        if self.config.trace_sample_rate < 1.0 {
            let sample: f64 = rand::random();
            if sample > self.config.trace_sample_rate {
                return;
            }
        }

        let mut buffer = self.trace_buffer.write().await;
        buffer.push(event);

        if buffer.len() > 5000 {
            buffer.drain(0..500);
        }
    }

    pub fn start_span(
        &self,
        trace_id: Uuid,
        name: &str,
        component: &str,
        parent_id: Option<Uuid>,
    ) -> TraceEvent {
        TraceEvent {
            id: Uuid::new_v4(),
            parent_id,
            trace_id,
            name: name.to_string(),
            component: component.to_string(),
            event_type: TraceEventType::Span,
            duration_ms: None,
            start_time: Utc::now(),
            end_time: None,
            attributes: HashMap::new(),
            status: TraceStatus::InProgress,
            error: None,
        }
    }

    pub fn end_span(&self, span: &mut TraceEvent, status: TraceStatus, error: Option<String>) {
        let end_time = Utc::now();
        span.end_time = Some(end_time);
        span.duration_ms = Some((end_time - span.start_time).num_milliseconds() as u64);
        span.status = status;
        span.error = error;
    }
}
