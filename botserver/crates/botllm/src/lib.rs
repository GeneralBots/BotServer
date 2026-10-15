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

/// The provider contract is owned by `botlib` (#1498): every provider in this
/// crate implements the same trait the runtime holds as `Arc<dyn LLMProvider>`.
/// Re-exported as a module (#1500) so callers spell `botllm::traits::LLMProvider`.
pub use botlib::traits;

mod re_cache;
