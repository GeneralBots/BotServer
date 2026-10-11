#[cfg(test)]
mod tests {
use super::super::*;


#[test]
pub(super) fn test_memory_stats() {
    let stats = MemoryStats::current();
    assert!(stats.rss_bytes > 0 || stats.virtual_bytes > 0);
}

#[test]
pub(super) fn test_format_bytes() {
    assert_eq!(MemoryStats::format_bytes(500), "500 B");
    assert_eq!(MemoryStats::format_bytes(1024), "1.00 KB");
    assert_eq!(MemoryStats::format_bytes(1024 * 1024), "1.00 MB");
    assert_eq!(MemoryStats::format_bytes(1024 * 1024 * 1024), "1.00 GB");
}

#[test]
pub(super) fn test_checkpoint() {
    let checkpoint = MemoryCheckpoint::new("test");
    checkpoint.compare_and_log();
}

#[test]
pub(super) fn test_thread_registry() {
    register_thread("test-thread", "test-component");
    record_thread_activity("test-thread");
    log_thread_stats();
    unregister_thread("test-thread");
}

}
