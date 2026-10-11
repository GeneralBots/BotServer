use chrono::{DateTime, Duration, Utc};

use std::collections::HashMap;


use uuid::Uuid;

use super::soc2::{
    CategoryStats,
    ControlTestResult,
    Evidence,
    Finding,
    FindingSeverity,
    FindingStatus,
    ImplementationStatus,
    IncidentRecord,
    IncidentStatus,
    IncidentTimelineEntry,
    Soc2ComplianceReport,
    Soc2ComplianceService,
    Soc2Control,
    SopLog,
    TestFrequency,
    TestResult,
    TrustServiceCategory,
};

impl Soc2ComplianceService {
    pub async fn get_control(&self, control_id: &str) -> Option<Soc2Control> {
        let controls = self.controls.read().await;
        controls.get(control_id).cloned()
    }

    pub async fn get_all_controls(&self) -> Vec<Soc2Control> {
        let controls = self.controls.read().await;
        controls.values().cloned().collect()
    }

    pub async fn get_controls_by_category(&self, category: TrustServiceCategory) -> Vec<Soc2Control> {
        let controls = self.controls.read().await;
        controls
            .values()
            .filter(|c| c.category == category)
            .cloned()
            .collect()
    }

    pub async fn record_test_result(&self, result: ControlTestResult) {
        let mut test_results = self.test_results.write().await;
        test_results.push(result.clone());

        let mut controls = self.controls.write().await;
        if let Some(control) = controls.get_mut(&result.control_id) {
            control.last_tested = Some(result.tested_at);
            control.next_test_due = Some(self.calculate_next_test_due(control.test_frequency));
        }
    }

    pub async fn get_test_results(&self, control_id: &str) -> Vec<ControlTestResult> {
        let test_results = self.test_results.read().await;
        test_results
            .iter()
            .filter(|r| r.control_id == control_id)
            .cloned()
            .collect()
    }

    pub async fn log_sop_execution(&self, sop_log: SopLog) {
        let mut logs = self.sop_logs.write().await;
        logs.push(sop_log);
    }

    pub async fn get_sop_logs(&self, procedure_id: Option<&str>, limit: usize) -> Vec<SopLog> {
        let logs = self.sop_logs.read().await;
        let filtered: Vec<SopLog> = match procedure_id {
            Some(id) => logs.iter().filter(|l| l.procedure_id == id).cloned().collect(),
            None => logs.iter().cloned().collect(),
        };
        filtered.into_iter().rev().take(limit).collect()
    }

    pub async fn record_incident(&self, incident: IncidentRecord) {
        let mut incidents = self.incidents.write().await;
        incidents.push(incident);
    }

    pub async fn update_incident(&self, incident_id: Uuid, status: IncidentStatus, action: &str, performed_by: &str) {
        let mut incidents = self.incidents.write().await;
        if let Some(incident) = incidents.iter_mut().find(|i| i.id == incident_id) {
            incident.status = status;
            incident.timeline.push(IncidentTimelineEntry {
                timestamp: Utc::now(),
                action: action.to_string(),
                performed_by: performed_by.to_string(),
                notes: None,
            });
            if status == IncidentStatus::Closed || status == IncidentStatus::Resolved {
                incident.resolved_at = Some(Utc::now());
            }
        }
    }

    pub async fn get_open_incidents(&self) -> Vec<IncidentRecord> {
        let incidents = self.incidents.read().await;
        incidents
            .iter()
            .filter(|i| !matches!(i.status, IncidentStatus::Closed | IncidentStatus::Resolved))
            .cloned()
            .collect()
    }

    pub async fn get_incidents_by_severity(&self, severity: FindingSeverity) -> Vec<IncidentRecord> {
        let incidents = self.incidents.read().await;
        incidents
            .iter()
            .filter(|i| i.severity == severity)
            .cloned()
            .collect()
    }

    pub async fn store_evidence(&self, evidence: Evidence) {
        let mut store = self.evidence_store.write().await;
        store.push(evidence);
    }

