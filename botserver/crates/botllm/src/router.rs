//! Multi-profile LLM routing with fallback and per-profile circuit breakers
//! (issue #1173).
//!
//! The global routing table is built once from [`crate::profiles::load_profiles`].
//! Each profile receives its own breaker (threshold 5, open 60 seconds) and a
//! concurrency semaphore. Request execution reuses the existing chat-completions
//! request path, namely the [`crate::traits::LLMProvider::generate`] implementations
//! constructed by [`crate::provider_factory::create_llm_provider_from_url`].

use crate::breaker::Breaker;
use crate::profiles::{load_profiles, LlmProfile};
use crate::{
    openai_client::OpenAIClient, provider_factory::create_llm_provider_from_url, traits::LLMProvider,
};
use log::{info, warn};
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use tokio::sync::Semaphore;

/// Maximum simultaneous in-flight requests allowed per profile.
const MAX_CONCURRENCY_PER_PROFILE: usize = 8;
/// Consecutive failures before a profile circuit opens.
const FAILURE_THRESHOLD: u32 = 5;
/// Seconds a profile circuit stays open after tripping.
const OPEN_SECS: u64 = 60;
/// Hard cap on total fallback attempts for one logical call.
const MAX_ATTEMPTS: usize = 4;

/// Runtime state attached to every configured profile.
pub struct ProfileRuntime {
    /// Immutable profile description.
    pub profile: Arc<LlmProfile>,
    /// Provider client built from the existing factory, keyed to this URL.
    pub provider: Arc<dyn LLMProvider>,
    /// Circuit breaker guarding this profile.
    pub breaker: Breaker,
    /// Concurrency limiter for this profile.
    pub permits: Semaphore,
}

static ROUTES: LazyLock<Vec<Arc<ProfileRuntime>>> = LazyLock::new(|| {
    load_profiles()
        .into_iter()
        .map(build_runtime)
        .collect::<Vec<_>>()
});

static ROUND_ROBIN: AtomicUsize = AtomicUsize::new(0);

fn build_runtime(profile: LlmProfile) -> Arc<ProfileRuntime> {
    let model = if profile.model.is_empty() {
        None
    } else {
        Some(profile.model.clone())
    };
    info!(
        "Registering LLM profile '{}' (model {}, priority {}, caps {:?})",
        profile.id, profile.model, profile.priority, profile.caps
    );
    let provider = create_llm_provider_from_url(&profile.url, model, None, None);
    Arc::new(ProfileRuntime {
        profile: Arc::new(profile),
        provider,
        breaker: Breaker::new(),
        permits: Semaphore::new(MAX_CONCURRENCY_PER_PROFILE),
    })
}

/// Forces evaluation of the global routing table exactly once; safe to call
/// repeatedly because `LazyLock` initialization is idempotent.
pub fn init_once() {
    let count = ROUTES.len();
    info!("LLM router ready with {count} profile(s)");
}

pub(crate) fn route_candidates(caps: &[&str], prefer_cheap: bool) -> Vec<Arc<ProfileRuntime>> {
    let mut candidates: Vec<Arc<ProfileRuntime>> = ROUTES
        .iter()
        .filter(|rt| rt.breaker.allow() && rt.profile.matches_caps(caps))
        .cloned()
        .collect();
    candidates.sort_by(|a, b| {
        b.profile
            .effective_priority(prefer_cheap)
            .cmp(&a.profile.effective_priority(prefer_cheap))
            .then_with(|| a.profile.id.cmp(&b.profile.id))
    });
    let len = candidates.len();
    if len > 1 {
        let start = ROUND_ROBIN.fetch_add(1, Ordering::Relaxed) % len;
        candidates.rotate_left(start);
    }
    candidates
}

/// Selects the next best profile honoring capability requirements, closed
/// circuits, cost-adjusted priority ordering and round-robin rotation.
pub fn route(caps: &[&str], prefer_cheap: bool) -> Option<Arc<LlmProfile>> {
    route_candidates(caps, prefer_cheap)
        .into_iter()
        .next()
        .map(|rt| rt.profile.clone())
}

