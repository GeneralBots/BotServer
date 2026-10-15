//! Provider catalog for the OpenAI-compatible LLM tier (issue #1467).
//!
//! Every host below speaks the OpenAI chat-completions wire shape, so a single
//! [`crate::openai_client::OpenAIClient`] serves them all. Before this module they were
//! reachable only by accident: `LLMProviderType` had no variant for them, the
//! rate limiter recognised exactly two hostnames, and the output-token ceiling
//! was an inline `if base_url.contains("groq")` check. This table makes the tier
//! configured data — one row per host carrying its detection patterns, operating
//! caps, token ceiling and reference pricing.
//!
//! Host detection and the resolution of a URL or config name to a row live in
//! [`crate::provider_resolve`]; this file holds only the data.
//!
//! Two rules govern the numbers:
//!
//! * **Rate limits are conservative operating caps, not published quotas.** The
//!   limiter's job is to keep a bot inside a provider's quota and to keep the
//!   platform responsive; a cap below the real quota is safe, one above it is
//!   not. A host without a documented quota gets a generous-but-bounded row
//!   rather than [`RateLimits::unlimited`], so a misconfigured URL cannot turn
//!   into an unbounded retry storm.
//! * **Token ceilings are serving caps for the reference model on that host.**
//!   They are a request-shaping default, not a hard model limit; a host that
//!   raises its cap needs a new row, not a code change.
//!
//! [`RateLimits::unlimited`]: crate::rate_limiter::RateLimits::unlimited

use crate::rate_limiter::RateLimits;

/// Every LLM backend the platform can construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LLMProviderType {
    /// Generic OpenAI and any host not listed below.
    OpenAI,
    Claude,
    AzureClaude,
    AzureGPT5,
    GLM,
    Bedrock,
    Vertex,
    Kiro,
    /// DeepInfra — `api.deepinfra.com`.
    DeepInfra,
    /// Cerebras Inference — `api.cerebras.ai`.
    Cerebras,
    /// Fireworks AI — `api.fireworks.ai`.
    Fireworks,
    /// Together AI — `api.together.xyz`.
    TogetherAI,
    /// OpenRouter gateway — `openrouter.ai`.
    OpenRouter,
    /// Requesty gateway — `router.requesty.ai`.
    Requesty,
}

impl LLMProviderType {
    /// Stable identifier used in the `llm-provider` bot-config key.
    #[must_use]
    pub const fn config_name(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Claude => "claude",
            Self::AzureClaude => "azure-claude",
            Self::AzureGPT5 => "azure-gpt5",
            Self::GLM => "glm",
            Self::Bedrock => "bedrock",
            Self::Vertex => "vertex",
            Self::Kiro => "kiro",
            Self::DeepInfra => "deepinfra",
            Self::Cerebras => "cerebras",
            Self::Fireworks => "fireworks",
            Self::TogetherAI => "together",
            Self::OpenRouter => "openrouter",
            Self::Requesty => "requesty",
        }
    }

    /// Human-readable name for logs and API responses.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::OpenAI => "OpenAI",
            Self::Claude => "Claude",
            Self::AzureClaude => "Azure Claude",
            Self::AzureGPT5 => "Azure GPT-5",
            Self::GLM => "GLM",
            Self::Bedrock => "Bedrock",
            Self::Vertex => "Vertex",
            Self::Kiro => "Kiro",
            Self::DeepInfra => "DeepInfra",
            Self::Cerebras => "Cerebras",
            Self::Fireworks => "Fireworks AI",
            Self::TogetherAI => "Together AI",
            Self::OpenRouter => "OpenRouter",
            Self::Requesty => "Requesty",
        }
    }

    /// True when the backend speaks the OpenAI chat-completions shape and can be
    /// served by [`crate::openai_client::OpenAIClient`]. The native clients exist for the
    /// providers whose wire format differs.
    #[must_use]
    pub const fn is_openai_compatible(self) -> bool {
        matches!(
            self,
            Self::OpenAI
                | Self::DeepInfra
                | Self::Cerebras
                | Self::Fireworks
                | Self::TogetherAI
                | Self::OpenRouter
                | Self::Requesty
        )
    }
}

