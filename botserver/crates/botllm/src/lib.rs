pub mod bedrock;
pub mod claude;
pub mod glm;
pub mod hallucination_detector;
pub mod kimi;
pub mod kiro;
pub mod llm_models;
pub mod rate_limiter;
pub mod retry;
pub mod vertex;
pub mod cache;
pub mod ci_gate;
pub mod episodic_memory;
pub mod evaluation;
pub mod local;
pub mod smart_router;
pub mod observability;
pub mod pipeline;
pub mod openai_client;
pub mod openai_stream;
pub(crate) mod openai_stream_generate;
pub(crate) mod openai_stream_sse;
pub mod breaker;
pub mod dynamic_provider;
pub mod council;
pub mod profiles;
pub mod provider_catalog;
pub mod provider_factory;
pub mod provider_resolve;
pub mod router;

pub use ci_gate::{CiGateConfig, CiGateReport, CiGateRunner, RegressionSummary};
pub use evaluation::{
    EvaluationCriterion, EvaluationGate, EvaluationResult, Evaluator, RegressionReport,
};
pub use provider_catalog::{
    ALL_PROVIDER_TYPES, DEFAULT_MAX_OUTPUT_TOKENS, LLMProviderType, PROVIDER_PROFILES,
    ProviderProfile, profile_for_type,
};
pub use provider_resolve::{
    is_private_host, max_output_tokens_for_type, max_output_tokens_for_url,
    profiles_for_url, provider_type_from_name, provider_type_from_url, rate_limits_for_url,
};
pub use provider_factory::{
    create_llm_provider, create_llm_provider_from_url, llm_provider_type_from_name,
};
pub use rate_limiter::{ApiRateLimiter, RateLimits};
pub use hallucination_detector::HallucinationDetector;
pub use llm_models::{get_handler, ProcessedChunk, ModelHandler};
pub use openai_client::{OpenAIClient, AzureGPT5Client};
pub use claude::ClaudeClient;
pub use glm::GLMClient;
pub use kiro::KiroClient;
pub use vertex::VertexTokenManager;
pub use bedrock::BedrockClient;
pub use pipeline::{PipelineConfig, LlmPipeline, MessageBuilder, KbContextManager, PromptManager};
pub use router::{chat_with_fallback, RouteMeta};
pub use dynamic_provider::DynamicLLMProvider;



/// The provider contract is owned by `botlib` (#1498): every provider in this
/// crate implements the same trait the runtime holds as `Arc<dyn LLMProvider>`.
pub use botlib::traits::LLMProvider;

mod re_cache;
