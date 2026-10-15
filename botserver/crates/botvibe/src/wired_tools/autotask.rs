//! AutoTask group of wired tools (Issue #796): classification, plan
//! compilation and script generation via `botautotask`.
//!
//! These tools use the offline heuristic classifier and the pure script
//! generation pipeline (`botautotask::execution`); the LLM-backed compiler
//! remains reachable through the AutoTask API endpoints.

use super::{err, handler, require_str, result_of};
use crate::tool_executor::{ToolHandler, ToolSchema};
use crate::types::VibeState;
use botautotask::intent_classifier::{ClassifiedIntent, IntentClassifier};
use serde_json::{json, Value};

/// Intent-type → execution steps mapping used by the heuristic plan builder.
fn steps_for(intent_type: botautotask::intent_classifier::IntentType, name: &str) -> Vec<Value> {
    use botautotask::intent_classifier::IntentType;
    let base = |id: u32, label: &str, minutes: i32, approval: bool| json!({
        "id": format!("{id}"),
        "order": id,
        "name": label,
        "description": format!("{label} for '{name}'"),
        "keywords": [label.to_lowercase()],
        "priority": if approval { "high" } else { "medium" },
        "risk_level": if approval { "high" } else { "low" },
        "estimated_minutes": minutes,
        "requires_approval": approval,
    });
    let mut steps: Vec<Value> = Vec::new();
    match intent_type {
        IntentType::Schedule => {
            steps.push(base(1, "Define schedule", 5, false));
            steps.push(base(2, "Generate scheduler script", 5, false));
            steps.push(base(3, "Register scheduler", 10, true));
        }
        IntentType::Monitor => {
            steps.push(base(1, "Identify monitored subject", 5, false));
            steps.push(base(2, "Generate monitor script", 5, false));
            steps.push(base(3, "Enable event handler", 10, true));
        }
        IntentType::Tool => {
            steps.push(base(1, "Define voice trigger", 5, false));
            steps.push(base(2, "Generate tool script", 5, false));
            steps.push(base(3, "Register tool", 10, true));
        }
        IntentType::AppCreate => {
            steps.push(base(1, "Scaffold application", 15, false));
            steps.push(base(2, "Implement core logic", 30, false));
            steps.push(base(3, "Deploy application", 20, true));
        }
        IntentType::Goal => {
            steps.push(base(1, "Define target metric", 5, false));
            steps.push(base(2, "Generate goal script", 5, false));
            steps.push(base(3, "Start goal tracking", 5, true));
        }
        IntentType::Todo => {
            steps.push(base(1, "Capture reminder", 5, false));
            steps.push(base(2, "Generate reminder script", 5, false));
        }
        IntentType::Action => {
            steps.push(base(1, "Confirm action target", 5, false));
            steps.push(base(2, "Generate action script", 5, true));
        }
        IntentType::Unknown => {
            steps.push(base(1, "Clarify intent with the user", 2, false));
        }
    }
    steps
}

/// Compiles a heuristic plan (steps + estimates) from an intent string.
fn plan_for_intent(intent: &str) -> Result<Value, String> {
    let classification = IntentClassifier::classify_heuristic(intent)
        .map_err(|e| format!("intent classification failed: {e}"))?;
    let (script_path, script_body) = script_artifact(&classification);
    let name = classification
        .suggested_name
        .clone()
        .unwrap_or_else(|| "autotask_plan".to_string());
    let requires_approval = !matches!(
        classification.intent_type,
        botautotask::intent_classifier::IntentType::Unknown
    );
    Ok(json!({
        "intent_type": classification.intent_type.to_string(),
        "plan_name": name,
        "plan_description": format!("Auto-generated plan for: {}", classification.original_text),
        "steps": steps_for(classification.intent_type, &name),
        "confidence": classification.confidence,
        "requires_clarification": classification.requires_clarification,
        "clarification_question": classification.clarification_question,
        "requires_approval": requires_approval,
        "estimated_duration_minutes": 45,
        "script_path": script_path,
        "script_body": script_body,
    }))
}

/// Builds the persistable BASIC script artifact (relative Drive path + body).
fn script_artifact(classification: &ClassifiedIntent) -> (String, String) {
    botautotask::execution::script_for(classification, None)
}

/// `classify_intent` — classifies user intent via the AutoTask engine
/// (offline heuristic, same first pass the AutoTask API uses).
fn classify_intent() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let intent = match require_str(&args, "intent") {
            Ok(i) => i.to_string(),
            Err(e) => return err(e),
        };
        result_of(
            IntentClassifier::classify_heuristic(&intent)
                .map(|c| {
                    json!({
                        "intent_type": c.intent_type.to_string(),
                        "confidence": c.confidence,
                        "suggested_name": c.suggested_name,
                        "requires_clarification": c.requires_clarification,
                        "clarification_question": c.clarification_question,
                        "original_text": c.original_text,
                    })
                })
                .map_err(|e| format!("intent classification failed: {e}")),
        )
    })
}

