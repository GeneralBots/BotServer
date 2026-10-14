use super::MonitoringState;
use axum::{extract::State, response::Html};
use std::sync::Arc;

pub(super) async fn quick_cpu<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let cpu_usage = {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        sys.global_cpu_usage()
    };
    #[cfg(not(feature = "monitoring"))]
    let cpu_usage = 0.0f32;

    Html(format!(
        r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"></rect><rect x="9" y="9" width="6" height="6"></rect></svg>
  <span class="stat-label">CPU</span>
  <span class="stat-value">{cpu_usage:.0}%</span>"##
    ))
}

pub(super) async fn quick_memory<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let memory_percent = {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        let total = sys.total_memory();
        let used = sys.used_memory();
        if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 }
    };
    #[cfg(not(feature = "monitoring"))]
    let memory_percent = 0.0f64;

    Html(format!(
        r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="6" width="20" height="12" rx="2"></rect><line x1="6" y1="12" x2="6" y2="12"></line><line x1="10" y1="12" x2="10" y2="12"></line><line x1="14" y1="12" x2="14" y2="12"></line><line x1="18" y1="12" x2="18" y2="12"></line></svg>
  <span class="stat-label">Memory</span>
  <span class="stat-value">{memory_percent:.0}%</span>"##
    ))
}

pub(super) async fn quick_disk<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let disk_percent = {
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let total: u64 = disks.list().iter().map(|d| d.total_space()).sum();
        let available: u64 = disks.list().iter().map(|d| d.available_space()).sum();
        let used = total.saturating_sub(available);
        if total > 0 { (used as f64 / total as f64) * 100.0 } else { 0.0 }
    };
    #[cfg(not(feature = "monitoring"))]
    let disk_percent = 0.0f64;

    Html(format!(
        r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><ellipse cx="12" cy="5" rx="9" ry="3"></ellipse><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path></svg>
  <span class="stat-label">Disk</span>
  <span class="stat-value">{disk_percent:.0}%</span>"##
    ))
}

pub(super) async fn quick_network<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    #[cfg(feature = "monitoring")]
    let (rx, tx) = {
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let mut total_rx = 0u64;
        let mut total_tx = 0u64;
        for (_, data) in networks.list() {
            total_rx += data.total_received();
            total_tx += data.total_transmitted();
        }
        (total_rx, total_tx)
    };
    #[cfg(not(feature = "monitoring"))]
    let (rx, tx) = (0u64, 0u64);

    Html(format!(
        r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 12.55a11 11 0 0 1 14.08 0"></path><path d="M1.42 9a16 16 0 0 1 21.16 0"></path><path d="M8.53 16.11a6 6 0 0 1 6.95 0"></path><line x1="12" y1="20" x2="12.01" y2="20"></line></svg>
  <span class="stat-label">Network</span>
  <span class="stat-value">{rx_mb:.1}/{tx_mb:.1} MB</span>"##,
        rx_mb = rx as f64 / 1_048_576.0,
        tx_mb = tx as f64 / 1_048_576.0,
    ))
}

pub(super) async fn quick_requests<S: MonitoringState>(_state: State<Arc<S>>) -> Html<String> {
    Html(
        r##"<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>
  <span class="stat-label">Requests</span>
  <span class="stat-value">--/s</span>"##.to_string(),
    )
}
