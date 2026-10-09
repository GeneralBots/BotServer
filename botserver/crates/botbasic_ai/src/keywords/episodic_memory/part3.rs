use super::*;

impl EpisodicMemoryManager {
    pub fn new(config: EpisodicMemoryConfig) -> Self {
        Self { config }
    }

    pub fn from_config(config_map: &std::collections::HashMap<String, String>) -> Self {
        let config = EpisodicMemoryConfig {
            enabled: config_map
                .get("episodic-memory-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            threshold: config_map
                .get("episodic-memory-threshold")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            history: config_map
                .get("episodic-memory-history")
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            model: config_map
                .get("episodic-memory-model")
                .cloned()
                .unwrap_or_else(|| "fast".to_string()),
            max_episodes: config_map
                .get("episodic-memory-max-episodes")
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            retention_days: config_map
                .get("episodic-memory-retention-days")
                .and_then(|v| v.parse().ok())
                .unwrap_or(365),
            auto_summarize: config_map
                .get("episodic-memory-auto-summarize")
                .map(|v| v == "true")
                .unwrap_or(true),
        };
        Self::new(config)
    }

    pub fn should_summarize(&self, message_count: usize) -> bool {
        self.config.enabled && self.config.auto_summarize && message_count >= self.config.threshold
    }

    pub fn get_history_to_keep(&self) -> usize {
        self.config.history
    }

    pub fn get_threshold(&self) -> usize {
        self.config.threshold
    }

    pub fn generate_summary_prompt(&self, messages: &[ConversationMessage]) -> String {
        let formatted_messages = messages
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

        format!(
            r#"Analyze and summarize this conversation. Extract key information.

CONVERSATION:
{}

Respond with valid JSON only:
{{
    "summary": "A concise 2-3 sentence summary of the conversation",
    "key_topics": ["topic1", "topic2"],
    "decisions": ["decision1", "decision2"],
    "action_items": [
        {{"description": "action description", "assignee": "user/bot/null", "priority": "low/medium/high/critical"}}
    ],
    "sentiment": {{
        "score": 0.0,
        "label": "very_negative/negative/neutral/positive/very_positive",
        "confidence": 0.8
    }},
    "resolution": "resolved/unresolved/escalated/pending/unknown"
}}"#,
            formatted_messages
        )
    }

    pub fn parse_summary_response(
        &self,
        response: &str,
        messages: &[ConversationMessage],
        user_id: Uuid,
        bot_id: Uuid,
        session_id: Uuid,
    ) -> Result<Episode, String> {
        let json_str = extract_json(response)?;

        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let summary = parsed["summary"]
            .as_str()
            .unwrap_or("Conversation summary unavailable")
            .to_string();

        let key_topics: Vec<String> = parsed["key_topics"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let decisions: Vec<String> = parsed["decisions"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let action_items: Vec<ActionItem> = parsed["action_items"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        Some(ActionItem {
                            description: v["description"].as_str()?.to_string(),
                            assignee: v["assignee"].as_str().map(String::from),
                            due_date: None,
                            priority: match v["priority"].as_str().unwrap_or("medium") {
                                "low" => Priority::Low,
                                "high" => Priority::High,
                                "critical" => Priority::Critical,
                                _ => Priority::Medium,
                            },
                            completed: false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let sentiment = Sentiment {
            score: parsed["sentiment"]["score"].as_f64().unwrap_or(0.0),
            label: match parsed["sentiment"]["label"].as_str().unwrap_or("neutral") {
                "very_negative" => SentimentLabel::VeryNegative,
                "negative" => SentimentLabel::Negative,
                "positive" => SentimentLabel::Positive,
                "very_positive" => SentimentLabel::VeryPositive,
                _ => SentimentLabel::Neutral,
            },
            confidence: parsed["sentiment"]["confidence"].as_f64().unwrap_or(0.5),
        };

        let resolution = match parsed["resolution"].as_str().unwrap_or("unknown") {
            "resolved" => ResolutionStatus::Resolved,
            "unresolved" => ResolutionStatus::Unresolved,
            "escalated" => ResolutionStatus::Escalated,
            "pending" => ResolutionStatus::Pending,
            _ => ResolutionStatus::Unknown,
        };

        let conversation_start = messages
            .first()
            .map(|m| m.timestamp)
            .unwrap_or_else(Utc::now);
        let conversation_end = messages
            .last()
            .map(|m| m.timestamp)
            .unwrap_or_else(Utc::now);

        Ok(Episode {
            id: Uuid::new_v4(),
            user_id,
            bot_id,
            session_id,
            summary,
            key_topics,
            decisions,
            action_items,
            sentiment,
            resolution,
            message_count: messages.len(),
            message_ids: messages.iter().map(|m| m.id).collect(),
            created_at: Utc::now(),
            conversation_start,
            conversation_end,
            metadata: serde_json::json!({}),
        })
    }

    pub fn get_retention_cutoff(&self) -> DateTime<Utc> {
        Utc::now() - Duration::days(i64::from(self.config.retention_days))
    }
}
pub fn extract_json(response: &str) -> Result<String, String> {
    if let Some(start) = response.find("```json") {
        if let Some(end) = response[start + 7..].find("```") {
            return Ok(response[start + 7..start + 7 + end].trim().to_string());
        }
    }

    if let Some(start) = response.find("```") {
        let after_start = start + 3;

        let json_start = response[after_start..]
            .find('\n')
            .map(|i| after_start + i + 1)
            .unwrap_or(after_start);
        if let Some(end) = response[json_start..].find("```") {
            return Ok(response[json_start..json_start + end].trim().to_string());
        }
    }

    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            if end > start {
                return Ok(response[start..=end].to_string());
            }
        }
    }

    Err("No JSON found in response".to_string())
}
