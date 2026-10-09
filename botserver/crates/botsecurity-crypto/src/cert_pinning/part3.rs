use super::*;

impl PinValidationResult {
    pub fn success(hostname: &str, fingerprint: &str, backup: bool) -> Self {
        Self {
            valid: true,
            hostname: hostname.to_string(),
            matched_pin: Some(fingerprint.to_string()),
            actual_fingerprint: fingerprint.to_string(),
            error: None,
            backup_match: backup,
        }
    }

    pub fn failure(hostname: &str, actual: &str, error: &str) -> Self {
        Self {
            valid: false,
            hostname: hostname.to_string(),
            matched_pin: None,
            actual_fingerprint: actual.to_string(),
            error: Some(error.to_string()),
            backup_match: false,
        }
    }
}
