use super::*;

impl SearchConfig {
    pub fn default_config() -> Self {
        Self {
            enabled_sources: vec![
                SearchSource::Email,
                SearchSource::Drive,
                SearchSource::Calendar,
                SearchSource::Tasks,
                SearchSource::Transcription,
            ],
            max_results_per_source: 50,
            snippet_length: 200,
            use_tantivy: false,
            postgres_fts_config: "english".to_string(),
            index_retention_days: 365,
        }
    }
}
