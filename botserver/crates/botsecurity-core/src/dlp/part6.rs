use super::*;

impl DlpScanResult {
    pub fn clean() -> Self {
        Self {
            matches: Vec::new(),
            severity: DlpSeverity::Low,
            action: DlpAction::Allow,
            blocked: false,
            redacted_content: None,
            policy_violations: Vec::new(),
        }
    }

    pub fn has_sensitive_data(&self) -> bool {
        !self.matches.is_empty()
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    pub fn data_types_found(&self) -> HashSet<SensitiveDataType> {
        self.matches.iter().map(|m| m.data_type).collect()
    }
}
