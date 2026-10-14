//! #1173 — Mixture-of-Agents routing.
//!
//! One prompt, many specialists: the request names a set of agent roles
//! (default: researcher / engineer / critic). Each role gets the same
//! prompt with its own system persona and runs in *parallel*; a final
//! synthesis LLM call aggregates the answers into one deliverable. If
//! `publish: true`, the deliverable gets a `share_token` served by the
//! anonymous `GET /api/vibe/moa/share/:token` route (#1180-style public
//! URLs).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::chat_completion;
use crate::planner::LlmOverride;

pub const DEFAULT_ROLES: [(&str, &str); 3] = [
    (
        "researcher",
        "You are a thorough researcher. Gather facts, weigh evidence, and report a precise, \
         source-minded answer.",
    ),
    (
        "engineer",
        "You are a pragmatic engineer. Turn the request into concrete, buildable outputs with \
         implementation detail.",
    ),
    (
        "critic",
        "You are a rigorous critic. Stress-test the request and the other answers, surface \
         risks, and propose the strongest improvement.",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaAnswer {
    pub role: String,
    pub output: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoaRun {
    pub run_id: Uuid,
    pub prompt: String,
    pub roles: Vec<String>,
    pub answers: Vec<MoaAnswer>,
    pub deliverable: Option<String>,
    pub share_token: Option<String>,
    pub published: bool,
    pub created_at: u64,
}

pub type MoaRef = Arc<MoaEngine>;

pub struct MoaEngine {
    runs: Arc<RwLock<HashMap<Uuid, MoaRun>>>,
    shares: Arc<RwLock<HashMap<String, MoaRun>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
pub struct MoaRequest {
    pub prompt: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub publish: bool,
    #[serde(flatten)]
    pub llm: LlmOverride,
}

impl MoaEngine {
    pub fn new() -> Self {
        Self {
            runs: Arc::new(RwLock::new(HashMap::new())),
            shares: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn list(&self) -> Vec<MoaRun> {
        let guard = self.runs.read().await;
        let mut all: Vec<MoaRun> = guard.values().cloned().collect();
        all.sort_by_key(|r| r.created_at);
        all
    }

    pub async fn get(&self, run_id: &Uuid) -> Option<MoaRun> {
        self.runs.read().await.get(run_id).cloned()
    }

    pub async fn resolve_share(&self, token: &str) -> Option<MoaRun> {
        self.shares.read().await.get(token).cloned()
    }

    /// Runs the role personas in parallel, then synthesizes one deliverable.
    pub async fn route(&self, req: &MoaRequest) -> Result<MoaRun, String> {
        let settings = req.llm.settings();
        let run_id = Uuid::new_v4();
        let roles: Vec<String> = if req.roles.is_empty() {
            DEFAULT_ROLES.iter().map(|(name, _)| name.to_string()).collect()
        } else {
            req.roles.clone()
        };
        // Hard cap: the role list is request-controlled and must never drive
        // unbounded parallel LLM fan-out.
        const MAX_ROLES: usize = 12;
        let roles = if roles.len() > MAX_ROLES { roles[..MAX_ROLES].to_vec() } else { roles };
        let seed = MoaRun {
            run_id,
            prompt: req.prompt.clone(),
            roles: roles.clone(),
            answers: Vec::new(),
            deliverable: None,
            share_token: None,
            published: false,
            created_at: now_secs(),
        };
        self.runs.write().await.insert(run_id, seed.clone());

        // Parallel specialist answers.
        let mut handles = Vec::with_capacity(roles.len());
        for role in &roles {
            let settings = settings.clone();
            let prompt = req.prompt.clone();
            let role_name = role.clone();
            let system = DEFAULT_ROLES
                .iter()
                .find(|(name, _)| name == &role_name)
                .map(|(_, sys)| sys.to_string())
                .unwrap_or_else(|| {
                    format!("You are the {role_name} specialist in a mixture-of-agents team.")
                });
            handles.push(tokio::spawn(async move {
                let output = chat_completion(&settings, &system, &prompt).await;
                (role_name, output)
            }));
        }
        let mut answers = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((role, Ok(output))) => answers.push(MoaAnswer {
                    role,
                    output,
                    status: "complete".to_string(),
                    error: None,
                }),
                Ok((role, Err(e))) => answers.push(MoaAnswer {
                    role,
                    output: String::new(),
                    status: "failed".to_string(),
                    error: Some(e),
                }),
                Err(e) => {
                    log::error!("MOA fork join failed: {e}");
                    answers.push(MoaAnswer {
                        role: "unknown".to_string(),
                        output: String::new(),
                        status: "failed".to_string(),
                        error: Some(format!("fork join failed: {e}")),
                    });
                }
            }
        }

        // Synthesis — aggregate only successful answers.
        let succeeded: Vec<&MoaAnswer> = answers.iter().filter(|a| a.status == "complete").collect();
        let deliverable = if succeeded.is_empty() {
            None
        } else {
            let body: Vec<String> = succeeded
                .iter()
                .map(|a| format!("## {}\n{}\n", a.role, a.output))
                .collect();
            let prompt = format!(
                "Synthesize the specialist answers below into ONE coherent deliverable that \
                 fully answers the prompt. Remove duplication, keep the best of each.\n\n\
                 PROMPT: {}\n\n{}\n",
                req.prompt,
                body.join("\n")
            );
            chat_completion(
                &settings,
                "You are the mixture-of-agents synthesis agent.",
                &prompt,
            )
            .await
            .ok()
        };

        let share_token = if req.publish {
            Some(format!("moa-{}", Uuid::new_v4().simple()))
        } else {
            None
        };
        let run = MoaRun {
            run_id,
            prompt: req.prompt.clone(),
            roles,
            answers,
            deliverable: deliverable.clone(),
            share_token: share_token.clone(),
            published: req.publish,
            created_at: now_secs(),
        };
        if let Some(token) = &share_token {
            self.shares.write().await.insert(token.clone(), run.clone());
        }
        self.runs.write().await.insert(run_id, run.clone());
        Ok(run)
    }
}

impl Default for MoaEngine {
    fn default() -> Self {
        Self::new()
    }
}
