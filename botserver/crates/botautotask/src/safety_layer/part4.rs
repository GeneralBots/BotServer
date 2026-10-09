use super::*;

impl Default for ImpactAssessment {
    fn default() -> Self {
        Self {
            risk_score: 0.0,
            risk_level: RiskLevel::Low,
            data_impact: DataImpact::default(),
            cost_impact: CostImpact::default(),
            time_impact: TimeImpact::default(),
            security_impact: SecurityImpact::default(),
            summary: "No impact assessed".to_string(),
        }
    }
}
