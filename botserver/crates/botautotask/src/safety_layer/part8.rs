use super::*;

impl Default for SecurityImpact {
    fn default() -> Self {
        Self {
            risk_level: RiskLevel::Low,
            credentials_accessed: Vec::new(),
            external_systems: Vec::new(),
            data_exposure_risk: false,
            requires_elevation: false,
            concerns: Vec::new(),
        }
    }
}
