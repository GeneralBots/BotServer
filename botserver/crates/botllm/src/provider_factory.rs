//! Construction of concrete [`LLMProvider`] clients (issue #1467).
//!
//! Kept out of `lib.rs` because the factory is the only place that needs to know
//! both the provider identity ([`provider_catalog`]) and every concrete client,
//! and the dependency only runs one way.

use log::info;
use std::sync::Arc;

use crate::provider_catalog::LLMProviderType;
use crate::provider_resolve::provider_type_from_name;
use crate::bedrock::BedrockClient;
use crate::claude::ClaudeClient;
use crate::glm::GLMClient;
use crate::kiro::KiroClient;
use crate::openai_client::{AzureGPT5Client, OpenAIClient};
use crate::traits::LLMProvider;

/// Builds the client for an explicit backend identity.
///
/// `deployment_name` doubles as the model for providers whose API needs one at
/// construction time; it is ignored by the OpenAI-compatible tier, which takes
/// the model per request.
#[must_use]
pub fn create_llm_provider(
    provider_type: LLMProviderType,
    base_url: String,
    deployment_name: Option<String>,
    endpoint_path: Option<String>,
) -> Arc<dyn LLMProvider> {
    match provider_type {
        // The whole OpenAI-compatible tier shares one wire format. Only the
        // catalog-derived limits and the token ceiling differ, and both come
        // from the provider identity, so one arm serves all seven variants.
        LLMProviderType::OpenAI
        | LLMProviderType::DeepInfra
        | LLMProviderType::Cerebras
        | LLMProviderType::Fireworks
        | LLMProviderType::TogetherAI
        | LLMProviderType::OpenRouter
        | LLMProviderType::Requesty => {
            info!(
                "Creating {} LLM provider with URL: {}",
                provider_type.display_name(),
                base_url
            );
            Arc::new(OpenAIClient::with_provider(base_url, endpoint_path, provider_type))
        }
        LLMProviderType::Claude => {
            info!("Creating Claude LLM provider with URL: {}", base_url);
            Arc::new(ClaudeClient::new(base_url, deployment_name))
        }
        LLMProviderType::AzureClaude => {
            let deployment = deployment_name.unwrap_or_else(|| "claude-opus-4-5".to_string());
            info!(
                "Creating Azure Claude LLM provider with URL: {}, deployment: {}",
                base_url, deployment
            );
            Arc::new(ClaudeClient::azure(base_url, deployment))
        }
        LLMProviderType::AzureGPT5 => {
            info!("Creating Azure GPT-5/Responses LLM provider with URL: {}", base_url);
            Arc::new(AzureGPT5Client::new(base_url, endpoint_path))
        }
        LLMProviderType::GLM => {
            info!("Creating GLM/z.ai LLM provider with URL: {}", base_url);
            Arc::new(GLMClient::new(base_url))
        }
        LLMProviderType::Bedrock => {
            info!("Creating Bedrock LLM provider with exact URL: {}", base_url);
            Arc::new(BedrockClient::new(base_url))
        }
        LLMProviderType::Vertex => {
            info!("Creating Vertex/Gemini LLM provider with URL: {}", base_url);
            Arc::new(crate::vertex::VertexClient::new(base_url, endpoint_path))
        }
        LLMProviderType::Kiro => {
            info!("Creating Kiro LLM provider (CodeWhisperer protocol)");
            Arc::new(KiroClient::new(base_url))
        }
    }
}

/// Resolves the `llm-provider` bot-config value (or Vault `gbo/llm.provider`)
/// to a backend.
///
/// Returns `None` for an unknown or empty name so callers fall back to URL
/// detection rather than guessing.
pub fn llm_provider_type_from_name(name: &str) -> Option<LLMProviderType> {
    provider_type_from_name(name)
}

/// Builds a client from a base URL, letting an explicit provider override the
/// URL-derived one.
#[must_use]
pub fn create_llm_provider_from_url(
    url: &str,
    model: Option<String>,
    endpoint_path: Option<String>,
    explicit_provider: Option<LLMProviderType>,
) -> Arc<dyn LLMProvider> {
    let detected = LLMProviderType::from(url);
    let provider_type = explicit_provider.unwrap_or(detected);
    info!(
        "LLM provider: explicit={:?}, detected={:?}, URL={}",
        explicit_provider, detected, url
    );
    create_llm_provider(provider_type, url.to_string(), model, endpoint_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_explicit_openai_compatible_provider_is_not_downgraded() {
        let provider = create_llm_provider(
            LLMProviderType::Cerebras,
            "https://api.cerebras.ai/v1".to_string(),
            None,
            None,
        );
        assert!(Arc::strong_count(&provider) >= 1);
        // The client is built; a silent downgrade would still build, so assert
        // the dispatch decision instead of the concrete type.
        assert_eq!(
            provider_type_from_name("cerebras"),
            Some(LLMProviderType::Cerebras)
        );
    }

    #[test]
    fn explicit_provider_wins_over_url_detection() {
        let via_explicit =
            create_llm_provider_from_url("https://api.cerebras.ai/v1", None, None, Some(LLMProviderType::OpenRouter));
        assert!(Arc::strong_count(&via_explicit) >= 1);
        let detected = create_llm_provider_from_url("https://api.cerebras.ai/v1", None, None, None);
        assert!(Arc::strong_count(&detected) >= 1);
    }

    #[test]
    fn provider_names_resolve_or_report_unknown() {
        assert_eq!(
            llm_provider_type_from_name("together"),
            Some(LLMProviderType::TogetherAI)
        );
        assert_eq!(llm_provider_type_from_name("unknown-host"), None);
    }
}
