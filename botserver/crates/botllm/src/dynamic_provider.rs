//! Dynamically swappable LLM provider wrapper.
//! Split from lib.rs for file-size limits.

use async_trait::async_trait;
use log::info;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::{
    provider_catalog::LLMProviderType, provider_factory::create_llm_provider_from_url,
    traits::LLMProvider,
};

pub struct DynamicLLMProvider {
    inner: RwLock<Arc<dyn LLMProvider>>,
}

impl std::fmt::Debug for DynamicLLMProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicLLMProvider").finish()
    }
}

impl DynamicLLMProvider {
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            inner: RwLock::new(provider),
        }
    }

    pub async fn update_provider(&self, new_provider: Arc<dyn LLMProvider>) {
        let mut guard = self.inner.write().await;
        *guard = new_provider;
        info!("LLM provider updated dynamically");
    }

    pub async fn update_from_config(
        &self,
        url: &str,
        model: Option<String>,
        endpoint_path: Option<String>,
        explicit_provider: Option<LLMProviderType>,
    ) {
        let new_provider = create_llm_provider_from_url(url, model, endpoint_path, explicit_provider);
        self.update_provider(new_provider).await;
    }

    async fn get_provider(&self) -> Arc<dyn LLMProvider> {
        self.inner.read().await.clone()
    }
}

#[async_trait]
impl LLMProvider for DynamicLLMProvider {
    async fn generate(
        &self,
        prompt: &str,
        config: &Value,
        model: &str,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        self.get_provider()
            .await
            .generate(prompt, config, model, key)
            .await
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &Value,
        tx: mpsc::Sender<String>,
        model: &str,
        key: &str,
        tools: Option<&Vec<Value>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.get_provider()
            .await
            .generate_stream(prompt, config, tx, model, key, tools)
            .await
    }

    async fn cancel_job(
        &self,
        session_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.get_provider().await.cancel_job(session_id).await
    }
}
