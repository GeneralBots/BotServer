// @generated from the actual migrations (6.0.21-01-compliance + 9.23-branch-scope-cleanup).
// Do not edit by hand; keep in sync with migrations/*/up.sql.

// Canonical definition lives in `botschema` (#1497).
pub use botschema::compliance_checks;

pub use botschema::compliance_issues;

pub use botschema::compliance_audit_log;

pub use botschema::compliance_evidence;

pub use botschema::compliance_risk_assessments;

pub use botschema::compliance_training_records;

pub use botschema::compliance_access_reviews;

pub use botschema::compliance_frameworks;

pub use botschema::compliance_controls;

pub use botschema::compliance_control_evidence;


// The list is unnecessary with Diesel 2.3 once the pairs cross crates (#1497).
