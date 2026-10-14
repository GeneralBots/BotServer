//! #1185 — Proactivity scheduler.
//!
//! The scheduler runs on an interval and evaluates registered triggers. A
//! trigger fires a **suggestion card** only when the user has *consented*
//! to that trigger category (consent lives with the trigger and can be
//! revoked). Cards are surfaced through `GET /api/vibe/proactivity/cards`
//! and rendered by the desktop Notification Center.
//!
//! The trigger body is a small `{fn}` closure evaluated inside the
//! scheduler tick; results are stored as cards with a `seen` flag so the
//! frontend can dismiss them.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionCard {
    pub card_id: Uuid,
    pub category: String,
    pub title: String,
    pub body: String,
    pub action: Option<String>,
    pub seen: bool,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDef {
    pub trigger_id: Uuid,
    pub category: String,
    pub description: String,
    pub interval_secs: u64,
    pub consent_required: bool,
    pub consented: bool,
    pub last_fired: u64,
}

pub type ProactivityRef = Arc<ProactivityEngine>;

pub struct ProactivityEngine {
    triggers: Arc<RwLock<Vec<TriggerDef>>>,
    cards: Arc<RwLock<Vec<SuggestionCard>>>,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Deserialize)]
pub struct RegisterTriggerRequest {
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(default = "default_consent")]
    pub consent_required: bool,
    #[serde(default)]
    pub consented: bool,
}

fn default_interval() -> u64 {
    3600
}

fn default_consent() -> bool {
    true
}

impl ProactivityEngine {
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(Vec::new())),
            cards: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, req: &RegisterTriggerRequest) -> TriggerDef {
        let trigger = TriggerDef {
            trigger_id: Uuid::new_v4(),
            category: req.category.clone(),
            description: req.description.clone(),
            interval_secs: req.interval_secs.max(60),
            consent_required: req.consent_required,
            consented: req.consented,
            last_fired: 0,
        };
        self.triggers.write().await.push(trigger.clone());
        trigger
    }

    pub async fn list_triggers(&self) -> Vec<TriggerDef> {
        self.triggers.read().await.clone()
    }

    pub async fn set_consent(&self, trigger_id: &Uuid, consented: bool) -> bool {
        let mut guard = self.triggers.write().await;
        if let Some(t) = guard.iter_mut().find(|t| t.trigger_id == *trigger_id) {
            t.consented = consented;
            true
        } else {
            false
        }
    }

    pub async fn cards(&self, include_seen: bool) -> Vec<SuggestionCard> {
        let guard = self.cards.read().await;
        let mut all = guard.clone();
        all.sort_by_key(|c| std::cmp::Reverse(c.created_at));
        if include_seen {
            all
        } else {
            all.into_iter().filter(|c| !c.seen).collect()
        }
    }

    pub async fn mark_seen(&self, card_id: &Uuid) -> bool {
        let mut guard = self.cards.write().await;
        if let Some(c) = guard.iter_mut().find(|c| c.card_id == *card_id) {
            c.seen = true;
            true
        } else {
            false
        }
    }

    pub async fn clear_cards(&self) {
        self.cards.write().await.clear();
    }

    /// One scheduler tick: fires due, consented triggers. Returns the
    /// number of cards emitted this tick. `emit` turns a due trigger into
    /// the card body (or `None` to skip).
    pub async fn tick(&self, emit: &(dyn Fn(&TriggerDef) -> Option<String> + Sync)) -> usize {
        let now = now_secs();
        let mut emitted = 0usize;
        let mut due: Vec<TriggerDef> = Vec::new();
        {
            let mut guard = self.triggers.write().await;
            for t in guard.iter_mut() {
                let elapsed = now.saturating_sub(t.last_fired);
                if elapsed >= t.interval_secs && (!t.consent_required || t.consented) {
                    due.push(t.clone());
                    t.last_fired = now;
                }
            }
        }
        for t in due {
            if let Some(body) = emit(&t) {
                let card = SuggestionCard {
                    card_id: Uuid::new_v4(),
                    category: t.category.clone(),
                    title: format!("{category} suggestion", category = t.category),
                    body,
                    action: Some(t.category.clone()),
                    seen: false,
                    created_at: now,
                };
                self.cards.write().await.push(card);
                emitted += 1;
            }
        }
        emitted
    }
}

impl Default for ProactivityEngine {
    fn default() -> Self {
        Self::new()
    }
}
