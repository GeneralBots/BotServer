//! Agentic UI plan protocol.
//!
//! The LLM emits a `{"__ui_plan__": {...}}` JSON block (mirroring the existing
//! `__tool_call__` mechanism) when the user requests a UI-level operation such
//! as creating a record. The backend validates every plan before it reaches
//! the frontend driver — the client never trusts the LLM directly.

pub(crate) use serde::{Deserialize, Serialize};

pub const UI_PLAN_TRIGGER: &str = "\"__ui_plan__\":";

/// Maximum number of steps accepted in a single plan. Spreadsheet data entry
/// can legitimately exceed 32 steps (e.g. filling a 10-row table), so keep
/// this generous while still bounding runaway LLM output.
pub const MAX_UI_STEPS: usize = 200;

/// Maximum length of any string value produced by the LLM.
pub const MAX_VALUE_LEN: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiOp {
    Open,
    Click,
    Fill,
    Select,
    Submit,
    Wait,
    Cell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiStep {
    pub op: UiOp,
    pub app: Option<String>,
    pub label: Option<String>,
    pub field: Option<String>,
    pub value: Option<String>,
    pub ms: Option<u64>,
    pub cell: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiPlan {
    pub app: Option<String>,
    pub steps: Vec<UiStep>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_and_validate() {
        let plan = serde_json::json!({
            "app": "crm",
            "steps": [
                {"op": "open", "app": "crm"},
                {"op": "fill", "field": "First Name", "value": "Jeff"},
                {"op": "submit"}
            ]
        });
        let parsed = parse_ui_plan(&plan).unwrap();
        assert_eq!(parsed.steps.len(), 3);
        assert_eq!(parsed.app.as_deref(), Some("crm"));
    }

    #[test]
    fn test_rejects_unknown_op() {
        let plan = serde_json::json!({
            "steps": [{"op": "rm -rf"}]
        });
        let raw: Result<UiPlan, _> = serde_json::from_value(plan);
        assert!(raw.is_err());
    }

    #[test]
    fn test_rejects_invalid_app() {
        let plan = serde_json::json!({
            "steps": [{"op": "open", "app": "../etc"}]
        });
        let raw: UiPlan = serde_json::from_value(plan).unwrap();
        assert!(validate_ui_plan(&raw).is_err());
    }

    #[test]
    fn test_sanitizes_values() {
        let plan = serde_json::json!({
            "steps": [{"op": "fill", "field": "Name", "value": "<script>alert(1)</script>"}]
        });
        let parsed = parse_ui_plan(&plan).unwrap();
        assert_eq!(parsed.steps[0].value.as_deref(), Some("scriptalert(1)/script"));
    }

    #[test]
    fn test_rejects_too_many_steps() {
        let steps: Vec<serde_json::Value> = (0..300)
            .map(|i| serde_json::json!({"op": "wait", "ms": i + 1}))
            .collect();
        let plan = serde_json::json!({"steps": steps});
        let raw: UiPlan = serde_json::from_value(plan).unwrap();
        assert!(validate_ui_plan(&raw).is_err());
    }

    #[test]
    fn test_open_step_requires_app() {
        let plan = serde_json::json!({
            "steps": [{"op": "open"}]
        });
        let raw: UiPlan = serde_json::from_value(plan).unwrap();
        assert!(validate_ui_plan(&raw).is_err());
    }

    #[test]
    fn test_extract_embedded_plan() {
        let raw = r#"Sure! Creating the customer now.
{"__ui_plan__": {"app": "crm", "steps": [
  {"op": "open", "app": "crm"},
  {"op": "click", "label": "New Lead"},
  {"op": "fill", "field": "First Name", "value": "Jeff"}
]}}
Done."#;
        let result = extract_and_validate_plan(raw);
        let plan = result.expect("plan present").expect("plan valid");
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.steps[2].op, UiOp::Fill);
        assert_eq!(
            plan.steps[2].value.as_deref(),
            Some("Jeff")
        );
    }

    #[test]
    fn test_extract_missing_plan_returns_none() {
        assert!(extract_and_validate_plan("just a greeting").is_none());
    }

    #[test]
    fn test_extract_invalid_json_returns_err() {
        let raw = r#"{"__ui_plan__": {"app": "crm", "steps": [{ "op": "bogus_op" }]}}"#;
        let result = extract_and_validate_plan(raw);
        assert!(result.is_some());
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn test_extract_unbalanced_json_returns_none() {
        let raw = r#"{"__ui_plan__": {"app": "crm", "steps": [broken"#;
        assert!(extract_and_validate_plan(raw).is_none());
    }

    #[test]
    fn test_strip_plan_json_removes_block() {
        let raw = r#"{"__ui_plan__": {"app": "mail", "steps": [{"op": "open", "app": "mail"}]}}
Compose a message to Rodrigo."#;
        let stripped = strip_plan_json(raw);
        assert!(!stripped.contains("__ui_plan__"));
        assert!(stripped.contains("Compose a message to Rodrigo."));
    }

    #[test]
    fn test_strip_plan_json_keeps_normal_content() {
        let raw = "Just a plain greeting.";
        assert_eq!(strip_plan_json(raw), raw);
    }
}

mod part1;
mod part2;
mod part3;

pub use part2::*;
pub use part3::*;
