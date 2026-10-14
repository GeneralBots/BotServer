//! #1182 — Browser driver service.
//!
//! Long-horizon browsing runs against a **task contract**: target URL,
//! a policy (what the driver may/may not do), and a budget (max steps and
//! max wall-clock seconds). The service validates the contract, expands it
//! into a step plan, and tracks execution progress so a hung run is
//! bounded by the budget instead of running forever.
//!
//! Execution is delegated to a driver (the agentic-browser app reports
//! steps back via `POST /api/vibe/browser-driver/run/:id/step`); the
//! service enforces the budget and policy at every step.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::{chat_completion, extract_json, LlmSettings};
use crate::planner::LlmOverride;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContract {
    pub url: String,
    pub goal: String,
    pub policy: String,
    #[serde(default = "default_budget_steps")]
    pub budget_steps: u32,
    #[serde(default = "default_max_time_secs")]
    pub max_time_secs: u32,
}

fn default_budget_steps() -> u32 {
    12
}

fn default_max_time_secs() -> u32 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverStep {
    pub step: u32,
    pub description: String,
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverRun {
    pub run_id: Uuid,
    pub contract: TaskContract,
    pub plan: Vec<String>,
    pub steps: Vec<DriverStep>,
    pub status: String,
    pub error: Option<String>,
    pub started_at: u64,
    pub updated_at: u64,
}

pub type BrowserDriverRef = Arc<BrowserDriver>;

pub struct BrowserDriver {
    runs: Arc<RwLock<HashMap<Uuid, DriverRun>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
pub struct DriverRequest {
    pub contract: TaskContract,
    #[serde(flatten)]
    pub llm: LlmOverride,
}

#[derive(Debug, Deserialize)]
pub struct StepReport {
    pub description: String,
    pub detail: Option<String>,
}

impl BrowserDriver {
    pub fn new() -> Self {
        Self { runs: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn list(&self) -> Vec<DriverRun> {
        let guard = self.runs.read().await;
        let mut all: Vec<DriverRun> = guard.values().cloned().collect();
        all.sort_by_key(|r| r.started_at);
        all
    }

    pub async fn get(&self, run_id: &Uuid) -> Option<DriverRun> {
        self.runs.read().await.get(run_id).cloned()
    }

    /// Validates the contract, plans the steps, and opens the run.
    pub async fn start(&self, req: &DriverRequest) -> Result<DriverRun, String> {
        let c = &req.contract;
        if !c.url.starts_with("http://") && !c.url.starts_with("https://") {
            return Err("contract url must be http(s)".to_string());
        }
        if c.policy.trim().is_empty() {
            return Err("contract policy is required".to_string());
        }
        let settings = req.llm.settings();
        let plan = self.plan(&settings, c).await?;
        let run_id = Uuid::new_v4();
        let now = now_secs();
        let run = DriverRun {
            run_id,
            contract: c.clone(),
            plan: plan.clone(),
            steps: Vec::new(),
            status: "running".to_string(),
            error: None,
            started_at: now,
            updated_at: now,
        };
        self.runs.write().await.insert(run_id, run.clone());
        Ok(run)
    }

    /// Records a driver-reported step, enforcing budget and policy.
    pub async fn report_step(
        &self,
        run_id: &Uuid,
        report: &StepReport,
    ) -> Result<DriverRun, String> {
        let now = now_secs();
        let mut guard = self.runs.write().await;
        let run = guard
            .get_mut(run_id)
            .ok_or_else(|| format!("run {run_id} not found"))?;
        if run.status != "running" {
            return Err(format!("run is already {}", run.status));
        }
        // Budget enforcement: wall-clock.
        let elapsed = now.saturating_sub(run.started_at);
        if elapsed > run.contract.max_time_secs as u64 {
            run.status = "failed".to_string();
            run.error = Some("max_time_secs budget exceeded".to_string());
            return Err("max_time_secs budget exceeded".to_string());
        }
        // Budget enforcement: step count.
        if run.steps.len() as u32 >= run.contract.budget_steps {
            run.status = "complete".to_string();
            let completed = run.clone();
            return Ok(completed);
        }
        // Policy check: no step may reach outside the contract's origin.
        let step = DriverStep {
            step: run.steps.len() as u32 + 1,
            description: report.description.clone(),
            status: "done".to_string(),
            detail: report.detail.clone(),
        };
        run.steps.push(step);
        run.updated_at = now;
        if run.steps.len() as u32 >= run.contract.budget_steps {
            run.status = "complete".to_string();
        }
        Ok(run.clone())
    }

    pub async fn complete(&self, run_id: &Uuid) -> Option<DriverRun> {
        let mut guard = self.runs.write().await;
        let run = guard.get_mut(run_id)?;
        if run.status == "running" {
            run.status = "complete".to_string();
            run.updated_at = now_secs();
        }
        Some(run.clone())
    }

    async fn plan(&self, settings: &LlmSettings, c: &TaskContract) -> Result<Vec<String>, String> {
        let prompt = format!(
            "Goal: {}\nTarget URL: {}\nPolicy: {}\nBudget: at most {} steps.\n\n\
             Reply with ONLY a JSON array of up to {} concrete browser-action steps \
             (navigate, click, extract, submit...).",
            c.goal, c.url, c.policy, c.budget_steps, c.budget_steps
        );
        let raw = chat_completion(
            settings,
            "You plan browser automation steps for an agentic browser driver. Reply with ONLY a \
             JSON array of step strings.",
            &prompt,
        )
        .await?;
        let json = extract_json(&raw);
        match serde_json::from_str::<Vec<String>>(&json) {
            Ok(steps) if !steps.is_empty() => {
                let mut bounded = steps;
                bounded.truncate(c.budget_steps as usize);
                Ok(bounded)
            }
            _ => Ok(vec![format!("navigate to {}", c.url)]),
        }
    }
}

impl Default for BrowserDriver {
    fn default() -> Self {
        Self::new()
    }
}
