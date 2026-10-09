use super::*;

impl RiskMatrix {
    pub fn calculate_risk_level(&self, likelihood: &Likelihood, impact: &Impact) -> RiskLevel {
        self.matrix
            .get(&(likelihood.clone(), impact.clone()))
            .cloned()
            .unwrap_or(RiskLevel::Medium)
    }
}

impl Default for RiskMatrix {
    fn default() -> Self {
        let mut matrix = HashMap::new();

        matrix.insert((Likelihood::Rare, Impact::Negligible), RiskLevel::Low);
        matrix.insert((Likelihood::Rare, Impact::Minor), RiskLevel::Low);
        matrix.insert((Likelihood::Rare, Impact::Moderate), RiskLevel::Low);
        matrix.insert((Likelihood::Rare, Impact::Major), RiskLevel::Medium);
        matrix.insert((Likelihood::Rare, Impact::Catastrophic), RiskLevel::High);

        matrix.insert((Likelihood::Unlikely, Impact::Negligible), RiskLevel::Low);
        matrix.insert((Likelihood::Unlikely, Impact::Minor), RiskLevel::Low);
        matrix.insert((Likelihood::Unlikely, Impact::Moderate), RiskLevel::Medium);
        matrix.insert((Likelihood::Unlikely, Impact::Major), RiskLevel::High);
        matrix.insert(
            (Likelihood::Unlikely, Impact::Catastrophic),
            RiskLevel::High,
        );

        matrix.insert((Likelihood::Possible, Impact::Negligible), RiskLevel::Low);
        matrix.insert((Likelihood::Possible, Impact::Minor), RiskLevel::Medium);
        matrix.insert((Likelihood::Possible, Impact::Moderate), RiskLevel::Medium);
        matrix.insert((Likelihood::Possible, Impact::Major), RiskLevel::High);
        matrix.insert(
            (Likelihood::Possible, Impact::Catastrophic),
            RiskLevel::Critical,
        );

        matrix.insert((Likelihood::Likely, Impact::Negligible), RiskLevel::Medium);
        matrix.insert((Likelihood::Likely, Impact::Minor), RiskLevel::Medium);
        matrix.insert((Likelihood::Likely, Impact::Moderate), RiskLevel::High);
        matrix.insert((Likelihood::Likely, Impact::Major), RiskLevel::Critical);
        matrix.insert(
            (Likelihood::Likely, Impact::Catastrophic),
            RiskLevel::Critical,
        );

        matrix.insert(
            (Likelihood::AlmostCertain, Impact::Negligible),
            RiskLevel::Medium,
        );
        matrix.insert(
            (Likelihood::AlmostCertain, Impact::Minor),
            RiskLevel::High,
        );
        matrix.insert(
            (Likelihood::AlmostCertain, Impact::Moderate),
            RiskLevel::High,
        );
        matrix.insert(
            (Likelihood::AlmostCertain, Impact::Major),
            RiskLevel::Critical,
        );
        matrix.insert(
            (Likelihood::AlmostCertain, Impact::Catastrophic),
            RiskLevel::Critical,
        );

        Self { matrix }
    }
}