    pub async fn get_evidence_for_control(&self, control_id: &str) -> Vec<Evidence> {
        let test_results = self.test_results.read().await;
        let evidence_ids: Vec<Uuid> = test_results
            .iter()
            .filter(|r| r.control_id == control_id)
            .flat_map(|r| r.evidence_collected.iter().map(|e| e.id))
            .collect();

        let store = self.evidence_store.read().await;
        store
            .iter()
            .filter(|e| evidence_ids.contains(&e.id))
            .cloned()
            .collect()
    }

    pub async fn generate_compliance_report(&self) -> Soc2ComplianceReport {
        let controls = self.controls.read().await;
        let test_results = self.test_results.read().await;
        let incidents = self.incidents.read().await;

        let total_controls = controls.len();
        let implemented = controls
            .values()
            .filter(|c| c.implementation_status == ImplementationStatus::FullyImplemented)
            .count();
        let partially_implemented = controls
            .values()
            .filter(|c| c.implementation_status == ImplementationStatus::PartiallyImplemented)
            .count();

        let recent_tests: Vec<ControlTestResult> = test_results
            .iter()
            .rev()
            .take(50)
            .cloned()
            .collect();

        let passed_tests = recent_tests.iter().filter(|r| r.result == TestResult::Pass).count();
        let failed_tests = recent_tests.iter().filter(|r| r.result == TestResult::Fail).count();

        let open_findings: Vec<Finding> = recent_tests
            .iter()
            .flat_map(|r| r.findings.iter())
            .filter(|f| f.status == FindingStatus::Open || f.status == FindingStatus::InProgress)
            .cloned()
            .collect();

        let open_incidents = incidents
            .iter()
            .filter(|i| !matches!(i.status, IncidentStatus::Closed | IncidentStatus::Resolved))
            .count();

        let compliance_score = if total_controls > 0 {
            ((implemented as f64 + (partially_implemented as f64 * 0.5)) / total_controls as f64) * 100.0
        } else {
            0.0
        };

        Soc2ComplianceReport {
            generated_at: Utc::now(),
            report_period_start: Utc::now() - Duration::days(365),
            report_period_end: Utc::now(),
            total_controls,
            implemented_controls: implemented,
            partially_implemented_controls: partially_implemented,
            not_implemented_controls: total_controls - implemented - partially_implemented,
            compliance_score,
            tests_performed: recent_tests.len(),
            tests_passed: passed_tests,
            tests_failed: failed_tests,
            open_findings: open_findings.len(),
            critical_findings: open_findings.iter().filter(|f| f.severity == FindingSeverity::Critical).count(),
            high_findings: open_findings.iter().filter(|f| f.severity == FindingSeverity::High).count(),
            open_incidents,
            controls_by_category: self.count_by_category(&controls),
        }
    }

    fn count_by_category(&self, controls: &HashMap<String, Soc2Control>) -> HashMap<TrustServiceCategory, CategoryStats> {
        let mut stats: HashMap<TrustServiceCategory, CategoryStats> = HashMap::new();

        for control in controls.values() {
            let entry = stats.entry(control.category).or_insert(CategoryStats {
                total: 0,
                implemented: 0,
                partially_implemented: 0,
                not_implemented: 0,
            });
            entry.total += 1;
            match control.implementation_status {
                ImplementationStatus::FullyImplemented => entry.implemented += 1,
                ImplementationStatus::PartiallyImplemented => entry.partially_implemented += 1,
                ImplementationStatus::NotImplemented => entry.not_implemented += 1,
                ImplementationStatus::NotApplicable => {}
            }
        }

        stats
    }

    fn calculate_next_test_due(&self, frequency: TestFrequency) -> DateTime<Utc> {
        let now = Utc::now();
        match frequency {
            TestFrequency::Continuous => now,
            TestFrequency::Daily => now + Duration::days(1),
            TestFrequency::Weekly => now + Duration::weeks(1),
            TestFrequency::Monthly => now + Duration::days(30),
            TestFrequency::Quarterly => now + Duration::days(90),
            TestFrequency::Annually => now + Duration::days(365),
        }
    }

    pub async fn get_overdue_tests(&self) -> Vec<Soc2Control> {
        let controls = self.controls.read().await;
        let now = Utc::now();
        controls
            .values()
            .filter(|c| c.next_test_due.map(|d| d < now).unwrap_or(false))
            .cloned()
            .collect()
    }
}
