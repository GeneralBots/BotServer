use super::*;

impl MemoryStats {
pub fn current() -> Self {
let (rss, virt) = get_process_memory().unwrap_or((0, 0));
Self {
rss_bytes: rss,
virtual_bytes: virt,
timestamp: Instant::now(),
}
}


pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn log(&self) {
    info!(
        "[MEMORY] RSS={}, Virtual={}",
        Self::format_bytes(self.rss_bytes),
        Self::format_bytes(self.virtual_bytes),
    );
}

}
