use super::*;

impl DistributedTracingService {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(RwLock::new(HashMap::new())),
            sampling_config: Arc::new(RwLock::new(SamplingConfig::default())),
            exporter_config: Arc::new(RwLock::new(ExporterConfig::default())),
            resource: Arc::new(ResourceAttributes::default()),
            span_buffer: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn with_resource(mut self, resource: ResourceAttributes) -> Self {
        self.resource = Arc::new(resource);
        self
    }

    pub fn span(&self, operation_name: &str) -> SpanBuilder {
        SpanBuilder::new(operation_name)
            .with_service_name(&self.resource.service_name)
    }

    pub async fn should_sample(&self, trace_id: &str, operation_name: &str) -> bool {
        let config = self.sampling_config.read().await;

        if let Some(rate) = config.operation_overrides.get(operation_name) {
            return self.should_sample_with_rate(*rate, trace_id);
        }

        match config.strategy {
            SamplingStrategy::Always => true,
            SamplingStrategy::Never => false,
            SamplingStrategy::Probabilistic => self.should_sample_with_rate(config.rate, trace_id),
            SamplingStrategy::RateLimiting | SamplingStrategy::Adaptive => {
                self.should_sample_with_rate(config.rate, trace_id)
            }
        }
    }

    pub(super) fn should_sample_with_rate(&self, rate: f32, trace_id: &str) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        trace_id.hash(&mut hasher);
        let hash = hasher.finish();
        let normalized = (hash as f64) / (u64::MAX as f64);
        normalized < (rate as f64)
    }

    pub async fn record_span(&self, mut span: Span) {
        span.resource = (*self.resource).clone();

        let mut spans = self.spans.write().await;
        spans
            .entry(span.trace_id.clone())
            .or_default()
            .push(span.clone());

        let mut buffer = self.span_buffer.write().await;
        buffer.push(span);

        let config = self.exporter_config.read().await;
        if buffer.len() >= config.batch_size as usize {
            let spans_to_export: Vec<Span> = buffer.drain(..).collect();
            drop(buffer);
            drop(config);
            self.export_spans(spans_to_export).await;
        }
    }

    async fn export_spans(&self, spans: Vec<Span>) {
        let config = self.exporter_config.read().await;

        if !config.enabled || spans.is_empty() {
            return;
        }

        match config.exporter_type {
            ExporterType::Console => {
                for span in &spans {
                    log::debug!(
                        "TRACE [{}] {} {} {}us",
                        span.trace_id,
                        span.operation_name,
                        match span.status {
                            SpanStatus::Ok => "OK",
                            SpanStatus::Error => "ERROR",
                            SpanStatus::Unset => "UNSET",
                        },
                        span.duration_us.unwrap_or(0)
                    );
                }
            }
            ExporterType::Otlp => {
                let endpoint = config.endpoint.trim_end_matches('/').to_string()
                    + "/v1/traces";
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .map_err(|e| log::error!("Failed to build OTLP client: {}", e));
                if let Ok(client) = client {
                    let payload = serde_json::json!({
                        "resourceSpans": [{
                            "resource": {
                                "attributes": [
                                    {"key": "service.name", "value": {"stringValue": spans[0].resource.service_name.clone()}}
                                ]
                            },
                            "scopeSpans": [{
                                "scope": {"name": "botserver"},
                                "spans": spans.iter().map(|s| {
                                    serde_json::json!({
                                        "traceId": s.trace_id,
                                        "spanId": s.span_id,
                                        "parentSpanId": s.parent_span_id,
                                        "name": s.operation_name,
                                        "kind": match s.kind {
                                            SpanKind::Internal => 1,
                                            SpanKind::Server => 2,
                                            SpanKind::Client => 3,
                                            SpanKind::Producer => 4,
                                            SpanKind::Consumer => 5,
                                        },
                                        "startTimeUnixNano": s.start_time.timestamp_nanos_opt().unwrap_or(0).to_string(),
                                        "endTimeUnixNano": s.end_time.and_then(|t| t.timestamp_nanos_opt()).unwrap_or(0).to_string(),
                                        "status": match s.status {
                                            SpanStatus::Ok => serde_json::json!({"code": 1}),
                                            SpanStatus::Error => serde_json::json!({"code": 2, "message": s.status_message}),
                                            SpanStatus::Unset => serde_json::json!({"code": 0}),
                                        },
                                        "attributes": s.attributes.iter().map(|(k, v)| {
                                            serde_json::json!({"key": k, "value": attr_to_otlp_value(v)})
                                        }).collect::<Vec<_>>(),
                                    })
                                }).collect::<Vec<_>>()
                            }]
                        }]
                    });
                    match client.post(&endpoint).json(&payload).send().await {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                log::warn!("OTLP export returned status: {}", resp.status());
                            }
                        }
                        Err(e) => log::error!("OTLP export failed: {}", e),
                    }
                }
            }
            ExporterType::Jaeger => {
                let endpoint = config.endpoint.trim_end_matches('/').to_string()
                    + "/api/traces";
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .map_err(|e| log::error!("Failed to build Jaeger client: {}", e));
                if let Ok(client) = client {
                    let batch = serde_json::json!({
                        "data": [{
                            "traceID": spans[0].trace_id,
                            "spans": spans.iter().map(|s| {
                                let start_us = s.start_time.timestamp_nanos_opt().unwrap_or(0) / 1000;
                                let duration_us = s.duration_us.unwrap_or(0);
                                serde_json::json!({
                                    "traceID": s.trace_id,
                                    "spanID": s.span_id,
                                    "operationName": s.operation_name,
                                    "startTime": start_us,
                                    "duration": duration_us,
                                    "tags": jaeger_tags(s),
                                    "logs": s.events.iter().map(|e| {
                                        serde_json::json!({
                                            "timestamp": e.timestamp.timestamp_nanos_opt().unwrap_or(0) / 1000,
                                            "fields": [{"key": "event", "type": "string", "value": e.name}]
                                        })
                                    }).collect::<Vec<_>>(),
                                })
                            }).collect::<Vec<_>>()
                        }]
                    });
                    match client.post(&endpoint).json(&batch).send().await {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                log::warn!("Jaeger export returned status: {}", resp.status());
                            }
                        }
                        Err(e) => log::error!("Jaeger export failed: {}", e),
                    }
                }
            }
            ExporterType::Zipkin => {
                let endpoint = config.endpoint.trim_end_matches('/').to_string()
                    + "/api/v2/spans";
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .map_err(|e| log::error!("Failed to build Zipkin client: {}", e));
                if let Ok(client) = client {
                    let zipkin_spans: Vec<serde_json::Value> = spans.iter().map(|s| {
                        let ts_micros = s.start_time.timestamp_nanos_opt().unwrap_or(0) / 1000;
                        let duration_micros = s.duration_us.unwrap_or(0).max(1);
                        serde_json::json!({
                            "id": s.span_id,
                            "traceId": s.trace_id,
                            "parentId": s.parent_span_id,
                            "name": s.operation_name,
                            "timestamp": ts_micros,
                            "duration": duration_micros,
                            "localEndpoint": {
                                "serviceName": s.service_name
                            },
                            "tags": zipkin_tags(s),
                        })
                    }).collect();
                    match client.post(&endpoint).json(&zipkin_spans).send().await {
                        Ok(resp) => {
                            if !resp.status().is_success() {
                                log::warn!("Zipkin export returned status: {}", resp.status());
                            }
                        }
                        Err(e) => log::error!("Zipkin export failed: {}", e),
                    }
                }
            }
            ExporterType::None => {}
        }
    }

    pub async fn get_trace(&self, trace_id: &str) -> Option<Trace> {
        let spans = self.spans.read().await;
        let trace_spans = spans.get(trace_id)?;

        if trace_spans.is_empty() {
            return None;
        }

        let root_span = trace_spans.iter().find(|s| s.parent_span_id.is_none());

        let services: std::collections::HashSet<_> = trace_spans.iter().map(|s| &s.service_name).collect();
        let has_errors = trace_spans.iter().any(|s| s.status == SpanStatus::Error);

        let start_time = trace_spans.iter().map(|s| s.start_time).min();
        let end_time = trace_spans.iter().filter_map(|s| s.end_time).max();

        let duration_us = match (start_time, end_time) {
            (Some(start), Some(end)) => Some((end - start).num_microseconds().unwrap_or(0)),
            _ => None,
        };

        Some(Trace {
            trace_id: trace_id.to_string(),
            spans: trace_spans.clone(),
            root_span_id: root_span.map(|s| s.span_id.clone()),
            service_count: services.len() as u32,
            span_count: trace_spans.len() as u32,
            duration_us,
            has_errors,
            start_time,
            end_time,
        })
    }
}
