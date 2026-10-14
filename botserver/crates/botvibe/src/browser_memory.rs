//! #1175 — Agentic browser memory.
//!
//! Long-horizon browsing needs to remember what it learned across tabs and
//! sessions. This store keeps per-domain memory snippets (facts + source
//! URL) so the browser app can surface **site chips** (quick recall per
//! domain) and answer questions with **cited answers** (an LLM synthesis
//! that cites the memory entries it used).
//!
//! Memory entries are domain-scoped and capped per domain to keep the
//! store bounded; eviction is oldest-first.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::llm_client::chat_completion;
use crate::planner::LlmOverride;

const MAX_ENTRIES_PER_DOMAIN: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Uuid,
    pub domain: String,
    pub url: String,
    pub fact: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitedAnswer {
    pub answer: String,
    pub citations: Vec<String>,
    pub memory_used: usize,
}

pub type BrowserMemoryRef = Arc<BrowserMemory>;

pub struct BrowserMemory {
    entries: Arc<RwLock<HashMap<String, Vec<MemoryEntry>>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
pub struct RememberRequest {
    pub domain: String,
    pub url: String,
    pub fact: String,
}

#[derive(Debug, Deserialize)]
pub struct AskRequest {
    pub question: String,
    #[serde(default = "default_domains")]
    pub domains: Vec<String>,
    #[serde(flatten)]
    pub llm: LlmOverride,
}

fn default_domains() -> Vec<String> {
    Vec::new()
}

impl BrowserMemory {
    pub fn new() -> Self {
        Self { entries: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn remember(&self, req: &RememberRequest) -> MemoryEntry {
        let domain = req.domain.trim().to_lowercase();
        let entry = MemoryEntry {
            id: Uuid::new_v4(),
            domain: domain.clone(),
            url: req.url.clone(),
            fact: req.fact.trim().to_string(),
            created_at: now_secs(),
        };
        let mut guard = self.entries.write().await;
        let bucket = guard.entry(domain).or_default();
        bucket.push(entry.clone());
        if bucket.len() > MAX_ENTRIES_PER_DOMAIN {
            bucket.sort_by_key(|e| e.created_at);
            let excess = bucket.len() - MAX_ENTRIES_PER_DOMAIN;
            bucket.drain(..excess);
        }
        entry
    }

    /// Site chips — the most recent memory snippets per domain.
    pub async fn chips(&self, domains: &[String]) -> Vec<MemoryEntry> {
        let guard = self.entries.read().await;
        let mut out = Vec::new();
        for (domain, bucket) in guard.iter() {
            if domains.is_empty() || domains.iter().any(|d| d.to_lowercase() == *domain) {
                let mut recent = bucket.clone();
                recent.sort_by_key(|e| std::cmp::Reverse(e.created_at));
                out.extend(recent.into_iter().take(3));
            }
        }
        out
    }

    /// Cited answer — LLM synthesis over the relevant memory, listing the
    /// exact URLs it used as citations.
    pub async fn ask(&self, req: &AskRequest) -> Result<CitedAnswer, String> {
        let relevant = self.chips(&req.domains).await;
        if relevant.is_empty() {
            return Err("no browsing memory for those domains yet — browse first".to_string());
        }
        let context: Vec<String> = relevant
            .iter()
            .map(|e| format!("- [{}] {} (source: {})", e.domain, e.fact, e.url))
            .collect();
        let settings = req.llm.settings();
        let prompt = format!(
            "Question: {}\n\nBrowsing memory:\n{}\n\n\
             Answer the question using ONLY the memory above. End your answer with a line \
             'CITED:' followed by the source URLs you used, one per line.",
            req.question,
            context.join("\n")
        );
        let raw = chat_completion(
            &settings,
            "You answer from the user's personal browsing memory. Never invent facts; only use \
             the provided memory entries.",
            &prompt,
        )
        .await?;
        let (answer, citations) = split_citations(&raw);
        let citations: Vec<String> = citations
            .into_iter()
            .filter(|c| c.starts_with("http"))
            .collect();
        let memory_used = relevant.len();
        Ok(CitedAnswer { answer, citations, memory_used })
    }

    pub async fn clear(&self, domain: Option<&str>) {
        let mut guard = self.entries.write().await;
        match domain {
            Some(d) => {
                guard.remove(&d.to_lowercase());
            }
            None => guard.clear(),
        }
    }
}

impl Default for BrowserMemory {
    fn default() -> Self {
        Self::new()
    }
}

fn split_citations(raw: &str) -> (String, Vec<String>) {
    if let Some(idx) = raw.find("CITED:") {
        let answer = raw[..idx].trim().to_string();
        let cited = &raw[idx + "CITED:".len()..];
        let urls: Vec<String> = cited
            .lines()
            .map(|l| l.trim().trim_start_matches('-').trim().trim_matches('"').to_string())
            .filter(|l| !l.is_empty())
            .collect();
        (answer, urls)
    } else {
        (raw.trim().to_string(), Vec::new())
    }
}
