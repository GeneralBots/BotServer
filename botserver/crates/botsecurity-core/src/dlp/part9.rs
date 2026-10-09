use super::*;

impl DlpManager {
    pub fn redact_content(&self, content: &str, matches: &[DlpMatch]) -> String {
        let mut redacted = content.to_string();
        let mut sorted_matches: Vec<&DlpMatch> = matches.iter().collect();
        sorted_matches.sort_by_key(|b| std::cmp::Reverse(b.start_position));

        for dlp_match in sorted_matches {
            let redaction = dlp_match.data_type.redaction_pattern();
            redacted.replace_range(dlp_match.start_position..dlp_match.end_position, redaction);
        }

        redacted
    }

    pub fn partial_redact_content(&self, content: &str, matches: &[DlpMatch]) -> String {
        let mut redacted = content.to_string();
        let mut sorted_matches: Vec<&DlpMatch> = matches.iter().collect();
        sorted_matches.sort_by_key(|b| std::cmp::Reverse(b.start_position));

        for dlp_match in sorted_matches {
            let masked = dlp_match.masked_value();
            redacted.replace_range(dlp_match.start_position..dlp_match.end_position, &masked);
        }

        redacted
    }

    pub async fn add_policy(&self, policy: DlpPolicy) {
        let mut policies = self.policies.write().await;
        policies.push(policy);
    }

    pub async fn remove_policy(&self, policy_id: Uuid) -> bool {
        let mut policies = self.policies.write().await;
        let initial_len = policies.len();
        policies.retain(|p| p.id != policy_id);
        policies.len() < initial_len
    }

    pub async fn get_policies(&self) -> Vec<DlpPolicy> {
        let policies = self.policies.read().await;
        policies.clone()
    }

    pub async fn add_custom_pattern(
        &self,
        name: &str,
        pattern: &str,
        data_type: SensitiveDataType,
    ) -> Result<()> {
        let regex =
            Regex::new(pattern).map_err(|e| anyhow!("Invalid regex pattern: {e}"))?;

        let mut patterns = self.custom_patterns.write().await;
        patterns.push((name.to_string(), regex, data_type));

        info!("Added custom DLP pattern: {}", name);
        Ok(())
    }

    pub async fn record_violation(
        &self,
        policy_id: Uuid,
        policy_name: &str,
        user_id: Option<Uuid>,
        result: &DlpScanResult,
        content: &str,
    ) {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let content_hash = hex::encode(hasher.finalize());

        let violation = DlpViolation {
            id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            policy_id,
            policy_name: policy_name.to_string(),
            user_id,
            data_types: result.data_types_found().into_iter().collect(),
            action_taken: result.action,
            severity: result.severity,
            content_hash,
            match_count: result.match_count(),
        };

        let mut violations = self.violations.write().await;
        violations.push(violation);
    }

    pub async fn get_violations(
        &self,
        limit: usize,
        user_id: Option<Uuid>,
    ) -> Vec<DlpViolation> {
        let violations = self.violations.read().await;

        let filtered: Vec<DlpViolation> = if let Some(uid) = user_id {
            violations
                .iter()
                .filter(|v| v.user_id == Some(uid))
                .cloned()
                .collect()
        } else {
            violations.clone()
        };

        filtered.into_iter().rev().take(limit).collect()
    }

    pub fn config(&self) -> &DlpConfig {
        &self.config
    }
}
