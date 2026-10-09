use super::*;

pub struct ComponentMemoryTracker {
components: Mutex<HashMap<String, Vec<MemoryStats>>>,
max_history: usize,
}

impl ComponentMemoryTracker {
pub fn new(max_history: usize) -> Self {
Self {
components: Mutex::new(HashMap::new()),
max_history,
}
}


pub fn record(&self, component: &str) {
    let stats = MemoryStats::current();
    if let Ok(mut components) = self.components.lock() {
        let history = components.entry(component.to_string()).or_default();
        history.push(stats);

        if history.len() > self.max_history {
            history.remove(0);
        }
    }
}

pub fn get_growth_rate(&self, component: &str) -> Option<f64> {
    if let Ok(components) = self.components.lock() {
        if let Some(history) = components.get(component) {
            if history.len() >= 2 {
                let first = &history[0];
                let last = &history[history.len() - 1];
                let duration = last.timestamp.duration_since(first.timestamp).as_secs_f64();
                if duration > 0.0 {
                    let byte_diff = last.rss_bytes as f64 - first.rss_bytes as f64;
                    return Some(byte_diff / duration);
                }
            }
        }
    }
    None
}

pub fn log_all(&self) {
    if let Ok(components) = self.components.lock() {
        for (name, history) in components.iter() {
            if let Some(last) = history.last() {
                let growth = if history.len() >= 2 {
                    let first = &history[0];
                    let duration = last.timestamp.duration_since(first.timestamp).as_secs_f64();
                    if duration > 0.0 {
                        let byte_diff = last.rss_bytes as f64 - first.rss_bytes as f64;
                        Some(byte_diff / duration)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let growth_str = growth
                    .map(|g| {
                        let sign = if g >= 0.0 { "+" } else { "-" };
                        format!("{}{}/s", sign, MemoryStats::format_bytes(g.abs() as u64))
                    })
                    .unwrap_or_else(|| "N/A".to_string());
                info!(
                    "[COMPONENT] {} | RSS={} | Growth={}",
                    name,
                    MemoryStats::format_bytes(last.rss_bytes),
                    growth_str
                );
            }
        }
    }
}

}

pub fn record_component(component: &str) {
COMPONENT_TRACKER.record(component);
}

pub fn log_component_stats() {
COMPONENT_TRACKER.log_all();
}
