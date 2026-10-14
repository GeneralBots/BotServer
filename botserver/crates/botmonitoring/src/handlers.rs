use super::{
    check_llm, check_minio, check_postgres, check_redis, HealthStatus, MonitoringState,
};
use axum::{extract::State, response::Html};
use std::sync::Arc;

pub(super) async fn dashboard<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let (cpu_usage, total_memory, used_memory, memory_percent, uptime_str) = {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();

        let cpu_usage = sys.global_cpu_usage();
        let total_memory = sys.total_memory();
        let used_memory = sys.used_memory();
        let memory_percent = if total_memory > 0 {
            (used_memory as f64 / total_memory as f64) * 100.0
        } else {
            0.0
        };

        let uptime = System::uptime();
        let uptime_str = format_uptime(uptime);

        (cpu_usage, total_memory, used_memory, memory_percent, uptime_str)
    };

    #[cfg(not(feature = "monitoring"))]
    let (cpu_usage, total_memory, used_memory, memory_percent, uptime_str) = (
        0.0f32, 0u64, 0u64, 0.0f64, "N/A".to_string()
    );

    let active_sessions = state.active_session_count();

    Html(format!(
        r##"<div class="dashboard-grid">
  <div class="metric-card">
    <div class="metric-header">
      <span class="metric-title">CPU Usage</span>
      <span class="metric-badge {cpu_status}">{cpu_usage:.1}%</span>
    </div>
    <div class="metric-value">{cpu_usage:.1}%</div>
    <div class="metric-bar">
      <div class="metric-bar-fill" style="width: {cpu_usage}%"></div>
    </div>
  </div>

  <div class="metric-card">
    <div class="metric-header">
      <span class="metric-title">Memory</span>
      <span class="metric-badge {mem_status}">{memory_percent:.1}%</span>
    </div>
    <div class="metric-value">{used_gb:.1} GB / {total_gb:.1} GB</div>
    <div class="metric-bar">
      <div class="metric-bar-fill" style="width: {memory_percent}%"></div>
    </div>
  </div>

  <div class="metric-card">
    <div class="metric-header">
      <span class="metric-title">Active Sessions</span>
    </div>
    <div class="metric-value">{active_sessions}</div>
    <div class="metric-subtitle">Current conversations</div>
  </div>

  <div class="metric-card">
    <div class="metric-header">
      <span class="metric-title">Uptime</span>
    </div>
    <div class="metric-value">{uptime_str}</div>
    <div class="metric-subtitle">System running time</div>
  </div>
</div><div class="refresh-indicator" hx-get="/api/monitoring/dashboard" hx-trigger="every 10s" hx-swap="outerHTML" hx-target="closest .dashboard-grid, .refresh-indicator"> <span class="refresh-dot"></span> Auto-refreshing </div>"##,
        cpu_status = if cpu_usage > 80.0 { "danger" } else if cpu_usage > 60.0 { "warning" } else { "success" },
        mem_status = if memory_percent > 80.0 { "danger" } else if memory_percent > 60.0 { "warning" } else { "success" },
        used_gb = used_memory as f64 / 1_073_741_824.0,
        total_gb = total_memory as f64 / 1_073_741_824.0,
    ))
}

pub(super) async fn services<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    let services = vec![
        ("PostgreSQL", check_postgres(), "Database"),
        ("Redis", check_redis(), "Cache"),
        ("MinIO", check_minio(), "Storage"),
        ("LLM Server", check_llm(), "AI Backend"),
    ];

    let mut rows = String::new();
    for (name, status, desc) in services {
        let (status_class, status_text) = if status {
            ("success", "Running")
        } else {
            ("danger", "Stopped")
        };

        rows.push_str(&format!(
            r##"<tr>
  <td>
    <div class="service-name">
      <span class="status-dot {status_class}"></span>
      {name}
    </div>
  </td>
  <td>{desc}</td>
  <td><span class="status-badge {status_class}">{status_text}</span></td>
  <td>
    <span class="status-badge {status_class}">{status_text}</span>
  </td>
</tr>"##,
        ));
    }

    Html(format!(
        r##"<div class="services-view">
  <div class="section-header">
    <h2>Services Status</h2>
    <button class="btn-secondary" hx-get="/api/monitoring/services" hx-target="#monitoring-content" hx-swap="innerHTML">
      Refresh
    </button>
  </div>
  <table class="data-table">
    <thead>
      <tr>
        <th>Service</th>
        <th>Description</th>
        <th>Status</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>
      {rows}
    </tbody>
  </table>
</div>"##
    ))
}

