//! Executor stage: dependency-ordered ready-set execution with bounded
//! concurrency (`Semaphore(4)`), fork fan-out, merges, delivery steps,
//! per-step span persistence and usage metering.

use super::{pool_conn, ExecReport, PlanStep, RunCtx, StepOutcome};
use crate::models::NewAgentSpan;
use crate::schema::agent_spans;
use crate::{delivery, merge, metering};
use diesel::prelude::*;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::sync::Semaphore;
use uuid::Uuid;

pub(crate) const MAX_CONCURRENCY: usize = 4;
const OUTPUT_PREVIEW_CHARS: usize = 300;
const VM_SECONDS_CAP: u64 = i32::MAX as u64;

struct SpanFinish {
    ok: bool,
    output: Option<String>,
    error: Option<String>,
    tokens_in: i32,
    tokens_out: i32,
    vm_seconds: i32,
}

fn span_finish_from(result: &StepResult, vm_seconds: i32) -> SpanFinish {
    match result {
        Ok((output, ti, to)) => SpanFinish {
            ok: true,
            output: Some(output.clone()),
            error: None,
            tokens_in: *ti,
            tokens_out: *to,
            vm_seconds,
        },
        Err(e) => SpanFinish {
            ok: false,
            output: None,
            error: Some(e.clone()),
            tokens_in: 0,
            tokens_out: 0,
            vm_seconds,
        },
    }
}

fn insert_span(
    conn: &mut diesel::PgConnection,
    run_id: Uuid,
    parent_id: Option<Uuid>,
    kind: &str,
    name: &str,
    input_ref: &str,
) -> Option<Uuid> {
    let span_id = Uuid::new_v4();
    let new_span = NewAgentSpan {
        id: span_id,
        run_id,
        parent_id,
        kind: kind.to_string(),
        name: name.to_string(),
        input_ref: Some(input_ref.to_string()),
        status: "running".to_string(),
        started_at: Some(chrono::Utc::now()),
    };
    match diesel::insert_into(agent_spans::dsl::agent_spans)
        .values(&new_span)
        .execute(conn)
    {
        Ok(_) => Some(span_id),
        Err(e) => {
            tracing::error!("automation run {run_id}: span insert failed: {e}");
            None
        }
    }
}

fn finish_span(conn: &mut diesel::PgConnection, span_id: Uuid, finish: SpanFinish) {
    let preview = finish
        .output
        .map(|o| o.chars().take(OUTPUT_PREVIEW_CHARS).collect::<String>());
    if let Err(e) = diesel::update(agent_spans::dsl::agent_spans.find(span_id))
        .set((
            agent_spans::dsl::status.eq(if finish.ok { "ok" } else { "failed" }),
            agent_spans::dsl::output_ref.eq(preview),
            agent_spans::dsl::error.eq(finish.error),
            agent_spans::dsl::tokens_in.eq(Some(finish.tokens_in)),
            agent_spans::dsl::tokens_out.eq(Some(finish.tokens_out)),
            agent_spans::dsl::vm_seconds.eq(Some(finish.vm_seconds)),
            agent_spans::dsl::finished_at.eq(diesel::dsl::now),
        ))
        .execute(conn)
    {
        tracing::error!("automation span {span_id}: finish failed: {e}");
    }
}

fn meter(state: &crate::state::AutomationService, org_id: Option<Uuid>, tokens: (i32, i32), vm_seconds: i32) {
    let total = tokens.0.saturating_add(tokens.1);
    if total == 0 && vm_seconds == 0 {
        return;
    }
    let Ok(mut conn) = state.pool().get() else {
        tracing::error!("automation metering: db pool unavailable");
        return;
    };
    if total > 0 {
        if let Err(e) = metering::record_usage(&mut conn, org_id, "llm_tokens", total as f64) {
            tracing::error!("automation metering llm_tokens failed: {e}");
        }
    }
    if vm_seconds > 0 {
        if let Err(e) =
            metering::record_usage(&mut conn, org_id, "vm_seconds", vm_seconds as f64)
        {
            tracing::error!("automation metering vm_seconds failed: {e}");
        }
    }
}

type StepResult = Result<(String, i32, i32), String>;

fn tool_call(ctx: &RunCtx, step: &PlanStep) -> StepResult {
    let started = std::time::Instant::now();
    let tool_name = step.tool.clone().unwrap_or_else(|| step.name.clone());
    if let Some(schedule) = ctx.schedule.as_ref() {
        if let Some(allowlist) = schedule.allowlisted_tools() {
            if !allowlist.contains(&tool_name) {
                return Err(format!("tool {tool_name} is not in the schedule allowlist"));
            }
        }
    }
    let tool_fn = ctx
        .state
        .tool(&tool_name)
        .ok_or_else(|| format!("tool {tool_name} is not registered"))?;
    let value = tool_fn(&tool_name, &step.input)?;
    let vm_seconds = started.elapsed().as_secs().min(VM_SECONDS_CAP) as i32;
    Ok((value.to_string(), 0, vm_seconds))
}

