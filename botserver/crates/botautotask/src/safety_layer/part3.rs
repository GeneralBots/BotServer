use super::*;

impl Default for SimulationResult {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            success: true,
            step_outcomes: Vec::new(),
            impact: ImpactAssessment::default(),
            resource_usage: PredictedResourceUsage::default(),
            side_effects: Vec::new(),
            recommendations: Vec::new(),
            confidence: 0.0,
            simulated_at: Utc::now(),
            simulation_duration_ms: 0,
        }
    }
}
