//! #1171 — Planner-Executor-Verifier runtime with parallel forks and
//! auto-merge.
//!
//! 1. **PLAN** — the LLM decomposes an intent into an ordered step list.
//! 2. **EXECUTE** — steps run as *parallel forks* (`tokio::spawn`), one
//!    independent executor agent per fork.
//! 3. **VERIFY** — each fork output is scored (0–1) by the LLM verifier.
//! 4. **MERGE** — the best-scoring outputs are assembled into a single
//!    final report, with lower-scoring forks kept as alternates.
//!
//! Runs are kept in an in-memory registry (same model the pre-#816
//! canvases/issues stores used) and surfaced through `planner_api.rs`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::{chat_completion, extract_json, resolve_llm, LlmSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerFork {
    pub step: usize,
    pub label: String,
    pub output: String,
    pub score: f64,
    pub verdict: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerRun {
    pub run_id: Uuid,
    pub intent: String,
    pub plan: Vec<String>,
    pub forks: Vec<PlannerFork>,
    pub merged: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: u64,
}

pub type PlannerRef = Arc<PlannerExecutor>;

pub struct PlannerExecutor {
    runs: Arc<RwLock<HashMap<Uuid, PlannerRun>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Request override block shared by the AI-OS modules so callers can point
/// a single run at a different model without touching global config.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmOverride {
    pub llm_url: Option<String>,
    pub llm_model: Option<String>,
    pub llm_key: Option<String>,
}

impl LlmOverride {
    pub fn settings(&self) -> LlmSettings {
        resolve_llm(self.llm_url.as_deref(), self.llm_model.as_deref(), self.llm_key.as_deref())
    }
}

#[derive(Debug, Deserialize)]
pub struct PlannerRequest {
    pub intent: String,
    #[serde(default = "default_forks")]
    pub forks: u32,
    #[serde(flatten)]
    pub llm: LlmOverride,
}

fn default_forks() -> u32 {
    3
}