fn llm_call(ctx: &RunCtx, step: &PlanStep) -> StepResult {
    let llm = ctx.state.llm().clone();
    let user_prompt = format!(
        "Execute this automation step.\nStep name: {}\nInput JSON:\n{}\nReturn the step output only.",
        step.name,
        serde_json::to_string_pretty(&step.input).unwrap_or_else(|_| "{}".to_string())
    );
    let raw = llm(super::plan::PLANNER_SYSTEM, &user_prompt, "{}")?;
    let tokens_in = (user_prompt.chars().count() / 4).min(i32::MAX as usize) as i32;
    let tokens_out = (raw.chars().count() / 4).min(i32::MAX as usize) as i32;
    Ok((raw, tokens_in, tokens_out))
}

/// Combines dependency outputs with the line/JSON merge helpers; conflicting
/// hunks are preserved inside the merged document instead of being dropped.
fn merge_call(step: &PlanStep, dep_outputs: &HashMap<String, String>) -> StepResult {
    let inputs: Vec<&String> = step
        .depends_on
        .iter()
        .filter_map(|d| dep_outputs.get(d))
        .collect();
    if inputs.is_empty() {
        return Err("merge step has no dependency outputs to combine".to_string());
    }
    let mut conflicts: Vec<String> = Vec::new();
    let mut merged = (*inputs[0]).clone();
    for next in &inputs[1..] {
        let both_json = serde_json::from_str::<serde_json::Value>(&merged).is_ok()
            && serde_json::from_str::<serde_json::Value>(next).is_ok();
        let outcome = if both_json {
            let a: serde_json::Value = serde_json::from_str(&merged).map_err(|e| e.to_string())?;
            let b: serde_json::Value = serde_json::from_str(next).map_err(|e| e.to_string())?;
            merge::merge_json(&a, &b)
        } else {
            merge::merge_text(&merged, next)
        };
        conflicts.extend(outcome.conflicts);
        merged = outcome.merged;
    }
    Ok((
        serde_json::to_string(&json!({ "merged": merged, "conflicts": conflicts }))
            .unwrap_or(merged),
        0,
        0,
    ))
}

async fn deliver_call(ctx: &Arc<RunCtx>) -> StepResult {
    let Some(run) = super::load_run(&ctx.state, ctx.run_id) else {
        return Err("run row disappeared before delivery".to_string());
    };
    let doc = delivery::dispatch(ctx.state.clone(), ctx.schedule.as_ref(), &run).await;
    if let Some(mut conn) = pool_conn(&ctx.state, "delivery status") {
        let _ = delivery::save_delivery_status(&mut conn, ctx.run_id, &doc);
    }
    Ok((doc.to_string(), 0, 0))
}

async fn fork_call(
    ctx: &Arc<RunCtx>,
    step: &PlanStep,
    parent_span: Option<Uuid>,
) -> StepResult {
    let children = step
        .input
        .get("children")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();
    if children.is_empty() {
        return Err("fork step has no children array in input".to_string());
    }
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut set = tokio::task::JoinSet::new();
    for child in children {
        let Ok(permit) = sem.clone().acquire_owned().await else {
            continue;
        };
        let name = child
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("child")
            .to_string();
        let tool = child.get("tool").and_then(|t| t.as_str()).map(str::to_string);
        let input = child.get("input").cloned().unwrap_or_else(|| json!({}));
        let child_step = PlanStep {
            id: format!("{}:{}", step.id, name),
            name: name.clone(),
            kind: if tool.is_some() { "tool" } else { "llm" }.to_string(),
            depends_on: Vec::new(),
            tool,
            input,
        };
        let ctx2 = ctx.clone();
        set.spawn(async move {
            let input_ref =
                serde_json::to_string(&child_step.input).unwrap_or_else(|_| "{}".to_string());
            let span = pool_conn(&ctx2.state, "fork child span")
                .and_then(|mut c| {
                    insert_span(&mut c, ctx2.run_id, parent_span, "fork", &child_step.name, &input_ref)
                });
            let result = match child_step.kind.as_str() {
                "tool" => tool_call(&ctx2, &child_step),
                _ => llm_call(&ctx2, &child_step),
            };
            let _guard = permit;
            if let Some(sid) = span {
                if let Some(mut c) = pool_conn(&ctx2.state, "fork child finish") {
                    let finish = span_finish_from(&result, 0);
                    finish_span(&mut c, sid, finish);
                }
            }
            (child_step.name, result)
        });
    }

    let mut outputs = Vec::new();
    let mut errors = Vec::new();
    let (mut ti_total, mut to_total) = (0, 0);
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok((name, Ok((output, ti, to)))) => {
                outputs.push(format!("{name}: {output}"));
                ti_total += ti;
                to_total += to;
            }
            Ok((name, Err(e))) => errors.push(format!("{name}: {e}")),
            Err(e) => errors.push(format!("join error: {e}")),
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok((outputs.join("\n"), ti_total, to_total))
}

