use super::*;

impl From<&[Recommendation]> for AdvisorSummary {
    fn from(recs: &[Recommendation]) -> Self {
        Self {
            total: recs.len(),
            critical: recs
                .iter()
                .filter(|r| r.severity == RecommendationSeverity::Critical)
                .count(),
            warnings: recs
                .iter()
                .filter(|r| r.severity == RecommendationSeverity::Warning)
                .count(),
            info: recs
                .iter()
                .filter(|r| r.severity == RecommendationSeverity::Info)
                .count(),
        }
    }
}
