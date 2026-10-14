use crate::types::{AutoTaskState, ConfigOps, LlmProviderOps, UserSession};
use crate::task_manifest::TaskManifest;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerSuggestion {
    pub id: String,
    pub category: String,
    pub title: String,
    pub description: String,
    pub confidence: f64,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerAnalysis {
    pub suggestions: Vec<DesignerSuggestion>,
    pub improvements: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct DesignerAI {
    state: Arc<dyn AutoTaskState>,
    config_ops: Arc<dyn ConfigOps>,
    llm_ops: Arc<dyn LlmProviderOps>,
}

impl DesignerAI {
    pub fn new(
        state: Arc<dyn AutoTaskState>,
        config_ops: Arc<dyn ConfigOps>,
        llm_ops: Arc<dyn LlmProviderOps>,
    ) -> Self {
        Self { state, config_ops, llm_ops }
    }

    pub fn state(&self) -> &Arc<dyn AutoTaskState> {
        &self.state
    }

    pub fn config_ops(&self) -> &Arc<dyn ConfigOps> {
        &self.config_ops
    }

    pub fn llm_ops(&self) -> &Arc<dyn LlmProviderOps> {
        &self.llm_ops
    }

    pub async fn analyze_intent(
        &self,
        intent: &str,
        _session: &UserSession,
    ) -> Result<DesignerAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        info!("DesignerAI analyzing intent: {}", botlib::text::truncate_at_char_boundary(intent, 80));
        let prompt = self.build_analysis_prompt(intent);
        let response = self.call_llm(&prompt).await?;
        self.parse_analysis_response(&response)
    }

    pub async fn suggest_improvements(
        &self,
        manifest: &TaskManifest,
    ) -> Result<Vec<DesignerSuggestion>, Box<dyn std::error::Error + Send + Sync>> {
        info!("DesignerAI suggesting improvements for: {}", manifest.app_name);
        let manifest_json = serde_json::to_string(manifest).unwrap_or_default();
        let prompt = format!(
            r#"Analyze this application manifest and suggest improvements.
Manifest: {manifest_json}

Respond with JSON array of suggestions:
[{{"category": "ui|data|logic|security|performance", "title": "...", "description": "...", "confidence": 0.8, "data": {{}}}}]"#
        );
        let response = self.call_llm(&prompt).await?;
        self.parse_suggestions_response(&response)
    }

    pub async fn refine_manifest(
        &self,
        manifest: &mut TaskManifest,
        feedback: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("DesignerAI refining manifest with feedback: {}", botlib::text::truncate_at_char_boundary(feedback, 50));
        let _ = (manifest, feedback);
        Ok(())
    }

    fn build_analysis_prompt(&self, intent: &str) -> String {
        format!(
            r#"You are a software designer. Analyze this user request and provide design suggestions.

USER REQUEST: "{intent}"

Respond with JSON only:
{{
  "suggestions": [
    {{
      "category": "ui|data|logic|security|performance",
      "title": "suggestion title",
      "description": "detailed description",
      "confidence": 0.8,
      "data": {{}}
    }}
  ],
  "improvements": ["improvement1"],
  "warnings": ["warning1"]
}}"#
        )
    }

    #[cfg(feature = "llm")]
    async fn call_llm(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let model = "gpt-4".to_string();
        let key = "".to_string();
        let config = serde_json::json!({"temperature": 0.4, "max_tokens": 2000});
        // Bounded — a stalled model must fail the design step, not hang it.
        crate::types::collect_llm_stream(self.llm_ops.as_ref(), prompt, &config, &model, &key, None)
            .await
    }

    #[cfg(not(feature = "llm"))]
    async fn call_llm(&self, _prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        warn!("LLM feature not enabled for designer AI");
        Ok("{}".to_string())
    }

    fn parse_analysis_response(
        &self,
        response: &str,
    ) -> Result<DesignerAnalysis, Box<dyn std::error::Error + Send + Sync>> {
        let cleaned = response.trim()
            .trim_start_matches("```json").trim_start_matches("```JSON")
            .trim_start_matches("```").trim_end_matches("```").trim();

        #[derive(Deserialize)]
        struct AnalysisResponse {
            suggestions: Option<Vec<SuggestionResponse>>,
            improvements: Option<Vec<String>>,
            warnings: Option<Vec<String>>,
        }
        #[derive(Deserialize)]
        struct SuggestionResponse {
            category: String,
            title: String,
            description: String,
            confidence: Option<f64>,
            data: Option<serde_json::Value>,
        }

        match serde_json::from_str::<AnalysisResponse>(cleaned) {
            Ok(resp) => {
                let suggestions = resp.suggestions.unwrap_or_default().into_iter().map(|s| DesignerSuggestion {
                    id: Uuid::new_v4().to_string(),
                    category: s.category,
                    title: s.title,
                    description: s.description,
                    confidence: s.confidence.unwrap_or(0.5),
                    data: s.data.unwrap_or(serde_json::Value::Null),
                }).collect();
                Ok(DesignerAnalysis {
                    suggestions,
                    improvements: resp.improvements.unwrap_or_default(),
                    warnings: resp.warnings.unwrap_or_default(),
                })
            }
            Err(e) => {
                warn!("Failed to parse designer analysis response: {e}");
                Ok(DesignerAnalysis {
                    suggestions: Vec::new(),
                    improvements: Vec::new(),
                    warnings: vec![format!("Failed to parse AI response: {e}")],
                })
            }
        }
    }

    fn parse_suggestions_response(
        &self,
        response: &str,
    ) -> Result<Vec<DesignerSuggestion>, Box<dyn std::error::Error + Send + Sync>> {
        let cleaned = response.trim()
            .trim_start_matches("```json").trim_start_matches("```JSON")
            .trim_start_matches("```").trim_end_matches("```").trim();

        #[derive(Deserialize)]
        struct SuggestionResponse {
            category: String,
            title: String,
            description: String,
            confidence: Option<f64>,
            data: Option<serde_json::Value>,
        }

        match serde_json::from_str::<Vec<SuggestionResponse>>(cleaned) {
            Ok(items) => {
                Ok(items.into_iter().map(|s| DesignerSuggestion {
                    id: Uuid::new_v4().to_string(),
                    category: s.category,
                    title: s.title,
                    description: s.description,
                    confidence: s.confidence.unwrap_or(0.5),
                    data: s.data.unwrap_or(serde_json::Value::Null),
                }).collect())
            }
            Err(e) => {
                warn!("Failed to parse suggestions response: {e}");
                Ok(Vec::new())
            }
        }
    }
}