/// Every backend the catalog can express, in a stable order for UIs, docs and
/// tests. Each entry appears in [`PROVIDER_PROFILES`] at least once.
pub const ALL_PROVIDER_TYPES: &[LLMProviderType] = &[
    LLMProviderType::OpenAI,
    LLMProviderType::Claude,
    LLMProviderType::AzureClaude,
    LLMProviderType::AzureGPT5,
    LLMProviderType::GLM,
    LLMProviderType::Bedrock,
    LLMProviderType::Vertex,
    LLMProviderType::Kiro,
    LLMProviderType::DeepInfra,
    LLMProviderType::Cerebras,
    LLMProviderType::Fireworks,
    LLMProviderType::TogetherAI,
    LLMProviderType::OpenRouter,
    LLMProviderType::Requesty,
];

/// One row of the catalog.
#[derive(Debug, Clone, Copy)]
pub struct ProviderProfile {
    /// Backend identity, used as the table key.
    pub provider_type: LLMProviderType,
    /// Hostname fragments that identify this provider in a base URL. Matched
    /// case-insensitively as substrings, in table order.
    pub host_patterns: &'static [&'static str],
    /// Operating caps applied to every client built for this host.
    pub rate_limits: RateLimits,
    /// `max_tokens` sent with each request for the reference model.
    pub max_output_tokens: u32,
    /// Model the pricing and latency figures below were measured against.
    pub reference_model: &'static str,
    /// List price in USD per million input tokens.
    pub usd_per_mtok_in: f64,
    /// List price in USD per million output tokens.
    pub usd_per_mtok_out: f64,
}

/// Output-token ceiling for a host with no catalog row, and for `openai`.
///
/// Matches the platform's long-standing value, so a bot on `llm-provider,openai`
/// behaves exactly as it did before this table existed.
pub const DEFAULT_MAX_OUTPUT_TOKENS: u32 = 65_536;

