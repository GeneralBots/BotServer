//! Adapter for a botlib LLM provider usable from the AutoTask
//! `LlmProviderOps` facade (#755): bridges the async provider into the sync
//! trait used by the AutoTask compiler/classifier. When no provider is
//! configured the adapter yields a Boxed error, letting callers fall back to
//! the offline heuristic classifier.

use crate::types::{BoxError, BoxFuture, LlmProviderOps};
use botlib::traits::LLMProvider;
use std::sync::Arc;

/// Wraps an optional botlib LLM provider.
pub struct BotlibLlmAdapter(pub Option<Arc<dyn LLMProvider>>);

impl LlmProviderOps for BotlibLlmAdapter {
    fn generate_stream(
        &self,
        prompt: &str,
        config: &serde_json::Value,
        tx: tokio::sync::mpsc::Sender<String>,
        model: &str,
        key: &str,
        _system_prompt: Option<&str>,
    ) -> BoxFuture<()> {
        let provider = self.0.clone();
        let prompt = prompt.to_string();
        let config = config.clone();
        let model = model.to_string();
        let key = key.to_string();
        Box::pin(async move {
            match provider {
                Some(p) => p
                    .generate_stream(&prompt, &config, tx, &model, &key, None)
                    .await
                    .map_err(BoxError::from),
                None => Err(BoxError::from(
                    "no LLM provider configured; AutoTask falls back to heuristics".to_string(),
                )),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_provider_yields_error() {
        let adapter = BotlibLlmAdapter(None);
        let (tx, _rx) = tokio::sync::mpsc::channel(4);
        let fut = adapter.generate_stream("hi", &serde_json::json!({}), tx, "model", "key", None);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        assert!(rt.block_on(fut).is_err());
    }

    #[test]
    fn provided_provider_streams_chunks() {
        #[derive(Debug)]
        struct FakeLlm;
        #[async_trait::async_trait]
        impl LLMProvider for FakeLlm {
            async fn generate(
                &self,
                _prompt: &str,
                _config: &serde_json::Value,
                _model: &str,
                _key: &str,
            ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                Ok("fake".to_string())
            }

            async fn generate_simple(&self, _prompt: &str) -> Result<String, String> {
                Ok("fake".to_string())
            }

            async fn generate_with_context(
                &self,
                _prompt: &str,
                _context: &str,
            ) -> Result<String, String> {
                Ok("fake".to_string())
            }

            async fn generate_stream(
                &self,
                _prompt: &str,
                _config: &serde_json::Value,
                tx: tokio::sync::mpsc::Sender<String>,
                _model: &str,
                _key: &str,
                _tools: Option<&Vec<serde_json::Value>>,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                let _ = tx.send("chunk-1".to_string()).await;
                Ok(())
            }
        }

        let adapter = BotlibLlmAdapter(Some(Arc::new(FakeLlm)));
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let fut = adapter.generate_stream("hi", &serde_json::json!({}), tx, "model", "key", None);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let res = rt.block_on(fut);
        assert!(res.is_ok());
        let chunk = rt.block_on(rx.recv());
        assert_eq!(chunk, Some("chunk-1".to_string()));
    }
}