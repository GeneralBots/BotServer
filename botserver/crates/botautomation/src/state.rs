//! Service state for the automation crate. External capabilities (LLM,
//! delivery channels, tools, owner e-mail resolution) are injected by the
//! integrator; the crate never constructs providers itself.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Shared PostgreSQL connection pool (same shape as sibling crates).
pub use botlib::db_pool::DbPool;

/// Injected LLM chat-completion function (shared contract, see BRIEF):
/// `(system prompt, user prompt, json params) -> raw completion or error`.
pub type LlmFn = Arc<dyn Fn(&str, &str, &str) -> Result<String, String> + Send + Sync>;

/// Injected delivery function (shared contract, see BRIEF):
/// `(channel: email|sms|whatsapp|telegram, to, subject, body) -> result`.
pub type DeliveryFn = Arc<dyn Fn(&str, &str, &str, &str) -> Result<(), String> + Send + Sync>;

/// Injected tool function: `(tool name, input JSON) -> output JSON or error`.
pub type ToolFn =
    Arc<dyn Fn(&str, &serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;

/// Resolves the notification e-mail of a schedule owner (CRM lookup wired by
/// the integrator). `None` means the owner has no reachable address.
pub type OwnerEmailResolver = Arc<dyn Fn(Uuid) -> Option<String> + Send + Sync>;

fn stub_llm(_system: &str, _user: &str, _params: &str) -> Result<String, String> {
    Err("LLM provider not wired for automation engine".to_string())
}

fn noop_delivery(channel: &str, to: &str, _subject: &str, _body: &str) -> Result<(), String> {
    tracing::warn!("delivery no-op dropped message to {to} via {channel}");
    Ok(())
}

fn no_owner_email(_owner_user_id: Uuid) -> Option<String> {
    None
}

pub struct AutomationService {
    pool: DbPool,
    llm_generate: LlmFn,
    deliver: DeliveryFn,
    tools: Arc<HashMap<String, ToolFn>>,
    run_cancels: Mutex<HashMap<Uuid, Arc<AtomicBool>>>,
    owner_email_resolver: OwnerEmailResolver,
}

impl AutomationService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            llm_generate: Arc::new(stub_llm),
            deliver: Arc::new(noop_delivery),
            tools: Arc::new(HashMap::new()),
            run_cancels: Mutex::new(HashMap::new()),
            owner_email_resolver: Arc::new(no_owner_email),
        }
    }

    pub fn pool(&self) -> &DbPool {
        &self.pool
    }

    pub fn with_llm(mut self, f: LlmFn) -> Self {
        self.llm_generate = f;
        self
    }

    pub fn with_delivery(mut self, f: DeliveryFn) -> Self {
        self.deliver = f;
        self
    }

    pub fn with_owner_email_resolver(mut self, f: OwnerEmailResolver) -> Self {
        self.owner_email_resolver = f;
        self
    }

    /// Registers a callable tool. Must be invoked before the service is
    /// wrapped in `Arc` and shared; late registrations are rejected with a
    /// logged error instead of panicking (AGENTS.md).
    pub fn register_tool(&mut self, name: impl Into<String>, f: ToolFn) -> &mut Self {
        match Arc::get_mut(&mut self.tools) {
            Some(tools) => {
                tools.insert(name.into(), f);
            }
            None => tracing::error!("register_tool ignored: AutomationService already shared"),
        }
        self
    }

    /// Registers (or returns the existing) cancellation flag for a run.
    pub fn cancel_flag(&self, run_id: Uuid) -> Arc<AtomicBool> {
        let mut guards = self
            .run_cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guards.entry(run_id).or_default().clone()
    }

    /// Removes and returns the cancellation flag once the run is finalized.
    pub fn take_cancel_flag(&self, run_id: Uuid) -> Option<Arc<AtomicBool>> {
        let mut guards = self
            .run_cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guards.remove(&run_id)
    }

    pub(crate) fn llm(&self) -> &LlmFn {
        &self.llm_generate
    }

    pub(crate) fn deliver_fn(&self) -> &DeliveryFn {
        &self.deliver
    }

    pub(crate) fn tool(&self, name: &str) -> Option<&ToolFn> {
        self.tools.get(name)
    }

    pub(crate) fn resolve_owner_email(&self, owner_user_id: Uuid) -> Option<String> {
        (self.owner_email_resolver)(owner_user_id)
    }
}
