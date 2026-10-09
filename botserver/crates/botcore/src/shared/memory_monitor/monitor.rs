use super::*;

pub fn start_memory_monitor(interval_secs: u64, warn_threshold_mb: u64) {
let detector = LeakDetector::new(warn_threshold_mb, 5);


tokio::spawn(async move {
    register_thread("memory-monitor", "monitoring");

    info!(
        "[MONITOR] Started (interval={}s, threshold={}MB)",
        interval_secs, warn_threshold_mb
    );

    let mut prev_rss: u64 = 0;
    let mut tick_count: u64 = 0;

    // First 2 minutes: check every 10 seconds for aggressive tracking
    // After that: use normal interval
    let startup_interval = Duration::from_secs(10);
    let normal_interval = Duration::from_secs(interval_secs);
    let startup_ticks = 12; // 2 minutes of 10-second intervals

    let mut interval = tokio::time::interval(startup_interval);

    loop {
        interval.tick().await;
        tick_count += 1;
        record_thread_activity("memory-monitor");

        let stats = MemoryStats::current();
        let rss_diff = if prev_rss > 0 {
            stats.rss_bytes as i64 - prev_rss as i64
        } else {
            0
        };

        let diff_str = if rss_diff > 0 {
            format!("+{}", MemoryStats::format_bytes(rss_diff as u64))
        } else if rss_diff < 0 {
            format!("-{}", MemoryStats::format_bytes((-rss_diff) as u64))
        } else {
            "±0".to_string()
        };

        trace!(
            "[MONITOR] tick={} RSS={} ({}) Virtual={}",
            tick_count,
            MemoryStats::format_bytes(stats.rss_bytes),
            diff_str,
            MemoryStats::format_bytes(stats.virtual_bytes),
        );

        // Log jemalloc stats every 5 ticks if available
        if tick_count.is_multiple_of(5) {
            log_jemalloc_stats();
        }

        prev_rss = stats.rss_bytes;
        record_component("global");

        if let Some(warning) = detector.check() {
            info!("{}", warning);
            stats.log();
            log_component_stats();
            log_thread_stats();
        }

        // Switch to normal interval after startup period
        if tick_count == startup_ticks {
            trace!("Switching to normal interval ({}s)", interval_secs);
            interval = tokio::time::interval(normal_interval);
        }
    }
});

}

#[cfg(feature = "monitoring")]
pub fn get_process_memory() -> Option<(u64, u64)> {
let pid = Pid::from_u32(std::process::id());
let mut sys = System::new();
sys.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);


sys.process(pid).map(|p| (p.memory(), p.virtual_memory()))

}

#[cfg(not(feature = "monitoring"))]
pub fn get_process_memory() -> Option<(u64, u64)> {
None
}

pub fn log_process_memory() {
if let Some((rss, virt)) = get_process_memory() {
trace!(
"[PROCESS] RSS={}, Virtual={}",
MemoryStats::format_bytes(rss),
MemoryStats::format_bytes(virt)
);
}
}
