use super::*;

impl ReflectionEngine {
    pub fn new(state: Arc<dyn BasicRuntime>, bot_id: Uuid) -> Self {
        let config = ReflectionConfig::from_bot_config(&state, bot_id);
        Self {
            state,
            config,
            bot_id,
        }
    }

    pub fn with_config(state: Arc<dyn BasicRuntime>, bot_id: Uuid, config: ReflectionConfig) -> Self {
        Self {
            state,
            config,
            bot_id,
        }
    }

    pub async fn reflect(
        &self,
        session_id: Uuid,
        reflection_type: ReflectionType,
    ) -> Result<ReflectionResult, String> {
        if !self.config.enabled {
            return Err("Reflection is not enabled for this bot".to_string());
        }

        let history = self.get_recent_history(session_id, 20)?;

        if history.is_empty() {
            return Err("No conversation history to analyze".to_string());
        }

        let messages_count = history.len();

        let prompt = self.build_reflection_prompt(&reflection_type, &history)?;

        let response = self.call_llm_for_reflection(&prompt).await?;

        let result = ReflectionResult::from_llm_response(
            self.bot_id,
            session_id,
            reflection_type,
            &response,
            messages_count,
        );

        self.store_reflection(&result)?;

        if self.config.auto_apply && result.needs_improvement(self.config.improvement_threshold) {
            self.apply_improvements(&result)?;
        }

        info!(
            "Reflection completed for bot {} session {}: score {:.1}",
            self.bot_id, session_id, result.score
        );

        Ok(result)
    }

    pub(super) fn get_recent_history(
        &self,
        session_id: Uuid,
        limit: usize,
    ) -> Result<Vec<ConversationMessage>, String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        #[derive(QueryableByName)]
        struct MessageRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            role: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            content: String,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let messages: Vec<MessageRow> = diesel::sql_query(
            "SELECT role, content, created_at FROM conversation_messages \
             WHERE session_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind::<diesel::sql_types::Uuid, _>(session_id)
        .bind::<diesel::sql_types::Integer, _>(limit as i32)
        .load(&mut conn)
        .unwrap_or_default();

        let history: Vec<ConversationMessage> = messages
            .into_iter()
            .rev()
            .map(|m| ConversationMessage {
                role: m.role,
                content: m.content,
                timestamp: m.created_at,
            })
            .collect();

        Ok(history)
    }

    pub(super) fn build_reflection_prompt(
        &self,
        reflection_type: &ReflectionType,
        history: &[ConversationMessage],
    ) -> Result<String, String> {
        let template = if let Some(custom) = &self.config.custom_prompt {
            custom.clone()
        } else {
            reflection_type.prompt_template()
        };

        let conversation = history
            .iter()
            .map(|m| {
                format!(
                    "[{}] {}: {}",
                    m.timestamp.format("%H:%M"),
                    m.role,
                    m.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = template
            .replace("{conversation}", &conversation)
            .replace("{tools}", "Standard tools available")
            .replace("{context}", "Retrieved context from knowledge base");

        Ok(prompt)
    }

    async fn call_llm_for_reflection(&self, prompt: &str) -> Result<String, String> {
        let (llm_url, llm_model, llm_key) = self.get_llm_config()?;

        let client = reqwest::Client::new();

        let messages = serde_json::json!([
            {
                "role": "system",
                "content": "You are an AI performance analyst. Analyze conversations and provide structured feedback in JSON format. Be objective and constructive."
            },
            {
                "role": "user",
                "content": prompt
            }
        ]);

        let request_body = serde_json::json!({
            "model": llm_model,
            "messages": messages,
            "temperature": 0.3,
            "max_tokens": 1000
        });

        let response = client
            .post(format!("{}/v1/chat/completions", llm_url))
            .header("Authorization", format!("Bearer {}", llm_key))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("LLM error: {}", error_text));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        let content = result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }

    pub(super) fn get_llm_config(&self) -> Result<(String, String, String), String> {
        let vault_configs = read_bot_config_from_vault(&self.bot_id)
            .ok_or_else(|| "Failed to read bot config from Vault".to_string())?;

        let llm_url = vault_configs
            .get("llm-url")
            .cloned()
            .ok_or_else(|| "LLM URL not configured".to_string())?;
        let llm_model = vault_configs
            .get("llm-model")
            .cloned()
            .ok_or_else(|| "LLM model not configured".to_string())?;
        let llm_key = vault_configs
            .get("llm-key")
            .cloned()
            .ok_or_else(|| "LLM key not configured".to_string())?;

        Ok((llm_url, llm_model, llm_key))
    }

    pub(super) fn store_reflection(&self, result: &ReflectionResult) -> Result<(), String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        let insights_json = serde_json::to_string(&result.insights).unwrap_or_default();
        let improvements_json = serde_json::to_string(&result.improvements).unwrap_or_default();
        let patterns_json = serde_json::to_string(&result.positive_patterns).unwrap_or_default();
        let concerns_json = serde_json::to_string(&result.concerns).unwrap_or_default();
        let reflection_type_str =
            serde_json::to_string(&result.reflection_type).unwrap_or_default();

        diesel::sql_query(
            "INSERT INTO bot_reflections \
             (id, bot_id, session_id, reflection_type, score, insights, improvements, \
              positive_patterns, concerns, raw_response, messages_analyzed, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind::<diesel::sql_types::Uuid, _>(result.id)
        .bind::<diesel::sql_types::Uuid, _>(result.bot_id)
        .bind::<diesel::sql_types::Uuid, _>(result.session_id)
        .bind::<diesel::sql_types::Text, _>(&reflection_type_str)
        .bind::<diesel::sql_types::Float, _>(result.score)
        .bind::<diesel::sql_types::Text, _>(&insights_json)
        .bind::<diesel::sql_types::Text, _>(&improvements_json)
        .bind::<diesel::sql_types::Text, _>(&patterns_json)
        .bind::<diesel::sql_types::Text, _>(&concerns_json)
        .bind::<diesel::sql_types::Text, _>(&result.raw_response)
        .bind::<diesel::sql_types::Integer, _>(result.messages_analyzed as i32)
        .bind::<diesel::sql_types::Timestamptz, _>(result.timestamp)
        .execute(&mut conn)
        .map_err(|e| format!("Failed to store reflection: {}", e))?;

        Ok(())
    }

    pub(super) fn apply_improvements(&self, result: &ReflectionResult) -> Result<(), String> {
        let mut conn = self
            .state
            .db_pool()
            .get()
            .map_err(|e| format!("Failed to acquire database connection: {}", e))?;

        for improvement in &result.improvements {
            let key = format!("improvement_{}", Uuid::new_v4());
            let now = chrono::Utc::now();

            diesel::sql_query(
                "INSERT INTO bot_memories (id, bot_id, key, value, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6) \
                 ON CONFLICT (bot_id, key) DO UPDATE SET value = $4, updated_at = $6",
            )
            .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(self.bot_id)
            .bind::<diesel::sql_types::Text, _>(&key)
            .bind::<diesel::sql_types::Text, _>(improvement)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .execute(&mut conn)
            .ok();
        }

        info!(
            "Applied {} improvements for bot {}",
            result.improvements.len(),
            self.bot_id
        );

        Ok(())
    }
}
