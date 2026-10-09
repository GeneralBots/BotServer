use super::*;

impl PromptSecurityManager {
    pub fn create_safe_prompt(&self, user_input: &str, system_prompt: &str) -> Result<String> {
        let detection = self.analyze_input(user_input);

        if detection.blocked {
            return Err(anyhow!(
                "Input blocked due to potential prompt injection: {:?}",
                detection.injection_types
            ));
        }

        let safe_input = detection.sanitized_input.as_deref().unwrap_or(user_input);
        let wrapped_system = self.wrap_system_prompt(system_prompt);

        Ok(format!(
            "{wrapped_system}\n\n<|user_input_start|>\n{safe_input}\n<|user_input_end|>"
        ))
    }

    pub async fn add_blocked_term(&self, term: String) {
        let mut terms = self.blocked_terms.write().await;
        terms.insert(term.to_lowercase());
    }

    pub async fn remove_blocked_term(&self, term: &str) {
        let mut terms = self.blocked_terms.write().await;
        terms.remove(&term.to_lowercase());
    }

    pub async fn add_custom_pattern(&self, pattern: &str, description: &str) -> Result<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| anyhow!("Invalid regex pattern: {e}"))?;

        let mut patterns = self.custom_patterns.write().await;
        patterns.push((regex, description.to_string()));

        info!("Added custom security pattern: {description}");
        Ok(())
    }

    pub fn config(&self) -> &PromptSecurityConfig {
        &self.config
    }
}
