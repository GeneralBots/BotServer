use super::*;

impl UsageSnapshot {
    pub fn iter_metrics(&self) -> impl Iterator<Item = (UsageMetric, u64, u64)> {
        vec![
            (UsageMetric::Messages, self.messages.0, self.messages.1),
            (UsageMetric::StorageBytes, self.storage_bytes.0, self.storage_bytes.1),
            (UsageMetric::ApiCalls, self.api_calls.0, self.api_calls.1),
            (UsageMetric::Bots, self.bots.0, self.bots.1),
            (UsageMetric::Users, self.users.0, self.users.1),
            (UsageMetric::KbDocuments, self.kb_documents.0, self.kb_documents.1),
            (UsageMetric::Apps, self.apps.0, self.apps.1),
        ]
        .into_iter()
        .filter(|(_, _, limit)| *limit > 0)
    }
}