/// Executes one plan step with full span lifecycle and metering.
async fn execute_step(
    ctx: Arc<RunCtx>,
    step: PlanStep,
    dep_outputs: HashMap<String, String>,
) -> StepOutcome {
    let input_ref = serde_json::to_string(&step.input).unwrap_or_else(|_| "{}".to_string());
    let span = pool_conn(&ctx.state, "span insert").and_then(|mut c| {
        insert_span(&mut c, ctx.run_id, None, &step.kind, &step.name, &input_ref)
    });
    let started = std::time::Instant::now();

    let result = match step.kind.as_str() {
        "tool" => tool_call(&ctx, &step),
        "llm" => llm_call(&ctx, &step),
        "merge" => merge_call(&step, &dep_outputs),
        "fork" => fork_call(&ctx, &step, span).await,
        "deliver" => deliver_call(&ctx).await,
        other => Err(format!("unknown step kind {other}")),
    };

    let vm_seconds = started.elapsed().as_secs().min(VM_SECONDS_CAP) as i32;
    let (tokens_in, tokens_out) = match &result {
        Ok((_, ti, to)) => (*ti, *to),
        Err(_) => (0, 0),
    };

    if let Some(sid) = span {
        if let Some(mut c) = pool_conn(&ctx.state, "span finish") {
            let finish = span_finish_from(&result, vm_seconds);
            finish_span(&mut c, sid, finish);
        }
    }
    meter(&ctx.state, ctx.org_id, (tokens_in, tokens_out), vm_seconds);

    let mut outcome = StepOutcome {
        id: step.id.clone(),
        name: step.name.clone(),
        kind: step.kind.clone(),
        ok: false,
        output: None,
        error: None,
        tokens_in,
        tokens_out,
        vm_seconds,
    };
    match result {
        Ok((output, _, _)) => {
            outcome.ok = true;
            outcome.output = Some(output);
        }
        Err(e) => outcome.error = Some(e),
    }
    outcome
}

/// Runs the plan as repeated ready-set waves: every step whose dependencies
/// succeeded becomes eligible; failed dependencies propagate as skipped
/// outcomes; circular leftovers are reported when no progress is possible.
pub(crate) async fn run_plan(ctx: Arc<RunCtx>, plan: Vec<PlanStep>) -> ExecReport {
    let order: Vec<String> = plan.iter().map(|s| s.id.clone()).collect();
    let known: HashSet<String> = order.iter().cloned().collect();
    let sem = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut outcomes: HashMap<String, StepOutcome> = HashMap::new();
    let mut todo = plan;
    let mut cancelled = false;

    while !todo.is_empty() {
        if ctx.cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let mut next_todo = Vec::new();
        let mut ready = Vec::new();
        let mut skipped_any = false;
        for step in std::mem::take(&mut todo) {
            let mut waiting = false;
            let mut dep_failed = false;
            for dep in &step.depends_on {
                if !known.contains(dep) {
                    continue;
                }
                match outcomes.get(dep) {
                    None => waiting = true,
                    Some(o) if !o.ok => dep_failed = true,
                    Some(_) => {}
                }
            }
            if waiting {
                next_todo.push(step);
            } else if dep_failed {
                skipped_any = true;
                let reason = format!("skipped: a dependency of {} was not successful", step.id);
                outcomes.insert(step.id.clone(), StepOutcome::skipped(&step, reason));
            } else {
                ready.push(step);
            }
        }
        todo = next_todo;

        if ready.is_empty() {
            if skipped_any {
                continue;
            }
            // No progress possible: circular or unresolved dependency graph.
            for step in &todo {
                let reason =
                    format!("skipped: circular or unresolved dependencies ({})", step.id);
                outcomes.insert(step.id.clone(), StepOutcome::skipped(step, reason));
            }
            todo.clear();
            continue;
        }

        let mut set = tokio::task::JoinSet::new();
        for step in ready {
            let mut dep_outputs = HashMap::new();
            for dep in &step.depends_on {
                if let Some(o) = outcomes.get(dep) {
                    if let Some(out) = o.output.as_ref() {
                        dep_outputs.insert(dep.clone(), out.clone());
                    }
                }
            }
            let ctx2 = ctx.clone();
            let sem2 = sem.clone();
            set.spawn(async move {
                let Ok(permit) = sem2.acquire_owned().await else {
                    return StepOutcome::skipped(
                        &step,
                        "executor semaphore closed".to_string(),
                    );
                };
                let outcome = execute_step(ctx2, step, dep_outputs).await;
                drop(permit);
                outcome
            });
        }
        while let Some(joined) = set.join_next().await {
            match joined {
                Ok(outcome) => {
                    outcomes.insert(outcome.id.clone(), outcome);
                }
                Err(e) => tracing::error!("automation step task join error: {e}"),
            }
        }
    }

    let mut report = ExecReport {
        cancelled,
        outcomes: Vec::with_capacity(order.len()),
    };
    for id in &order {
        if let Some(o) = outcomes.remove(id) {
            report.outcomes.push(o);
        }
    }
    report
}

/// Repair-path entry point used by the verifier stage; identical semantics
/// to a normal wave execution for a single step.
pub(crate) async fn execute_step_for_repair(
    ctx: Arc<RunCtx>,
    step: PlanStep,
    dep_outputs: HashMap<String, String>,
) -> StepOutcome {
    execute_step(ctx, step, dep_outputs).await
}
