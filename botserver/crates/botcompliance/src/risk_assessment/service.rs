use super::*;

impl RiskAssessmentService {
    pub fn new() -> Self {
        Self {
            assessments: HashMap::new(),
            mitigation_plans: HashMap::new(),
            risk_matrix: RiskMatrix::default(),
        }
    }

    pub fn create_assessment(
        &mut self,
        title: String,
        description: String,
        category: RiskCategory,
        owner: String,
    ) -> Result<RiskAssessment> {
        let assessment = RiskAssessment {
            id: Uuid::new_v4(),
            title,
            description,
            category,
            likelihood: Likelihood::Possible,
            impact: Impact::Moderate,
            risk_level: RiskLevel::Medium,
            status: RiskStatus::Identified,
            identified_date: Utc::now(),
            assessed_date: None,
            owner,
            affected_assets: Vec::new(),
            vulnerabilities: Vec::new(),
            threats: Vec::new(),
            controls: Vec::new(),
            residual_risk: None,
        };

        self.assessments.insert(assessment.id, assessment.clone());
        log::info!("Created risk assessment: {}", assessment.id);

        Ok(assessment)
    }

    pub fn assess_risk(
        &mut self,
        risk_id: Uuid,
        likelihood: Likelihood,
        impact: Impact,
    ) -> Result<RiskLevel> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        assessment.likelihood = likelihood.clone();
        assessment.impact = impact.clone();
        assessment.risk_level = self.risk_matrix.calculate_risk_level(&likelihood, &impact);
        assessment.assessed_date = Some(Utc::now());
        assessment.status = RiskStatus::Assessed;

        log::info!(
            "Assessed risk {}: level = {:?}",
            risk_id,
            assessment.risk_level
        );

        Ok(assessment.risk_level.clone())
    }

    pub fn add_vulnerability(&mut self, risk_id: Uuid, vulnerability: Vulnerability) -> Result<()> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        assessment.vulnerabilities.push(vulnerability);
        self.recalculate_risk_level(risk_id)?;

        Ok(())
    }

    pub fn add_threat(&mut self, risk_id: Uuid, threat: Threat) -> Result<()> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        assessment.threats.push(threat);
        self.recalculate_risk_level(risk_id)?;

        Ok(())
    }

    pub fn add_control(&mut self, risk_id: Uuid, control: Control) -> Result<()> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        assessment.controls.push(control);
        self.calculate_residual_risk(risk_id)?;

        Ok(())
    }

    pub(super) fn recalculate_risk_level(&mut self, risk_id: Uuid) -> Result<()> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        if !assessment.threats.is_empty() {
            let max_threat_likelihood = assessment
                .threats
                .iter()
                .map(|t| &t.likelihood)
                .max()
                .cloned()
                .unwrap_or(Likelihood::Possible);

            if max_threat_likelihood > assessment.likelihood {
                assessment.likelihood = max_threat_likelihood;
            }
        }

        if !assessment.vulnerabilities.is_empty() {
            let critical_vulns = assessment
                .vulnerabilities
                .iter()
                .filter(|v| v.severity == RiskLevel::Critical)
                .count();

            if critical_vulns > 0 && assessment.impact < Impact::Major {
                assessment.impact = Impact::Major;
            }
        }

        assessment.risk_level = self
            .risk_matrix
            .calculate_risk_level(&assessment.likelihood, &assessment.impact);

        Ok(())
    }

    pub(super) fn calculate_residual_risk(&mut self, risk_id: Uuid) -> Result<()> {
        let assessment = self
            .assessments
            .get_mut(&risk_id)
            .ok_or_else(|| anyhow!("Risk assessment not found"))?;

        if assessment.controls.is_empty() {
            assessment.residual_risk = Some(assessment.risk_level.clone());
            return Ok(());
        }

        let effective_controls = assessment
            .controls
            .iter()
            .filter(|c| {
                c.effectiveness == Effectiveness::Effective
                    || c.effectiveness == Effectiveness::HighlyEffective
            })
            .count();

        let residual = match (assessment.risk_level.clone(), effective_controls) {
            (RiskLevel::Critical, n) if n >= 3 => RiskLevel::High,
            (RiskLevel::Critical, n) if n >= 1 => RiskLevel::Critical,
            (RiskLevel::High, n) if n >= 2 => RiskLevel::Medium,
            (RiskLevel::High, n) if n >= 1 => RiskLevel::High,
            (RiskLevel::Medium, n) if n >= 1 => RiskLevel::Low,
            (level, _) => level,
        };

        assessment.residual_risk = Some(residual);

        Ok(())
    }

    pub fn create_mitigation_plan(
        &mut self,
        risk_id: Uuid,
        strategy: MitigationStrategy,
        timeline: Duration,
        budget: f64,
        responsible_party: String,
    ) -> Result<MitigationPlan> {
        if !self.assessments.contains_key(&risk_id) {
            return Err(anyhow!("Risk assessment not found"));
        }

        let plan = MitigationPlan {
            id: Uuid::new_v4(),
            risk_id,
            strategy,
            actions: Vec::new(),
            timeline,
            budget,
            responsible_party,
            approval_status: ApprovalStatus::Pending,
        };

        self.mitigation_plans.insert(plan.id, plan.clone());
        log::info!("Created mitigation plan {} for risk {}", plan.id, risk_id);

        Ok(plan)
    }

    pub fn get_high_risk_assessments(&self) -> Vec<RiskAssessment> {
        self.assessments
            .values()
            .filter(|a| a.risk_level >= RiskLevel::High)
            .cloned()
            .collect()
    }

    pub fn get_risk_dashboard(&self) -> RiskDashboard {
        let total_risks = self.assessments.len();
        let mut risks_by_level = HashMap::new();
        let mut risks_by_category = HashMap::new();
        let mut risks_by_status = HashMap::new();

        for assessment in self.assessments.values() {
            *risks_by_level
                .entry(assessment.risk_level.clone())
                .or_insert(0) += 1;
            *risks_by_category
                .entry(assessment.category.clone())
                .or_insert(0) += 1;
            *risks_by_status
                .entry(assessment.status.clone())
                .or_insert(0) += 1;
        }

        let mitigation_plans_pending = self
            .mitigation_plans
            .values()
            .filter(|p| p.approval_status == ApprovalStatus::Pending)
            .count();

        RiskDashboard {
            total_risks,
            risks_by_level,
            risks_by_category,
            risks_by_status,
            mitigation_plans_pending,
            last_updated: Utc::now(),
        }
    }
}
