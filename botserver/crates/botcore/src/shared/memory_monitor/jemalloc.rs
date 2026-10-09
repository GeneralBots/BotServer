use super::*;

/// Get mimalloc memory statistics when the feature is enabled
#[cfg(feature = "jemalloc")]
pub fn get_jemalloc_stats() -> Option<JemallocStats> {
    // mimalloc statistics (simplified - mimalloc doesn't expose detailed stats easily)
    // We use a basic approach since mimalloc's stats API is limited
    Some(JemallocStats {
        allocated: 0, // mimalloc doesn't expose this directly
        active: 0,    // mimalloc doesn't expose this directly
        resident: 0,  // mimalloc doesn't expose this directly
        mapped: 0,    // mimalloc doesn't expose this directly
        retained: 0,  // mimalloc doesn't expose this directly
    })
}

#[cfg(not(feature = "jemalloc"))]
pub fn get_jemalloc_stats() -> Option<JemallocStats> {
    None
}

/// Jemalloc memory statistics
#[derive(Debug, Clone)]
pub struct JemallocStats {
/// Total bytes allocated by the application
pub allocated: u64,
/// Total bytes in active pages allocated by the application
pub active: u64,
/// Total bytes in physically resident pages
pub resident: u64,
/// Total bytes in active extents mapped by the allocator
pub mapped: u64,
/// Total bytes retained (not returned to OS)
pub retained: u64,
}

impl JemallocStats {
pub fn log(&self) {
info!(
"[JEMALLOC] allocated={} active={} resident={} mapped={} retained={}",
MemoryStats::format_bytes(self.allocated),
MemoryStats::format_bytes(self.active),
MemoryStats::format_bytes(self.resident),
MemoryStats::format_bytes(self.mapped),
MemoryStats::format_bytes(self.retained),
);
}


/// Calculate fragmentation ratio (1.0 = no fragmentation)
pub fn fragmentation_ratio(&self) -> f64 {
    if self.allocated > 0 {
        self.active as f64 / self.allocated as f64
    } else {
        1.0
    }
}

}

/// Log mimalloc stats if available
pub fn log_jemalloc_stats() {
    // mimalloc stats are not as detailed as jemalloc
    // This is a placeholder for future mimalloc stats implementation
    trace!("[MIMALLOC] Stats collection not fully implemented");
}
