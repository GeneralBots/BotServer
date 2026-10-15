use super::*;

impl DistributedTracingService {
    pub async fn query_traces(&self, query: TraceQuery) -> Vec<Trace> {
        let spans = self.spans.read().await;
        let mut traces: Vec<Trace> = Vec::new();

        for (trace_id, trace_spans) in spans.iter() {
            let matches = trace_spans.iter().any(|span| {
                let mut matches = true;

                if let Some(ref service) = query.service_name {
                    matches = matches && &span.service_name == service;
                }

                if let Some(ref operation) = query.operation_name {
                    matches = matches && &span.operation_name == operation;
                }

                if let Some(min_duration) = query.min_duration_us {
                    matches = matches && span.duration_us.unwrap_or(0) >= min_duration;
                }

                if let Some(max_duration) = query.max_duration_us {
                    matches = matches && span.duration_us.unwrap_or(0) <= max_duration;
                }

                if let Some(start_time) = query.start_time {
                    matches = matches && span.start_time >= start_time;
                }

                if let Some(end_time) = query.end_time {
                    matches = matches && span.start_time <= end_time;
                }

                if let Some(has_errors) = query.has_errors {
                    if has_errors {
                        matches = matches && span.status == SpanStatus::Error;
                    }
                }

                matches
            });

            if matches {
                if let Some(trace) = self.build_trace(trace_id, trace_spans) {
                    traces.push(trace);
                }
            }
        }

        traces.sort_by_key(|a| std::cmp::Reverse(a.start_time));

        traces
            .into_iter()
            .skip(query.offset as usize)
            .take(query.limit as usize)
            .collect()
    }

    pub(super) fn build_trace(&self, trace_id: &str, trace_spans: &[Span]) -> Option<Trace> {
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
            spans: trace_spans.to_vec(),
            root_span_id: root_span.map(|s| s.span_id.clone()),
            service_count: services.len() as u32,
            span_count: trace_spans.len() as u32,
            duration_us,
            has_errors,
            start_time,
            end_time,
        })
    }

    pub async fn get_statistics(&self, duration_hours: u32) -> TraceStatistics {
        let spans = self.spans.read().await;
        let cutoff = Utc::now() - chrono::Duration::hours(duration_hours as i64);

        let mut total_spans: u64 = 0;
        let mut total_duration: i64 = 0;
        let mut error_count: u64 = 0;
        let mut durations: Vec<i64> = Vec::new();
        let mut service_stats: HashMap<String, ServiceStats> = HashMap::new();
        let mut operation_stats: HashMap<String, OperationStats> = HashMap::new();

        for trace_spans in spans.values() {
            for span in trace_spans {
                if span.start_time < cutoff {
                    continue;
                }

                total_spans += 1;
                let duration = span.duration_us.unwrap_or(0);
                total_duration += duration;
                durations.push(duration);

                if span.status == SpanStatus::Error {
                    error_count += 1;
                }

                let service_entry = service_stats
                    .entry(span.service_name.clone())
                    .or_insert(ServiceStats {
                        span_count: 0,
                        error_count: 0,
                        avg_duration_us: 0.0,
                    });
                service_entry.span_count += 1;
                if span.status == SpanStatus::Error {
                    service_entry.error_count += 1;
                }

                let op_entry = operation_stats
                    .entry(span.operation_name.clone())
                    .or_insert(OperationStats {
                        span_count: 0,
                        error_count: 0,
                        avg_duration_us: 0.0,
                        min_duration_us: i64::MAX,
                        max_duration_us: 0,
                    });
                op_entry.span_count += 1;
                if span.status == SpanStatus::Error {
                    op_entry.error_count += 1;
                }
                op_entry.min_duration_us = op_entry.min_duration_us.min(duration);
                op_entry.max_duration_us = op_entry.max_duration_us.max(duration);
            }
        }

        durations.sort();

        let p50 = self.percentile(&durations, 50);
        let p90 = self.percentile(&durations, 90);
        let p95 = self.percentile(&durations, 95);
        let p99 = self.percentile(&durations, 99);

        let avg_duration = if total_spans > 0 {
            total_duration as f64 / total_spans as f64
        } else {
            0.0
        };

        let error_rate = if total_spans > 0 {
            error_count as f32 / total_spans as f32
        } else {
            0.0
        };

        let spans_per_second = if duration_hours > 0 {
            total_spans as f32 / (duration_hours as f32 * 3600.0)
        } else {
            0.0
        };

        TraceStatistics {
            total_traces: spans.len() as u64,
            total_spans,
            error_rate,
            avg_duration_us: avg_duration,
            p50_duration_us: p50,
            p90_duration_us: p90,
            p95_duration_us: p95,
            p99_duration_us: p99,
            spans_per_second,
            service_breakdown: service_stats,
            operation_breakdown: operation_stats,
        }
    }

    pub async fn get_service_dependencies(&self) -> Vec<ServiceDependency> {
        let spans = self.spans.read().await;
        let mut dependencies: HashMap<(String, String), ServiceDependency> = HashMap::new();

        for trace_spans in spans.values() {
            let span_map: HashMap<&str, &Span> = trace_spans
                .iter()
                .map(|s| (s.span_id.as_str(), s))
                .collect();

            for span in trace_spans {
                if let Some(parent_id) = &span.parent_span_id {
                    if let Some(parent_span) = span_map.get(parent_id.as_str()) {
                        if parent_span.service_name != span.service_name {
                            let key = (parent_span.service_name.clone(), span.service_name.clone());
                            let dep = dependencies.entry(key).or_insert(ServiceDependency {
                                parent_service: parent_span.service_name.clone(),
                                child_service: span.service_name.clone(),
                                call_count: 0,
                                error_count: 0,
                                avg_duration_us: 0.0,
                            });
                            dep.call_count += 1;
                            if span.status == SpanStatus::Error {
                                dep.error_count += 1;
                            }
                        }
                    }
                }
            }
        }

        dependencies.into_values().collect()
    }

    pub async fn update_sampling_config(&self, config: SamplingConfig) {
        let mut current = self.sampling_config.write().await;
        *current = config;
    }

    pub async fn update_exporter_config(&self, config: ExporterConfig) {
        let mut current = self.exporter_config.write().await;
        *current = config;
    }

    pub async fn flush(&self) {
        let mut buffer = self.span_buffer.write().await;
        let spans_to_export: Vec<Span> = buffer.drain(..).collect();
        drop(buffer);

        if spans_to_export.is_empty() {
            return;
        }

        let exporter_config = self.exporter_config.read().await;
        if exporter_config.enabled {
            for span in spans_to_export {
                log::debug!("Exporting span: {} ({})", span.operation_name, span.span_id);
                let _ = span;
            }
        }
    }

    pub(super) fn percentile(&self, sorted_data: &[i64], p: u8) -> i64 {
        if sorted_data.is_empty() {
            return 0;
        }
        let idx = ((p as f64 / 100.0) * (sorted_data.len() as f64 - 1.0)).round() as usize;
        sorted_data[idx.min(sorted_data.len() - 1)]
    }
}