/// `compile_plan` — compiles an execution plan from the classified intent.
fn compile_plan() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let intent = match require_str(&args, "intent") {
            Ok(i) => i.to_string(),
            Err(e) => return err(e),
        };
        result_of(plan_for_intent(&intent))
    })
}

/// `execute_plan` — classifies, compiles and generates the executable BASIC
/// artifact for the plan. The generated script is the persisted execution
/// unit; the DriveMonitor compiles it into the bot.
fn execute_plan() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let intent = match require_str(&args, "intent") {
            Ok(i) => i.to_string(),
            Err(e) => return err(e),
        };
        result_of(plan_for_intent(&intent).map(|mut plan| {
            plan["status"] = json!("generated");
            plan
        }))
    })
}

/// `create_and_execute` — classify + compile + generate in one flow.
fn create_and_execute() -> ToolHandler {
    handler(|args, _state: &dyn VibeState| async move {
        let intent = match require_str(&args, "intent") {
            Ok(i) => i.to_string(),
            Err(e) => return err(e),
        };
        result_of(plan_for_intent(&intent).map(|mut plan| {
            plan["status"] = json!("generated");
            plan
        }))
    })
}

/// Registration triplets for the autotask group.
pub fn autotask_tools() -> Vec<(String, ToolSchema, ToolHandler)> {
    use crate::types::VibeUseCase;
    let cases = vec![VibeUseCase::SoftwareDevelopment, VibeUseCase::CustomerSupport];
    vec![
        (
            "classify_intent".into(),
            ToolSchema::new("classify_intent", "Classifies user intent using the AutoTask engine")
                .with_parameters(json!({
                    "type": "object",
                    "properties": {
                        "intent": {"type": "string", "description": "Raw user intent text"}
                    },
                    "required": ["intent"]
                }))
                .with_use_cases(cases.clone()),
            classify_intent(),
        ),
        (
            "compile_plan".into(),
            ToolSchema::new("compile_plan", "Compiles an execution plan from the classified intent")
                .with_parameters(json!({
                    "type": "object",
                    "properties": {
                        "intent": {"type": "string", "description": "Raw user intent text"}
                    },
                    "required": ["intent"]
                }))
                .with_use_cases(cases.clone()),
            compile_plan(),
        ),
        (
            "execute_plan".into(),
            ToolSchema::new("execute_plan", "Executes a compiled plan (generates the executable BASIC artifact)")
                .with_parameters(json!({
                    "type": "object",
                    "properties": {
                        "intent": {"type": "string", "description": "Raw user intent text"}
                    },
                    "required": ["intent"]
                }))
                .with_approval()
                .with_use_cases(cases.clone()),
            execute_plan(),
        ),
        (
            "create_and_execute".into(),
            ToolSchema::new("create_and_execute", "Classify + compile + execute in one flow")
                .with_parameters(json!({
                    "type": "object",
                    "properties": {
                        "intent": {"type": "string", "description": "Raw user intent text"}
                    },
                    "required": ["intent"]
                }))
                .with_approval()
                .with_use_cases(cases),
            create_and_execute(),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_cover_all_intent_types() {
        use botautotask::intent_classifier::IntentType;
        for ty in [
            IntentType::AppCreate,
            IntentType::Todo,
            IntentType::Monitor,
            IntentType::Action,
            IntentType::Schedule,
            IntentType::Goal,
            IntentType::Tool,
            IntentType::Unknown,
        ] {
            let steps = steps_for(ty, "test");
            assert!(!steps.is_empty(), "missing steps for {ty:?}");
            assert!(steps.first().unwrap().get("order").is_some());
        }
    }

    #[test]
    fn plan_for_intent_creates_artifact() {
        let plan = plan_for_intent("criar um site de vendas").expect("plan");
        assert_eq!(plan["intent_type"], "APP_CREATE");
        assert!(plan["script_path"].as_str().unwrap().ends_with(".bas"));
        assert!(plan["script_body"].as_str().unwrap().contains("TALK"));
        assert_eq!(plan["requires_approval"], json!(true));
    }

    #[test]
    fn plan_for_unknown_intent_asks_clarification() {
        let plan = plan_for_intent("asdfghjkl qwerty").expect("plan");
        assert_eq!(plan["intent_type"], "UNKNOWN");
        assert_eq!(plan["requires_clarification"], json!(true));
    }
}
