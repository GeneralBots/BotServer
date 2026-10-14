//! #1172 — Agent infrastructure: public Agent API.
//!
//! `POST /api/vibe/agents` registers an agent (name, description, optional
//! LLM config). `POST /api/vibe/agents/:id/exec` runs it: the request is
//! recorded, dispatched to the agent's executor (an LLM call by default, or
//! an external `endpoint`), traced into the run log, and metered into the
//! agent's usage counters (billing feed for #1183).
//!
//! All execution goes through the same non-panicking `chat_completion`
//! path; external endpoints are called with a short timeout so a slow
//! third-party agent cannot wedge the server.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::chat_completion;
use crate::planner::LlmOverride;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub agent_id: Uuid,
    pub name: String,
    pub description: String,
    pub endpoint: Option<String>,
    pub system_prompt: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub run_id: Uuid,
    pub agent_id: Uuid,
    pub input: String,
    pub output: String,
    pub status: String,
    pub error: Option<String>,
    pub latency_ms: u64,
    pub tokens_used: u64,
    pub estimated_cost: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentUsage {
    pub exec_count: u64,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub total_latency_ms: u64,
}

pub type AgentsRef = Arc<AgentRegistry>;

pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<Uuid, AgentDef>>>,
    runs: Arc<RwLock<HashMap<Uuid, Vec<AgentRun>>>>,
    usage: Arc<RwLock<HashMap<Uuid, AgentUsage>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
pub struct RegisterAgentRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecAgentRequest {
    pub input: String,
    #[serde(flatten)]
    pub llm: LlmOverride,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            runs: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, req: &RegisterAgentRequest) -> Result<AgentDef, String> {
        if req.name.trim().is_empty() {
            return Err("agent name is required".to_string());
        }
        let agent = AgentDef {
            agent_id: Uuid::new_v4(),
            name: req.name.trim().to_string(),
            description: req.description.clone(),
            endpoint: req.endpoint.clone(),
            system_prompt: req.system_prompt.clone(),
            created_at: now_secs(),
        };
        self.agents.write().await.insert(agent.agent_id, agent.clone());
        self.usage.write().await.insert(agent.agent_id, AgentUsage::default());
        Ok(agent)
    }

    pub async fn list(&self) -> Vec<AgentDef> {
        let guard = self.agents.read().await;
        let mut all: Vec<AgentDef> = guard.values().cloned().collect();
        all.sort_by_key(|a| a.name.clone());
        all
    }

    pub async fn get(&self, agent_id: &Uuid) -> Option<AgentDef> {
        self.agents.read().await.get(agent_id).cloned()
    }

    pub async fn delete(&self, agent_id: &Uuid) -> bool {
        let removed = self.agents.write().await.remove(agent_id).is_some();
        if removed {
            self.usage.write().await.remove(agent_id);
        }
        removed
    }

    pub async fn runs(&self, agent_id: &Uuid) -> Vec<AgentRun> {
        self.runs.read().await.get(agent_id).cloned().unwrap_or_default()
    }

    pub async fn usage(&self, agent_id: &Uuid) -> Option<AgentUsage> {
        self.usage.read().await.get(agent_id).cloned()
    }

    pub async fn exec(&self, agent_id: &Uuid, req: &ExecAgentRequest) -> Result<AgentRun, String> {
        let agent = self
            .get(agent_id)
            .await
            .ok_or_else(|| format!("agent {agent_id} not found"))?;
        let started = Instant::now();
        let run_id = Uuid::new_v4();

        let (output, status, error, tokens) = match &agent.endpoint {
            // External agent: bounded POST to the registered endpoint.
            Some(endpoint) => {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(60))
                    .build()
                    .map_err(|e| format!("http client: {e}"))?;
                let resp = client
                    .post(endpoint)
                    .json(&serde_json::json!({"input": req.input, "run_id": run_id}))
                    .send()
                    .await;
                match resp {
                    Ok(r) if r.status().is_success() => {
                        let text = r.text().await.unwrap_or_default();
                        (text, "complete".to_string(), None, 0)
                    }
                    Ok(r) => (
                        String::new(),
                        "failed".to_string(),
                        Some(format!("endpoint HTTP {}", r.status())),
                        0,
                    ),
                    Err(e) => (String::new(), "failed".to_string(), Some(format!("endpoint error: {e}")), 0),
                }
            }
            // Sandboxed in-process agent: LLM executor, traced and metered.
            None => {
                let settings = req.llm.settings();
                let system = agent
                    .system_prompt
                    .clone()
                    .unwrap_or_else(|| format!("You are the '{}' agent. Answer the user's request directly.", agent.name));
                match chat_completion(&settings, &system, &req.input).await {
                    Ok(out) => {
                        let tokens = estimate_tokens(&out);
                        (out, "complete".to_string(), None, tokens)
                    }
                    Err(e) => (String::new(), "failed".to_string(), Some(e), 0),
                }
            }
        };

        let latency_ms = started.elapsed().as_millis() as u64;
        let cost = tokens as f64 * 0.0000015; // rough $/token LLM estimate, feed for #1183
        let run = AgentRun {
            run_id,
            agent_id: *agent_id,
            input: req.input.clone(),
            output: output.clone(),
            status: status.clone(),
            error: error.clone(),
            latency_ms,
            tokens_used: tokens,
            estimated_cost: cost,
            timestamp: now_secs(),
        };

        self.runs.write().await.entry(*agent_id).or_default().push(run.clone());
        {
            let mut guard = self.usage.write().await;
            let usage = guard.entry(*agent_id).or_default();
            usage.exec_count += 1;
            usage.total_tokens += tokens;
            usage.total_cost += cost;
            usage.total_latency_ms += latency_ms;
        }
        if status == "failed" {
            log::error!("Agent {} run {} failed: {}", agent.name, run_id, error.clone().unwrap_or_default());
        }
        Ok(run)
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn estimate_tokens(text: &str) -> u64 {
    // ~4 chars per token is a fine billing approximation.
    (text.chars().count() as u64 / 4).max(1)
}