fn build_messages(system: &str, user: &str, params: &Value) -> Value {
    if let Some(array) = params.as_array() {
        if !array.is_empty() {
            return params.clone();
        }
    }
    serde_json::json!([
        { "role": "system", "content": OpenAIClient::sanitize_utf8(system) },
        { "role": "user", "content": OpenAIClient::sanitize_utf8(user) }
    ])
}

fn classify_error(detail: &str) -> &'static str {
    if detail.contains("429") {
        "rate limited"
    } else if detail.contains("500")
        || detail.contains("502")
        || detail.contains("503")
        || detail.contains("504")
    {
        "server error"
    } else {
        "transport error"
    }
}

pub(crate) async fn attempt(
    rt: &Arc<ProfileRuntime>,
    system: &str,
    user: &str,
    params: &Value,
) -> Result<(String, u64, u64), String> {
    let permit = rt
        .permits
        .acquire()
        .await
        .map_err(|e| format!("semaphore closed for profile {}: {e}", rt.profile.id))?;
    let messages = build_messages(system, user, params);
    let outcome = rt
        .provider
        .generate(user, &messages, &rt.profile.model, &rt.profile.key)
        .await;
    drop(permit);
    match outcome {
        Ok(content) => {
            rt.breaker.record_success();
            let tokens_in = OpenAIClient::estimate_tokens(&format!("{system}\n{user}")) as u64;
            let tokens_out = OpenAIClient::estimate_tokens(&content) as u64;
            Ok((content, tokens_in, tokens_out))
        }
        Err(e) => {
            let detail = format!("{e}");
            rt.breaker.record_failure(FAILURE_THRESHOLD, OPEN_SECS);
            warn!(
                "LLM profile '{}' failed ({}) : {}",
                rt.profile.id,
                classify_error(&detail),
                detail
            );
            Err(detail)
        }
    }
}

/// Metadata describing how a routed call was served.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMeta {
    /// Identifier of the profile that produced the answer.
    pub profile_id: String,
    /// Input tokens reported by the provider when present, otherwise estimated.
    pub tokens_in: u64,
    /// Output tokens reported by the provider when present, otherwise estimated.
    pub tokens_out: u64,
    /// True when at least one earlier profile failed before success.
    pub fallback_used: bool,
}

/// Executes a chat completion over the routing table with bounded fallback.
///
/// Candidates are ordered by the router; transport failures, HTTP 429 and 5xx
/// responses trip the profile breaker and iteration continues with the next
/// candidate, up to [`MAX_ATTEMPTS`] total attempts. The `params` value, when a
/// non-empty JSON array, replaces the default system/user message pair built
/// from the `system` and `user` arguments.
pub async fn chat_with_fallback(
    system: &str,
    user: &str,
    params: &Value,
    caps: &[&str],
) -> Result<(String, RouteMeta), String> {
    init_once();
    let candidates = route_candidates(caps, false);
    if candidates.is_empty() {
        return Err("no available LLM profile matches the requested capabilities".to_string());
    }
    let mut last_error = String::from("no attempts were made");
    let mut attempted = 0usize;
    for (index, rt) in candidates.iter().enumerate() {
        if attempted >= MAX_ATTEMPTS {
            break;
        }
        if !rt.breaker.allow() {
            continue;
        }
        attempted += 1;
        match attempt(rt, system, user, params).await {
            Ok((text, tokens_in, tokens_out)) => {
                return Ok((
                    text,
                    RouteMeta {
                        profile_id: rt.profile.id.clone(),
                        tokens_in,
                        tokens_out,
                        fallback_used: index > 0,
                    },
                ));
            }
            Err(e) => {
                last_error = e;
            }
        }
    }
    Err(format!(
        "all LLM profiles failed after {attempted} attempt(s): {last_error}"
    ))
}
