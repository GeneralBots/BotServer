//! Data contracts for the per-app agent consent system.
//!
//! Action classes classify every application operation the agent may attempt.
//! Sensitive classes listed in [`ALWAYS_REPROMPT`] ignore stored "always"
//! grants once their cycle expires: the grant row carries an `expires_at`
//! set to end-of-month UTC at grant time and enforcement recomputes it.

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::schema::{app_permissions, consent_audit};

/// Every action class an application command can belong to.
pub const ACTION_CLASSES: &[&str] = &[
    "read", "create", "update", "delete", "payment", "share", "publish",
];

/// Classes that always re-prompt when the current grant cycle expires,
/// regardless of a stored "always" decision.
pub const ALWAYS_REPROMPT: &[&str] = &["payment"];

/// Classes reserved for administrator pre-approval (placeholder for future use).
pub const ADMIN_PREAPPROVAL_ONLY: &[&str] = &[];

pub fn is_known_action_class(class: &str) -> bool {
    ACTION_CLASSES.contains(&class)
}

pub fn is_reprompt_class(class: &str) -> bool {
    ALWAYS_REPROMPT.contains(&class)
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = app_permissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AppPermissionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub org_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub app_id: String,
    pub action_class: String,
    pub scope: serde_json::Value,
    pub granted: bool,
    pub granted_via: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub granted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = consent_audit)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ConsentAuditRow {
    pub id: Uuid,
    pub permission_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub request: serde_json::Value,
    pub outcome: String,
    pub decided_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Body requesting a stored grant for an application action class.
#[derive(Debug, Clone, Deserialize)]
pub struct GrantBody {
    pub app_id: String,
    pub action_class: String,
    #[serde(default)]
    pub scope: serde_json::Value,
    pub expires_at: Option<DateTime<Utc>>,
}

/// User decision over one pending consent request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    #[serde(rename = "allow_once")]
    AllowOnce,
    #[serde(rename = "always")]
    Always,
    #[serde(rename = "deny")]
    Deny,
}

/// Body posted by the consent card / settings UI to resolve a request.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveBody {
    pub request_id: String,
    pub decision: Decision,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_serde_roundtrip_lowercase() {
        assert_eq!(
            serde_json::from_str::<Decision>("\"allow_once\"").expect("parse allow_once"),
            Decision::AllowOnce
        );
        assert_eq!(
            serde_json::from_str::<Decision>("\"always\"").expect("parse always"),
            Decision::Always
        );
        assert_eq!(
            serde_json::from_str::<Decision>("\"deny\"").expect("parse deny"),
            Decision::Deny
        );
        assert_eq!(
            serde_json::to_string(&Decision::AllowOnce).expect("serialize allow_once"),
            "\"allow_once\""
        );
    }

    #[test]
    fn reprompt_classes_are_known() {
        assert!(is_reprompt_class("payment"));
        assert!(!is_reprompt_class("read"));
        assert!(is_known_action_class("publish"));
        assert!(!is_known_action_class("sudo"));
    }
}
