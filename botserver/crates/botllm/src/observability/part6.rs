use super::*;

impl ObservabilityManager {
    pub async fn get_aggregated_metrics(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> AggregatedMetrics {
        let filtered: Vec<LLMRequestMetrics> = {
            let buffer = self.metrics_buffer.read().await;
            buffer
                .iter()
                .filter(|m| m.timestamp >= start && m.timestamp <= end)
                .cloned()
                .collect()
        };

        if filtered.is_empty() {
            return AggregatedMetrics {
                period_start: start,
                period_end: end,
                ..Default::default()
            };
        }

        let mut metrics = AggregatedMetrics {
            period_start: start,
            period_end: end,
            total_requests: filtered.len() as u64,
            ..Default::default()
        };

        let mut latencies: Vec<u64> = Vec::new();

        for m in &filtered {
            if m.success {
                metrics.successful_requests += 1;
            } else {
                metrics.failed_requests += 1;
                *metrics
                    .errors_by_type
                    .entry(m.error.clone().unwrap_or_else(|| "unknown".to_string()))
                    .or_insert(0) += 1;
            }

            if m.cached {
                metrics.cache_hits += 1;
            } else {
                metrics.cache_misses += 1;
            }

            metrics.total_input_tokens += m.input_tokens;
            metrics.total_output_tokens += m.output_tokens;
            metrics.total_tokens += m.total_tokens;
            metrics.total_cost += m.estimated_cost;

            latencies.push(m.latency_ms);

            *metrics
                .requests_by_model
                .entry(m.model.clone())
                .or_insert(0) += 1;
            *metrics.tokens_by_model.entry(m.model.clone()).or_insert(0) += m.total_tokens;
            *metrics.cost_by_model.entry(m.model.clone()).or_insert(0.0) += m.estimated_cost;
            *metrics
                .requests_by_type
                .entry(m.request_type.to_string())
                .or_insert(0) += 1;
        }

        if !latencies.is_empty() {
            latencies.sort_unstable();
            let len = latencies.len();

            metrics.avg_latency_ms = latencies.iter().sum::<u64>() as f64 / len as f64;
            metrics.min_latency_ms = latencies[0];
            metrics.max_latency_ms = latencies[len - 1];
            metrics.p50_latency_ms = latencies[len / 2] as f64;
            metrics.p95_latency_ms = latencies[(len as f64 * 0.95) as usize] as f64;
            metrics.p99_latency_ms =
                latencies[(len as f64 * 0.99).min((len - 1) as f64) as usize] as f64;
        }

        metrics
    }

    pub async fn get_current_metrics(&self) -> AggregatedMetrics {
        self.current_metrics.read().await.clone()
    }

    pub async fn update_current_metrics(&self) {
        let now = Utc::now();
        let start = now - chrono::Duration::hours(1);
        let metrics = self.get_aggregated_metrics(start, now).await;
        let mut current = self.current_metrics.write().await;
        *current = metrics;
    }

    pub fn get_quick_stats(&self) -> QuickStats {
        QuickStats {
            total_requests: self.request_count.load(Ordering::Relaxed),
            total_tokens: self.token_count.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            error_count: self.error_count.load(Ordering::Relaxed),
            cache_hit_rate: {
                let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
                let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
                if hits + misses > 0.0 {
                    hits / (hits + misses)
                } else {
                    0.0
                }
            },
            error_rate: {
                let errors = self.error_count.load(Ordering::Relaxed) as f64;
                let total = self.request_count.load(Ordering::Relaxed) as f64;
                if total > 0.0 {
                    errors / total
                } else {
                    0.0
                }
            },
        }
    }

    pub async fn get_recent_traces(&self, limit: usize) -> Vec<TraceEvent> {
        let buffer = self.trace_buffer.read().await;
        buffer.iter().rev().take(limit).cloned().collect()
    }

    pub async fn get_trace(&self, trace_id: Uuid) -> Vec<TraceEvent> {
        let buffer = self.trace_buffer.read().await;
        buffer
            .iter()
            .filter(|t| t.trace_id == trace_id)
            .cloned()
            .collect()
    }
}
