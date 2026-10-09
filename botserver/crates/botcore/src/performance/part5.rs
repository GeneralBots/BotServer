use super::*;

impl Default for QueryOptimizationHint {
    fn default() -> Self {
        Self {
            use_index: None,
            limit: Some(100),
            offset: None,
            order_by: None,
            select_fields: None,
            include_count: false,
            use_cache: true,
            cache_ttl_seconds: Some(60),
        }
    }
}
