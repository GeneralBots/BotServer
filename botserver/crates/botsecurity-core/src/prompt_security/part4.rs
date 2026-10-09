use super::*;

impl InjectionDetection {
    pub fn safe() -> Self {
        Self {
            detected: false,
            threat_level: ThreatLevel::None,
            injection_types: Vec::new(),
            matched_patterns: Vec::new(),
            sanitized_input: None,
            blocked: false,
        }
    }

    pub fn is_safe(&self) -> bool {
        !self.detected && !self.blocked
    }
}
