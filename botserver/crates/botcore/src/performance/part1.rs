use super::*;

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            default_ttl_seconds: 300,
            cleanup_interval_seconds: 60,
            enable_statistics: true,
            compression_threshold_bytes: Some(1024),
            region_configs: HashMap::new(),
        }
    }
}