impl PlannerExecutor {
    pub fn new() -> Self {
        Self { runs: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn list(&self) -> Vec<PlannerRun> {
        let guard = self.runs.read().await;
        let mut all: Vec<PlannerRun> = guard.values().cloned().collect();
        all.sort_by_key(|r| r.created_at);
        all
    }

    pub async fn get(&self, run_id: &Uuid) -> Option<PlannerRun> {
        self.runs.read().await.get(run_id).cloned()
    }

    /// Full plan → execute → verify → merge lifecycle for one intent.
    pub async fn execute(&self, req: &PlannerRequest) -> Result<PlannerRun, String> {
        let settings = req.llm.settings();
        let run_id = Uuid::new_v4();
        let seed = PlannerRun {
            run_id,
            intent: req.intent.clone(),
            plan: Vec::new(),
            forks: Vec::new(),
            merged: None,
            status: "planning".to_string(),
            error: None,
            created_at: now_secs(),
        };
        self.runs.write().await.insert(run_id, seed.clone());

        // 1. PLAN — LLM decomposes the intent into steps.
        let plan = self.plan(&settings, &req.intent).await?;
        {
            let mut guard = self.runs.write().await;
            if let Some(run) = guard.get_mut(&run_id) {
                run.plan = plan.clone();
                run.status = "executing".to_string();
            }
        }
        if plan.is_empty() {
            self.fail(&run_id, "planner produced an empty step list".to_string()).await;
            return Err("planner produced an empty step list".to_string());
        }

        // 2. EXECUTE — parallel forks, one per step (bounded by forks param).
        // Hard caps: both values are request/LLM-influenced, so they must never
        // drive unbounded task fan-out.
        const MAX_FORKS: usize = 16;
        let fork_count = (req.forks.max(1) as usize)
            .min(plan.len())
            .min(MAX_FORKS);
        let mut handles = Vec::with_capacity(fork_count);
        for i in 0..fork_count {
            let settings = settings.clone();
            let plan = plan.clone();
            let intent = req.intent.clone();
            handles.push(tokio::spawn(async move {
                let step = i % plan.len();
                let label = plan[step].clone();
                let prompt = format!(
                    "Fork {step} of a parallel execution. Work on this step only:\n{label}\n\n\
                     Full intent for context:\n{intent}\n\nProduce a concrete, complete result \
                     (code, document, or answer) for this step."
                );
                let output = chat_completion(
                    &settings,
                    "You are an executor agent in a parallel multi-agent runtime. Produce the best \
                     possible result for your assigned step.",
                    &prompt,
                )
                .await;
                (step, label, output)
            }));
        }
        let mut outputs: Vec<(usize, String, Result<String, String>)> = Vec::new();
        for handle in handles {
            match handle.await {
                Ok((step, label, output)) => outputs.push((step, label, output)),
                Err(e) => {
                    self.fail(&run_id, format!("fork join failed: {e}")).await;
                    return Err(format!("fork join failed: {e}"));
                }
            }
        }

        // 3. VERIFY — score each fork output against the intent.
        let mut forks = Vec::with_capacity(outputs.len());
        for (step, label, output) in outputs {
            let output = match output {
                Ok(out) => out,
                Err(e) => {
                    self.fail(&run_id, format!("fork {step} ({label}) failed: {e}")).await;
                    return Err(format!("fork {step} ({label}) failed: {e}"));
                }
            };
            let (score, verdict) = self.verify(&settings, &req.intent, &label, &output).await;
            forks.push(PlannerFork {
                step,
                label,
                output,
                score,
                verdict,
                status: if score >= 0.5 { "passed" } else { "flagged" }.to_string(),
                error: None,
            });
        }

        // 4. MERGE — assemble the verified outputs into the final report.
        let merged = self.merge(&settings, &req.intent, &forks).await;
        let run = PlannerRun {
            run_id,
            intent: req.intent.clone(),
            plan,
            forks,
            merged: merged.clone(),
            status: "complete".to_string(),
            error: None,
            created_at: now_secs(),
        };
        self.runs.write().await.insert(run_id, run.clone());
        Ok(run)
    }

    async fn plan(&self, settings: &LlmSettings, intent: &str) -> Result<Vec<String>, String> {
        let raw = chat_completion(
            settings,
            "You decompose an intent into a short numbered execution plan. Reply with ONLY a JSON \
             array of step strings, no prose, no markdown fences.",
            intent,
        )
        .await?;
        let json = extract_json(&raw);
        match serde_json::from_str::<Vec<String>>(&json) {
            Ok(steps) => Ok(steps),
            Err(_) => Ok(vec![raw]),
        }
    }

    async fn verify(
        &self,
        settings: &LlmSettings,
        intent: &str,
        label: &str,
        output: &str,
    ) -> (f64, String) {
        let prompt = format!(
            "Intent: {intent}\nStep: {label}\n\nCandidate output:\n{output}\n\n\
             Reply with ONLY a JSON object: {{\"score\": 0.0..1.0, \"verdict\": \"short reason\"}}"
        );
        match chat_completion(
            settings,
            "You are a strict verifier. Score how well the candidate output satisfies its step \
             within the overall intent.",
            &prompt,
        )
        .await
        {
            Ok(raw) => {
                let json = extract_json(&raw);
                match serde_json::from_str::<serde_json::Value>(&json) {
                    Ok(v) => {
                        let score = v["score"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
                        let verdict = v["verdict"]
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| "no verdict".to_string());
                        (score, verdict)
                    }
                    Err(_) => (0.0, "unparseable verifier output".to_string()),
                }
            }
            Err(e) => (0.0, format!("verifier failed: {e}")),
        }
    }

    async fn merge(
        &self,
        settings: &LlmSettings,
        intent: &str,
        forks: &[PlannerFork],
    ) -> Option<String> {
        let mut passed: Vec<&PlannerFork> = forks.iter().filter(|f| f.score >= 0.5).collect();
        passed.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        if passed.is_empty() {
            return None;
        }
        let body: Vec<String> = passed
            .iter()
            .map(|f| format!("[{:.2}] {}\n{}\n", f.score, f.label, f.output))
            .collect();
        let prompt = format!(
            "Intent: {intent}\n\nVerified fork outputs:\n{}\n\n\
             Merge the best parts into one coherent final deliverable. Do not restate scores.",
            body.join("\n---\n")
        );
        match chat_completion(
            settings,
            "You are the merge agent. Combine the best verified fork outputs into a single \
             coherent deliverable that fully satisfies the intent.",
            &prompt,
        )
        .await
        {
            Ok(merged) => Some(merged),
            Err(e) => {
                log::error!("Planner merge failed: {e}");
                None
            }
        }
    }

    async fn fail(&self, run_id: &Uuid, error: String) {
        log::error!("Planner run {run_id} failed: {error}");
        let mut guard = self.runs.write().await;
        if let Some(run) = guard.get_mut(run_id) {
            run.status = "failed".to_string();
            run.error = Some(error);
        }
    }
}

impl Default for PlannerExecutor {
    fn default() -> Self {
        Self::new()
    }
}
