//! Verifier stage: after non-deliver steps complete, an LLM pass judges the
//! run and may request repairs; flagged steps are rerun up to two iterations
//! before the final verdict is persisted.

use super::{exec, load_run, PlanStep, RunCtx, StepOutcome};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) const VERIFIER_SYSTEM: &str = "You are an automation verifier. Given a plan digest \
with each step's output, judge whether the goal of the automation was achieved. Reply with a \
strict JSON object only: {\"verdict\":\"pass|fail\",\"issues\":[\"...\"],\"repairs\":[\"step-id\",\
...]} where repairs lists the ids of steps that must be rerun to fix the issues.";

const MAX_REPAIR_ITERATIONS: usize = 2;
const OUTPUT_DIGEST_CHARS: usize = 500;

#[derive(Debug, Deserialize)]
struct VerifierRaw {
    verdict: Option<String>,
    #[serde(default)]
    issues: Vec<String>,
    #[serde(default)]
    repairs: Vec<String>,
}

/// Final verification artifact stored on `agent_runs.verdict`, plus the
/// (possibly repaired) step outcomes.
pub(crate) struct VerifyReport {
    pub verdict: serde_json::Value,
    pub outcomes: Vec<StepOutcome>,
}

fn build_digest(goal: &str, outcomes: &[StepOutcome]) -> String {
    let mut digest = String::from("{\"goal\":");
    digest.push_str(&serde_json::to_string(goal).unwrap_or_else(|_| "\"\"".to_string()));
    digest.push_str(",\"steps\":[");
    for (index, outcome) in outcomes.iter().enumerate() {
        if index > 0 {
            digest.push(',');
        }
        let output = outcome
            .output
            .as_deref()
            .map(|o| o.chars().take(OUTPUT_DIGEST_CHARS).collect::<String>())
            .unwrap_or_default();
        let entry = serde_json::json!({
            "id": outcome.id,
            "name": outcome.name,
            "kind": outcome.kind,
            "ok": outcome.ok,
            "error": outcome.error,
            "output": output,
        });
        digest.push_str(&entry.to_string());
    }
    digest.push_str("]}");
    digest
}

fn parse_verdict(raw: &str) -> Option<(String, Vec<String>, Vec<String>)> {
    let parsed: VerifierRaw = serde_json::from_str(raw.trim()).ok()?;
    Some((
        parsed.verdict.unwrap_or_else(|| "unknown".to_string()),
        parsed.issues,
        parsed.repairs,
    ))
}

fn unavailable_verdict(reason: &str, outcomes: Vec<StepOutcome>) -> VerifyReport {
    VerifyReport {
        verdict: serde_json::json!({
            "verdict": "unknown",
            "issues": [reason],
            "repairs": [],
        }),
        outcomes,
    }
}

fn exhausted_verdict(outcomes: Vec<StepOutcome>) -> VerifyReport {
    VerifyReport {
        verdict: serde_json::json!({
            "verdict": "fail",
            "issues": ["repair iterations were exhausted without a passing verdict"],
            "repairs": [],
        }),
        outcomes,
    }
}

