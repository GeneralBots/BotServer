use super::*;

impl Default for ExporterConfig {
    fn default() -> Self {
        Self {
            exporter_type: ExporterType::Otlp,
            endpoint: String::new(),
            headers: HashMap::new(),
            batch_size: 512,
            flush_interval_ms: 5000,
            timeout_ms: 10000,
            compression: true,
            enabled: true,
        }
    }
}
