use super::{
    check_llm, check_minio, check_postgres, check_redis, MonitoringState,

};
use axum::{extract::State, response::Html};
use chrono::Local;
use std::sync::Arc;

pub(super) async fn timestamp<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    let now = Local::now();
    Html(format!("Last updated: {}", now.format("%H:%M:%S")))
}

pub(super) async fn bots<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let active_sessions = state.active_session_count();

    Html(format!(
        r##"<div class="bots-list">
  <div class="bot-item">
    <span class="bot-name">Active Sessions</span>
    <span class="bot-count">{active_sessions}</span>
  </div>
</div>"##
    ))
}

pub(super) async fn services_status<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    let services = vec![
        ("postgresql", check_postgres()),
        ("redis", check_redis()),
        ("minio", check_minio()),
        ("llm", check_llm()),
    ];

    let mut status_updates = String::new();
    for (name, running) in services {
        let status = if running { "running" } else { "stopped" };
        status_updates.push_str(&format!(
            r##"<script>
(function() {{
  var el = document.querySelector('[data-service="{name}"]');
  if (el) el.setAttribute('data-status', '{status}');
}})();
</script>"##
        ));
    }

    Html(status_updates)
}

pub(super) async fn resources_bars<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let (cpu_usage, memory_percent) = {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_percent = if total_memory > 0 {
            ((used_memory as f64 / total_memory as f64) * 100.0) as f32
        } else {
            0.0
        };

        (cpu_usage, memory_percent)
    };

    #[cfg(not(feature = "monitoring"))]
    let (cpu_usage, memory_percent): (f32, f32) = (0.0, 0.0);

    Html(format!(
        r##"<g>
  <text x="0" y="0" fill="#94a3b8" font-family="system-ui" font-size="10">CPU</text>
  <rect x="40" y="-8" width="100" height="10" rx="2" fill="#1e293b"/>
  <rect x="40" y="-8" width="{cpu_width}" height="10" rx="2" fill="#3b82f6"/>
  <text x="150" y="0" fill="#f8fafc" font-family="system-ui" font-size="10">{cpu_usage:.0}%</text>
</g> <g transform="translate(0, 20)"> <text x="0" y="0" fill="#94a3b8" font-family="system-ui" font-size="10">MEM</text> <rect x="40" y="-8" width="100" height="10" rx="2" fill="#1e293b"/> <rect x="40" y="-8" width="{mem_width}" height="10" rx="2" fill="#10b981"/> <text x="150" y="0" fill="#f8fafc" font-family="system-ui" font-size="10">{memory_percent:.0}%</text> </g>"##,
        cpu_width = cpu_usage.min(100.0f32),
        mem_width = memory_percent.min(100.0f32),
    ))
}

pub(super) async fn activity_latest<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let active = state.active_session_count();
    let collector = state.metrics_collector();
    let health = collector.get_health().await;
    Html(format!(
        "Active sessions: {active} · Requests/s: {rps:.1} · Error rate: {err:.1}%",
        rps = health.requests_per_second,
        err = health.error_rate_percent,
    ))
}

pub(super) async fn metric_sessions<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let active_sessions = state.active_session_count();
    Html(active_sessions.to_string())
}

pub(super) async fn metric_messages<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let collector = state.metrics_collector();
    let count = collector
        .get_metrics()
        .await
        .into_iter()
        .find(|m| m.name == "messages_total")
        .map(|m| m.current_value as u64)
        .unwrap_or(0);
    Html(count.to_string())
}

pub(super) async fn metric_response_time<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let collector = state.metrics_collector();
    let health = collector.get_health().await;
    Html(format!("{:.0} ms", health.average_latency_ms))
}

pub(super) async fn trend_sessions<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let active = state.active_session_count();
    let collector = state.metrics_collector();
    let previous = collector
        .get_metrics()
        .await
        .into_iter()
        .find(|m| m.name == "sessions_total")
        .map(|m| m.current_value as usize)
        .unwrap_or(0);

    if previous == 0 {
        return Html(if active > 0 { "↑ new".to_string() } else { "—".to_string() });
    }

    let delta = if active >= previous {
        active as i64 - previous as i64
    } else {
        previous as i64 - active as i64
    };
    let pct = (delta as f64 / previous as f64) * 100.0;
    if active >= previous {
        Html(format!("↑ {pct:.0}%"))
    } else {
        Html(format!("↓ {pct:.0}%"))
    }
}

pub(super) async fn rate_messages<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let collector = state.metrics_collector();
    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::minutes(60);
    let recent = collector
        .get_metrics()
        .await
        .into_iter()
        .filter(|m| m.name == "messages_total")
        .flat_map(|m| m.data_points)
        .filter(|p| p.timestamp >= cutoff)
        .count();

    Html(format!("{recent}/hr"))
}

pub(super) async fn sessions_panel<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let active_sessions = state.active_session_count();

    let sessions_list = if active_sessions > 0 {
        format!(r#"<div class="session-item"><span class="session-name">Active conversations</span><span class="session-value">{active_sessions}</span></div>"#)
    } else {
        r#"<div class="empty-state"><p>No active sessions</p></div>"#.to_string()
    };

    Html(format!(
        r##"<div class="sessions-panel">
  <div class="panel-header">
    <h3>Active Sessions</h3>
    <span class="session-count">{active_sessions}</span>
  </div>
  <div class="session-list">
    {sessions_list}
  </div>
</div>"##
    ))
}

pub(super) async fn messages_panel<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let collector = state.metrics_collector();
    let messages = collector
        .get_metrics()
        .await
        .into_iter()
        .find(|m| m.name == "messages_total")
        .map(|m| m.current_value as u64)
        .unwrap_or(0);

    Html(format!(
        r##"<div class="messages-panel">
  <div class="panel-header">
    <h3>Recent Messages</h3>
  </div>
  <div class="message-list">
    <div class="message-stats">
      <div class="stat-row"><span>Total messages</span><span>{messages}</span></div>
    </div>
  </div>
</div>"##
    ))
}

// Quick stats bar handlers — returns just the stat value for innerHTML swap
