use super::*;

impl Default for TraceQuery {
    fn default() -> Self {
        Self {
            service_name: None,
            operation_name: None,
            min_duration_us: None,
            max_duration_us: None,
            start_time: None,
            end_time: None,
            tags: HashMap::new(),
            has_errors: None,
            limit: 100,
            offset: 0,
        }
    }
}
