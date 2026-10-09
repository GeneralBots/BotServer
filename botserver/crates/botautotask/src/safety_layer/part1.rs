use super::*;

impl Default for ConstraintCheckResult {
    fn default() -> Self {
        Self {
            passed: true,
            results: Vec::new(),
            risk_score: 0.0,
            blocking: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }
}