pub(super) async fn resources<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let (disk_rows, net_rows) = {
        use sysinfo::{Disks, Networks, System};
        let mut sys = System::new_all();
        sys.refresh_all();

        let disks = Disks::new_with_refreshed_list();
        let mut disk_rows = String::new();

        for disk in disks.list() {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total - available;
            let percent = if total > 0 {
                (used as f64 / total as f64) * 100.0
            } else {
                0.0
            };

            disk_rows.push_str(&format!(
                r##"<tr>
  <td>{mount}</td>
  <td>{used_gb:.1} GB</td>
  <td>{total_gb:.1} GB</td>
  <td>
    <div class="usage-bar">
      <div class="usage-fill {status}" style="width: {percent:.0}%"></div>
    </div>
    <span class="usage-text">{percent:.1}%</span>
  </td>
</tr>"##,
                mount = disk.mount_point().display(),
                used_gb = used as f64 / 1_073_741_824.0,
                total_gb = total as f64 / 1_073_741_824.0,
                status = if percent > 90.0 { "danger" } else if percent > 70.0 { "warning" } else { "success" },
            ));
        }
        let networks = Networks::new_with_refreshed_list();
        let mut net_rows = String::new();

        for (name, data) in networks.list() {
            net_rows.push_str(&format!(
                r##"<tr>
  <td>{name}</td>
  <td>{rx:.2} MB</td>
  <td>{tx:.2} MB</td>
</tr>"##,
                rx = data.total_received() as f64 / 1_048_576.0,
                tx = data.total_transmitted() as f64 / 1_048_576.0,
            ));
        }
        (disk_rows, net_rows)
    };

    #[cfg(not(feature = "monitoring"))]
    let (disk_rows, net_rows) = (
        String::new(),
        String::new()
    );

    Html(format!(
        r##"<div class="resources-view">
  <div class="section-header">
    <h2>System Resources</h2>
  </div>

  <div class="resource-section">
    <h3>Disk Usage</h3>
    <table class="data-table">
      <thead>
        <tr>
          <th>Mount</th>
          <th>Used</th>
          <th>Total</th>
          <th>Usage</th>
        </tr>
      </thead>
      <tbody>
        {disk_rows}
      </tbody>
    </table>
  </div>

  <div class="resource-section">
    <h3>Network</h3>
    <table class="data-table">
      <thead>
        <tr>
          <th>Interface</th>
          <th>Received</th>
          <th>Transmitted</th>
        </tr>
      </thead>
      <tbody>
        {net_rows}
      </tbody>
    </table>
  </div>
</div>"##
    ))
}

pub(super) async fn logs<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    Html(
        r##"<div class="logs-view">
  <div class="section-header">
    <h2>System Logs</h2>
    <div class="log-controls">
      <select id="log-level" onchange="filterLogs(this.value)">
        <option value="all">All Levels</option>
        <option value="error">Error</option>
        <option value="warn">Warning</option>
        <option value="info">Info</option>
        <option value="debug">Debug</option>
      </select>
      <button class="btn-secondary" onclick="clearLogs()">Clear</button>
    </div>
  </div>
  <div class="log-container" id="log-container"
    hx-get="/api/monitoring/logs/stream"
    hx-trigger="every 2s"
    hx-swap="beforeend scroll:bottom">
    <div class="log-entry info">
      <span class="log-time">System ready</span>
      <span class="log-level">INFO</span>
      <span class="log-message">Monitoring initialized</span>
    </div>
  </div>

</div>"##.to_string(),
    )
}

pub(super) async fn llm_metrics<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    Html(
        r##"<div class="llm-metrics-view">
  <div class="section-header">
    <h2>LLM Metrics</h2>
  </div>

  <div class="metrics-grid">
    <div class="metric-card">
      <div class="metric-title">Total Requests</div>
      <div class="metric-value" id="llm-total-requests"
        hx-get="/api/monitoring/llm/total"
        hx-trigger="load, every 30s"
        hx-swap="innerHTML">
        --
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-title">Cache Hit Rate</div>
      <div class="metric-value" id="llm-cache-rate"
        hx-get="/api/monitoring/llm/cache-rate"
        hx-trigger="load, every 30s"
        hx-swap="innerHTML">
        --
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-title">Avg Latency</div>
      <div class="metric-value" id="llm-latency"
        hx-get="/api/monitoring/llm/latency"
        hx-trigger="load, every 30s"
        hx-swap="innerHTML">
        --
      </div>
    </div>

    <div class="metric-card">
      <div class="metric-title">Total Tokens</div>
      <div class="metric-value" id="llm-tokens"
        hx-get="/api/monitoring/llm/tokens"
        hx-trigger="load, every 30s"
        hx-swap="innerHTML">
        --
      </div>
    </div>
  </div>
</div>"##.to_string(),
    )
}

pub(super) async fn health<S: MonitoringState>(State(state): State<Arc<S>>) -> Html<String> {
    let collector = state.metrics_collector();
    let health = collector.get_health().await;
    let db_ok = state.is_db_healthy();
    let status = if health.status == HealthStatus::Healthy && db_ok {
        "healthy"
    } else if health.status == HealthStatus::Unhealthy || !db_ok {
        "unhealthy"
    } else {
        "degraded"
    };

    Html(format!(
        r##"<div class="health-status {status}">
  <span class="status-icon"></span>
  <span class="status-text">{status}</span>
  <span class="status-detail">CPU {cpu:.1}% · Mem {mem:.1}% · Disk {disk:.1}%</span>
</div>"##,
        cpu = health.cpu_usage_percent,
        mem = health.memory_usage_percent,
        disk = health.disk_usage_percent,
    ))
}

#[cfg(feature = "monitoring")]
pub(super) fn format_uptime(seconds: u64) -> String {
    format_uptime_duration(seconds)
}