/// Verifies the executed plan and drives at most `MAX_REPAIR_ITERATIONS`
/// repair rounds over the steps the verifier flags. Deliver steps are never
/// rerun by repairs to avoid duplicate notifications.
pub(crate) async fn verify_and_repair(
    ctx: Arc<RunCtx>,
    mut outcomes: Vec<StepOutcome>,
) -> VerifyReport {
    if outcomes.iter().all(|o| o.kind == "deliver") {
        return VerifyReport {
            verdict: serde_json::json!({ "verdict": "pass", "issues": [], "repairs": [] }),
            outcomes,
        };
    }

    let llm = ctx.state.llm().clone();
    let goal = ctx
        .schedule
        .as_ref()
        .map(|s| s.goal.clone())
        .unwrap_or_default();

    for iteration in 0..MAX_REPAIR_ITERATIONS {
        let raw = llm(VERIFIER_SYSTEM, &build_digest(&goal, &outcomes), "{}");
        let Ok(raw) = raw else {
            tracing::error!("automation run {}: verifier llm unavailable", ctx.run_id);
            return unavailable_verdict("verifier LLM unavailable", outcomes);
        };
        let Some((verdict, issues, repairs)) = parse_verdict(&raw) else {
            tracing::error!("automation run {}: verifier output unparseable", ctx.run_id);
            return unavailable_verdict("verifier output was not valid JSON", outcomes);
        };

        let repair_ids: Vec<String> = repairs
            .into_iter()
            .filter(|id| {
                outcomes
                    .iter()
                    .any(|o| &o.id == id && o.kind != "deliver")
            })
            .collect();
        if verdict == "pass" || repair_ids.is_empty() {
            return VerifyReport {
                verdict: serde_json::json!({
                    "verdict": verdict,
                    "issues": issues,
                    "repairs": repair_ids,
                }),
                outcomes,
            };
        }

        tracing::error!(
            "automation run {}: verifier requested repairs on {repair_ids:?} (iteration {iteration})",
            ctx.run_id
        );
        rerun_steps(&ctx, &repair_ids, &mut outcomes).await;
    }

    exhausted_verdict(outcomes)
}

async fn rerun_steps(ctx: &Arc<RunCtx>, repair_ids: &[String], outcomes: &mut [StepOutcome]) {
    let Some(plan_steps) = load_plan_steps(ctx).await else {
        tracing::error!("automation run {}: repair skipped, plan unavailable", ctx.run_id);
        return;
    };
    for id in repair_ids {
        let Some(step) = plan_steps.iter().find(|s| &s.id == id) else {
            continue;
        };
        let dep_outputs = collect_dep_outputs(step, outcomes);
        let fresh =
            exec::execute_step_for_repair(ctx.clone(), step.clone(), dep_outputs).await;
        if let Some(slot) = outcomes.iter_mut().find(|o| &o.id == id) {
            *slot = fresh;
        }
    }
}

async fn load_plan_steps(ctx: &Arc<RunCtx>) -> Option<Vec<PlanStep>> {
    let run = load_run(&ctx.state, ctx.run_id)?;
    let plan_value = run.plan?;
    serde_json::from_value(plan_value).ok()
}

fn collect_dep_outputs(
    step: &PlanStep,
    outcomes: &[StepOutcome],
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for dep in &step.depends_on {
        if let Some(o) = outcomes.iter().find(|o| &o.id == dep) {
            if let Some(out) = o.output.as_ref() {
                map.insert(dep.clone(), out.clone());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(id: &str, kind: &str, ok: bool) -> StepOutcome {
        StepOutcome {
            id: id.to_string(),
            name: id.to_string(),
            kind: kind.to_string(),
            ok,
            output: Some("out".to_string()),
            error: None,
            tokens_in: 1,
            tokens_out: 1,
            vm_seconds: 0,
        }
    }

    #[test]
    fn verifier_digest_contains_goal_and_outputs() {
        let outcomes = vec![outcome("a", "llm", true)];
        let digest = build_digest("ship report", &outcomes);
        assert!(digest.contains("\"goal\":\"ship report\""));
        assert!(digest.contains("out"));
    }

    #[test]
    fn verdict_parsing_tolerates_missing_fields() {
        let (verdict, issues, repairs) =
            parse_verdict(r#"{"verdict":"fail","issues":["x"]}"#).expect("parses");
        assert_eq!(verdict, "fail");
        assert_eq!(issues.len(), 1);
        assert!(repairs.is_empty());
        assert!(parse_verdict("no json").is_none());
    }

    #[test]
    fn max_repair_iterations_is_bounded_to_two() {
        assert_eq!(MAX_REPAIR_ITERATIONS, 2);
    }
}