/// The catalog. Order matters: the first row whose `host_patterns` match wins, so
/// specific hosts precede the generic `openai.com` row.
///
/// Detection precedence for a bare URL string is intentionally *not* taken from
/// this list — `provider_type_from_url` keeps the historical `claude` →
/// `azuregpt5` → `glm` → `bedrock` → `vertex` → `kiro` ordering so no existing
/// deployment changes provider.
pub const PROVIDER_PROFILES: &[ProviderProfile] = &[
    ProviderProfile {
        provider_type: LLMProviderType::Claude,
        host_patterns: &["api.anthropic.com", "anthropic.com"],
        rate_limits: RateLimits::anthropic_standard(),
        max_output_tokens: 8_192,
        reference_model: "claude-sonnet-4-5",
        usd_per_mtok_in: 3.00,
        usd_per_mtok_out: 15.00,
    },
    ProviderProfile {
        provider_type: LLMProviderType::AzureClaude,
        host_patterns: &["azure", "claude"],
        rate_limits: RateLimits::anthropic_standard(),
        max_output_tokens: 8_192,
        reference_model: "claude-sonnet-4-5",
        usd_per_mtok_in: 3.00,
        usd_per_mtok_out: 15.00,
    },
    ProviderProfile {
        provider_type: LLMProviderType::AzureGPT5,
        host_patterns: &["openai.azure.com", "azuregpt5", "gpt5"],
        rate_limits: RateLimits::azure_standard(),
        max_output_tokens: 16_384,
        reference_model: "gpt-5",
        usd_per_mtok_in: 1.25,
        usd_per_mtok_out: 10.00,
    },
    ProviderProfile {
        provider_type: LLMProviderType::GLM,
        host_patterns: &["z.ai", "glm"],
        rate_limits: RateLimits::glm_standard(),
        max_output_tokens: 16_384,
        reference_model: "glm-4.6",
        usd_per_mtok_in: 0.60,
        usd_per_mtok_out: 2.20,
    },
    ProviderProfile {
        provider_type: LLMProviderType::Bedrock,
        host_patterns: &["bedrock", "amazonaws.com"],
        rate_limits: RateLimits::bedrock_standard(),
        max_output_tokens: 8_192,
        reference_model: "anthropic.claude-sonnet-4-5",
        usd_per_mtok_in: 3.00,
        usd_per_mtok_out: 15.00,
    },
    ProviderProfile {
        provider_type: LLMProviderType::Vertex,
        host_patterns: &["googleapis.com", "vertex", "generativelanguage"],
        rate_limits: RateLimits::vertex_standard(),
        max_output_tokens: 8_192,
        reference_model: "gemini-2.5-pro",
        usd_per_mtok_in: 1.25,
        usd_per_mtok_out: 10.00,
    },
    ProviderProfile {
        provider_type: LLMProviderType::Kiro,
        host_patterns: &["kiro", "codewhisperer", "q.us-east-1.amazonaws.com"],
        rate_limits: RateLimits::kiro_standard(),
        max_output_tokens: 32_000,
        reference_model: "claude-sonnet-4-5",
        usd_per_mtok_in: 0.00,
        usd_per_mtok_out: 0.00,
    },
    // ── OpenAI-compatible tier ────────────────────────────────────────────
    ProviderProfile {
        provider_type: LLMProviderType::Cerebras,
        host_patterns: &["cerebras.ai"],
        rate_limits: RateLimits::cerebras_free_tier(),
        max_output_tokens: 8_192,
        reference_model: "llama-3.3-70b",
        usd_per_mtok_in: 0.60,
        usd_per_mtok_out: 0.60,
    },
    ProviderProfile {
        provider_type: LLMProviderType::DeepInfra,
        host_patterns: &["deepinfra.com"],
        rate_limits: RateLimits::deepinfra_standard(),
        max_output_tokens: 8_192,
        reference_model: "deepseek-v3",
        usd_per_mtok_in: 0.14,
        usd_per_mtok_out: 0.28,
    },
    ProviderProfile {
        provider_type: LLMProviderType::Fireworks,
        host_patterns: &["fireworks.ai"],
        rate_limits: RateLimits::fireworks_standard(),
        max_output_tokens: 16_384,
        reference_model: "llama-3.3-70b",
        usd_per_mtok_in: 0.90,
        usd_per_mtok_out: 0.90,
    },
    ProviderProfile {
        provider_type: LLMProviderType::TogetherAI,
        host_patterns: &["together.xyz", "together.ai", "api.together"],
        rate_limits: RateLimits::together_standard(),
        max_output_tokens: 8_192,
        reference_model: "llama-3.3-70b",
        usd_per_mtok_in: 0.88,
        usd_per_mtok_out: 0.88,
    },
    ProviderProfile {
        provider_type: LLMProviderType::OpenRouter,
        host_patterns: &["openrouter.ai"],
        rate_limits: RateLimits::openrouter_standard(),
        max_output_tokens: 16_384,
        reference_model: "llama-4-scout",
        usd_per_mtok_in: 0.08,
        usd_per_mtok_out: 0.30,
    },
    ProviderProfile {
        provider_type: LLMProviderType::Requesty,
        host_patterns: &["requesty.ai", "requesty.com"],
        rate_limits: RateLimits::requesty_standard(),
        max_output_tokens: 16_384,
        reference_model: "gateway",
        usd_per_mtok_in: 0.05,
        usd_per_mtok_out: 0.05,
    },
    // Groq stays the generic `openai` variant: it has always been reached that
    // way, and giving it its own enum variant would change behaviour for
    // existing deployments. The row still exists so its published limits stop
    // being an inline hostname check.
    ProviderProfile {
        provider_type: LLMProviderType::OpenAI,
        host_patterns: &["groq.com"],
        rate_limits: RateLimits::groq_free_tier(),
        max_output_tokens: 4_096,
        reference_model: "llama-3.3-70b-versatile",
        usd_per_mtok_in: 0.59,
        usd_per_mtok_out: 0.79,
    },
    // NVIDIA NIM serves the OpenAI shape and stays the generic `openai`
    // identity for backwards compatibility; the row exists so its rate limits
    // are no longer an implicit "everything else is unlimited".
    ProviderProfile {
        provider_type: LLMProviderType::OpenAI,
        host_patterns: &["build.nvidia.com", "nvidia.com"],
        rate_limits: RateLimits::nvidia_standard(),
        max_output_tokens: 32_768,
        reference_model: "gpt-oss-120b",
        usd_per_mtok_in: 0.15,
        usd_per_mtok_out: 0.60,
    },
    ProviderProfile {
        provider_type: LLMProviderType::OpenAI,
        host_patterns: &["api.openai.com", "openai.com"],
        rate_limits: RateLimits::openai_free_tier(),
        max_output_tokens: DEFAULT_MAX_OUTPUT_TOKENS,
        reference_model: "gpt-5",
        usd_per_mtok_in: 1.25,
        usd_per_mtok_out: 10.00,
    },
];

/// Catalog row for an explicit backend identity.
#[must_use]
pub fn profile_for_type(provider_type: LLMProviderType) -> Option<&'static ProviderProfile> {
    PROVIDER_PROFILES
        .iter()
        .find(|p| p.provider_type == provider_type)
}
