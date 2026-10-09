use super::*;

#[derive(Debug)]
pub struct MemoryCheckpoint {
pub name: String,
pub stats: MemoryStats,
}

impl MemoryCheckpoint {
pub fn new(name: &str) -> Self {
let stats = MemoryStats::current();
info!(
"[CHECKPOINT] {} started at RSS={}",
name,
MemoryStats::format_bytes(stats.rss_bytes)
);
Self {
name: name.to_string(),
stats,
}
}


pub fn compare_and_log(&self) {
    let current = MemoryStats::current();
    let diff = current.rss_bytes as i64 - self.stats.rss_bytes as i64;

    if diff > 0 {
        info!(
            "[CHECKPOINT] {} INCREASED by {}",
            self.name,
            MemoryStats::format_bytes(diff as u64),
        );
    } else if diff < 0 {
        info!(
            "[CHECKPOINT] {} decreased by {}",
            self.name,
            MemoryStats::format_bytes((-diff) as u64),
        );
    } else {
        debug!("[CHECKPOINT] {} unchanged", self.name);
    }
}

}
