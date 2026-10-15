use super::super::actions::AI_PLATFORM_ACTIONS;
use super::super::auth::KEYLESS;
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

/// LLM inference providers. These seed the integration catalog so users can
/// connect a provider in Settings (Integrations → LLM Providers) and then
/// assign providers to the three Vibe agent types (Reasoning / Agentic /
/// Fast) in the Vibe settings section.
///
/// Only providers verified with a tool-capable burst test (10 parallel
/// requests, 15s max, tool-call support) are listed here:
/// - Kilo gateway (`api.kilo.ai`) — keyless free-tier models, all 10/10 200
///   on burst with tool calls (except `nex-agi` 1x transient 429 upstream).
/// - LLM7 (`api.llm7.io`) — `minimax-m2.7` passes tool-capable sequentially
///   (the endpoint enforces a single concurrent request, so parallel bursts
///   return 429 by design).
///
/// Excluded: `mistral-Nemo-Instruct-2407` (no tool support) and
/// `nvidia/nemotron-3-ultra-550b` (read timeouts on the 15s budget).
pub(super) const PROVIDERS: &[ProviderSeed] = &[
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-dots",
            "Kilo · Dots 3 Note",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-nex",
            "Kilo · Nex N2.5 Pro",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-ling",
            "Kilo · Ling 3.0 Flash",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-openrouter",
            "Kilo · OpenRouter Free",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-nemotron-ultra",
            "Kilo · Nemotron Ultra 550B",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-nemotron-super",
            "Kilo · Nemotron Super 120B",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-nemotron-lightning",
            "Kilo · Nemotron Lightning",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-cohere",
            "Kilo · Cohere North Mini",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "kilo-stepfun",
            "Kilo · StepFun Flash",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.kilo.ai"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "llm7-minimax",
            "LLM7 · MiniMax M2.7 (stable)",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.llm7.io"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
    ProviderSeed {
        llm_available: true,
        ..provider(
            "llm7-minimax-v2",
            "LLM7 · MiniMax M2.7 (alt)",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: None, official_docs: Some("https://api.llm7.io"),
                auth: &KEYLESS, actions: AI_PLATFORM_ACTIONS,
            },
        )
    },
];