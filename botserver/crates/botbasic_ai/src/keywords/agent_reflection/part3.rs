use super::*;

impl ReflectionResult {
    pub fn new(bot_id: Uuid, session_id: Uuid, reflection_type: ReflectionType) -> Self {
        Self {
            id: Uuid::new_v4(),
            bot_id,
            session_id,
            reflection_type,
            score: 0.0,
            insights: Vec::new(),
            improvements: Vec::new(),
            positive_patterns: Vec::new(),
            concerns: Vec::new(),
            raw_response: String::new(),
            timestamp: chrono::Utc::now(),
            messages_analyzed: 0,
        }
    }

    pub fn from_llm_response(
        bot_id: Uuid,
        session_id: Uuid,
        reflection_type: ReflectionType,
        response: &str,
        messages_analyzed: usize,
    ) -> Self {
        let mut result = Self::new(bot_id, session_id, reflection_type);
        result.raw_response = response.to_string();
        result.messages_analyzed = messages_analyzed;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(response) {
            result.score = json
                .get("score")
                .or_else(|| json.get("overall_score"))
                .and_then(|v| v.as_f64())
                .unwrap_or(5.0) as f32;

            if let Some(insights) = json.get("key_insights").or_else(|| json.get("insights")) {
                if let Some(arr) = insights.as_array() {
                    result.insights = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            if let Some(improvements) = json
                .get("improvements")
                .or_else(|| json.get("critical_improvements"))
                .or_else(|| json.get("recommendations"))
            {
                if let Some(arr) = improvements.as_array() {
                    result.improvements = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            if let Some(patterns) = json
                .get("positive_patterns")
                .or_else(|| json.get("strengths"))
                .or_else(|| json.get("effective_uses"))
            {
                if let Some(arr) = patterns.as_array() {
                    result.positive_patterns = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }

            if let Some(concerns) = json
                .get("weaknesses")
                .or_else(|| json.get("concerns"))
                .or_else(|| json.get("factual_errors"))
                .or_else(|| json.get("knowledge_gaps"))
            {
                if let Some(arr) = concerns.as_array() {
                    result.concerns = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
        } else {
            warn!("Reflection response was not valid JSON, extracting from plain text");
            result.insights = extract_insights_from_text(response);
            result.score = 5.0;
        }

        result
    }

    pub fn needs_improvement(&self, threshold: f32) -> bool {
        self.score < threshold
    }

    pub fn summary(&self) -> String {
        format!(
            "Reflection Score: {:.1}/10\n\
             Messages Analyzed: {}\n\
             Key Insights: {}\n\
             Improvements Needed: {}\n\
             Positive Patterns: {}",
            self.score,
            self.messages_analyzed,
            self.insights.len(),
            self.improvements.len(),
            self.positive_patterns.len()
        )
    }
}
pub fn extract_insights_from_text(text: &str) -> Vec<String> {
    let mut insights = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(|c: char| c.is_ascii_digit())
            || trimmed.starts_with('-')
            || trimmed.starts_with('•')
            || trimmed.starts_with('*')
        {
            let cleaned = trimmed
                .trim_start_matches(|c: char| {
                    c.is_ascii_digit() || c == '.' || c == '-' || c == '•' || c == '*'
                })
                .trim();
            if !cleaned.is_empty() && cleaned.len() > 10 {
                insights.push(cleaned.to_string());
            }
        }
    }

    if insights.is_empty() {
        for sentence in text.split(['.', '!', '?']) {
            let trimmed = sentence.trim();
            if trimmed.len() > 20 && trimmed.len() < 200 {
                insights.push(format!("{}.", trimmed));
            }
        }
    }

    insights.truncate(10);
    insights
}
