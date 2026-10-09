pub(crate) use crate::types::{AutoTaskState, ConfigOps, LlmProviderOps, UserSession};
pub(crate) use crate::task_types::{ExecutionMode, TaskPriority};
pub(crate) use crate::intent_classifier::{ClassifiedIntent, IntentType};
pub(crate) use crate::safety_layer::{SafetyLayer, SimulationResult};
pub(crate) use log::{info, warn};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledIntent {
    pub id: String,
    pub intent_type: IntentType,
    pub plan_name: String,
    pub plan_description: String,
    pub steps: Vec<PlanStep>,
    pub alternatives: Vec<Alternative>,
    pub confidence: f64,
    pub risk_level: String,
    pub estimated_duration_minutes: i32,
    pub estimated_cost: f64,
    pub resource_estimate: ResourceEstimate,
    pub basic_program: Option<String>,
    pub requires_approval: bool,
    pub mcp_servers: Vec<String>,
    pub external_apis: Vec<String>,
    pub risks: Vec<Risk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub order: i32,
    pub name: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub priority: String,
    pub risk_level: String,
    pub estimated_minutes: i32,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub id: String,
    pub description: String,
    pub confidence: f64,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub estimated_cost: Option<f64>,
    pub estimated_time_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEstimate {
    pub compute_hours: f64,
    pub storage_gb: f64,
    pub api_calls: i32,
    pub llm_tokens: i32,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Risk {
    pub id: String,
    pub category: String,
    pub description: String,
    pub probability: f64,
    pub impact: String,
}

/// Catalog-free instruction used when the host exposes no keyword list.
const DEGRADED_BASIC_REFERENCE: &str =
    "BASIC reference unavailable: stick to TALK, GET FROM, SET, SAVE and IF/THEN/ELSE/END IF.";

pub struct IntentCompiler {
    state: Arc<dyn AutoTaskState>,
    config_ops: Arc<dyn ConfigOps>,
    llm_ops: Arc<dyn LlmProviderOps>,
    safety: SafetyLayer,
}

#[cfg(test)]
mod tests {
    use super::{compile_prompt, extract_json_object, DEGRADED_BASIC_REFERENCE};

    const CATALOG: &str = "## BASIC keywords (closed set — use only these)\n- TALK, GET, SET, SAVE";
    const PROMPT_DUMP_MARKER: &str = "===PROMPT===\n";

    #[test]
    fn compile_prompt_demands_a_real_basic_program() {
        let prompt = compile_prompt("liste as faturas pendentes", CATALOG);
        // The regression this closes: the prompt carried
        // `"basic_program": null`, so the model answered a plan and the
        // pipeline persisted a `TALK` stub instead of an automation.
        assert!(!prompt.contains("\"basic_program\": null"));
        assert!(prompt.contains("\"basic_program\": \""));
        assert!(prompt.contains("ON ERROR RESUME NEXT"));
        assert!(prompt.contains("IF cond THEN"));
        assert!(prompt.contains("liste as faturas pendentes"));
    }

    #[test]
    fn compile_prompt_embeds_the_host_keyword_catalog() {
        let prompt = compile_prompt("x", CATALOG);
        assert!(prompt.contains("closed set"));
        assert!(prompt.contains("TALK, GET, SET, SAVE"));
    }

    #[test]
    fn compile_prompt_dump_is_printable() {
        // Printing the assembled prompt is the fastest way to inspect what the
        // generator actually asks the model for (no network involved).
        println!("{PROMPT_DUMP_MARKER}{}", compile_prompt("dump", CATALOG));
    }

    #[test]
    fn compile_prompt_works_without_a_catalog() {
        let prompt = compile_prompt("x", DEGRADED_BASIC_REFERENCE);
        assert!(prompt.contains("\"basic_program\": \""));
        assert!(prompt.contains("BASIC reference unavailable"));
    }

    #[test]
    fn extracts_the_object_from_prose_wrapped_completions() {
        // Prod regression (2026-09-29): the model answered with prose around
        // the JSON object; parsing the raw text died with "trailing
        // characters" and every such compile degraded to the TALK stub.
        let wrapped = "Sure! Here is the plan:\n{\"plan_name\":\"x\"}\nLet me know if that works.";
        assert_eq!(extract_json_object(wrapped), "{\"plan_name\":\"x\"}");
    }

    #[test]
    fn extracts_the_object_from_inside_code_fences() {
        let fenced = "```json\n{\"plan_name\":\"x\",\"steps\":[]\n}\n```";
        let extracted = extract_json_object(fenced);
        assert!(extracted.starts_with('{'));
        assert!(extracted.ends_with('}'));
        assert!(!extracted.contains("```"));
    }

    #[test]
    fn extracts_the_first_object_when_junk_follows() {
        // Prod regression (2026-09-29, "trailing characters at line 50"): the
        // model emitted the JSON object and then more content after it, so the
        // naive first-{ … last-} slice still failed to parse. The balanced
        // block must end at its own closing brace.
        let with_junk = "{\"plan_name\":\"x\"}\n{\"note\":\"dup\"}\nHope that helps!";
        assert_eq!(extract_json_object(with_junk), "{\"plan_name\":\"x\"}");
    }

    #[test]
    fn braces_inside_strings_do_not_confuse_the_balanced_scan() {
        let tricky = "{\"plan_name\":\"a}b{c\"}";
        assert_eq!(extract_json_object(tricky), tricky);
    }

    #[test]
    fn keeps_the_input_when_there_is_no_object() {
        assert_eq!(extract_json_object("no braces here"), "no braces here");
        assert_eq!(extract_json_object(""), "");
    }
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
use part3::*;
