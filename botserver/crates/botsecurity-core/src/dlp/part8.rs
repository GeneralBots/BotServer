use super::*;

impl DlpManager {
    pub fn new(config: DlpConfig) -> Result<Self> {
        let patterns = Self::build_detection_patterns()?;

        Ok(Self {
            config,
            patterns,
            policies: Arc::new(RwLock::new(Vec::new())),
            violations: Arc::new(RwLock::new(Vec::new())),
            custom_patterns: Arc::new(RwLock::new(Vec::new())),
        })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(DlpConfig::default())
    }

    pub(super) fn build_detection_patterns() -> Result<HashMap<SensitiveDataType, Vec<Regex>>> {
        let mut patterns = HashMap::new();

        patterns.insert(
            SensitiveDataType::Email,
            vec![Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b")?],
        );

        patterns.insert(
            SensitiveDataType::PhoneNumber,
            vec![
                Regex::new(r"\b\d{3}[-.\s]?\d{3}[-.\s]?\d{4}\b")?,
                Regex::new(r"\b\+\d{1,3}[-.\s]?\d{1,4}[-.\s]?\d{1,4}[-.\s]?\d{1,9}\b")?,
                Regex::new(r"\(\d{3}\)\s*\d{3}[-.\s]?\d{4}")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::CreditCard,
            vec![
                Regex::new(r"\b4\d{3}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")?,
                Regex::new(r"\b5[1-5]\d{2}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")?,
                Regex::new(r"\b3[47]\d{2}[-\s]?\d{6}[-\s]?\d{5}\b")?,
                Regex::new(r"\b6(?:011|5\d{2})[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::Ssn,
            vec![
                Regex::new(r"\b\d{3}[-\s]?\d{2}[-\s]?\d{4}\b")?,
                Regex::new(r"(?i)\bssn[:\s]*\d{3}[-\s]?\d{2}[-\s]?\d{4}\b")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::IpAddress,
            vec![
                Regex::new(r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b")?,
                Regex::new(r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::ApiKey,
            vec![
                Regex::new(r#"(?i)(?:api[_-]?key|apikey)["\s:=]+["']?([a-zA-Z0-9_\-]{20,})["']?"#)?,
                Regex::new(r#"(?i)(?:secret|token)[_\-]?key[:\s=]+['\"]?([a-zA-Z0-9_\-]{16,})['\"]?"#)?,
            ],
        );

        patterns.insert(
            SensitiveDataType::Password,
            vec![
                Regex::new(r#"(?i)password["\s:=]+["']?([^\s"']{8,})["']?"#)?,
                Regex::new(r#"(?i)(?:passwd|pwd)[:\s=]+['\"]?([^\s'\"]{8,})['\"]?"#)?,
            ],
        );

        patterns.insert(
            SensitiveDataType::AwsAccessKey,
            vec![Regex::new(r"\b(?:AKIA|ABIA|ACCA|ASIA)[0-9A-Z]{16}\b")?],
        );

        patterns.insert(
            SensitiveDataType::AwsSecretKey,
            vec![Regex::new(r#"(?i)aws[_\-]?secret[_\-]?(?:access[_\-]?)?key[:\s=]+['\"]?([a-zA-Z0-9/+=]{40})['\"]?"#)?],
        );

        patterns.insert(
            SensitiveDataType::PrivateKey,
            vec![
                Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----")?,
                Regex::new(r"-----BEGIN PGP PRIVATE KEY BLOCK-----")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::JwtToken,
            vec![Regex::new(r"\beyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*\b")?],
        );

        patterns.insert(
            SensitiveDataType::BankAccount,
            vec![
                Regex::new(r#"(?i)(?:account|acct)[_\-\s]?(?:number|no|#)?[:\s]*\d{8,17}"#)?,
                Regex::new(r#"(?i)(?:routing|aba)[_\-\s]?(?:number|no|#)?[:\s]*\d{9}"#)?,
                Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{4}\d{7}(?:[A-Z0-9]?){0,16}\b")?,
            ],
        );

        patterns.insert(
            SensitiveDataType::PassportNumber,
            vec![Regex::new(r"(?i)passport[_\-\s]?(?:number|no|#)?[:\s]*[A-Z0-9]{6,9}")?],
        );

        patterns.insert(
            SensitiveDataType::DriversLicense,
            vec![Regex::new(r"(?i)(?:driver'?s?|dl)[_\-\s]?(?:license|lic)[_\-\s]?(?:number|no|#)?[:\s]*[A-Z0-9]{5,15}")?],
        );

        patterns.insert(
            SensitiveDataType::HealthId,
            vec![
                Regex::new(r"(?i)(?:medicare|medicaid)[_\-\s]?(?:id|number|no|#)?[:\s]*[A-Z0-9]{10,12}")?,
                Regex::new(r"(?i)(?:health|medical)[_\-\s]?(?:id|record)[_\-\s]?(?:number|no|#)?[:\s]*[A-Z0-9]{8,15}")?,
            ],
        );

        Ok(patterns)
    }

    pub async fn scan(&self, content: &str) -> DlpScanResult {
        if !self.config.enabled {
            return DlpScanResult::clean();
        }

        if content.len() > self.config.max_content_length {
            return DlpScanResult {
                matches: Vec::new(),
                severity: DlpSeverity::Medium,
                action: DlpAction::Block,
                blocked: true,
                redacted_content: None,
                policy_violations: vec!["Content exceeds maximum allowed length".into()],
            };
        }

        let mut all_matches = Vec::new();

        for (data_type, patterns) in &self.patterns {
            for pattern in patterns {
                for mat in pattern.find_iter(content) {
                    let context = extract_context(content, mat.start(), mat.end(), 20);
                    let dlp_match = DlpMatch::new(*data_type, mat.as_str(), mat.start(), mat.end())
                        .with_context(context);
                    all_matches.push(dlp_match);
                }
            }
        }

        let custom_patterns = self.custom_patterns.read().await;
        for (_, pattern, data_type) in custom_patterns.iter() {
            for mat in pattern.find_iter(content) {
                let context = extract_context(content, mat.start(), mat.end(), 20);
                let dlp_match = DlpMatch::new(*data_type, mat.as_str(), mat.start(), mat.end())
                    .with_context(context);
                all_matches.push(dlp_match);
            }
        }
        drop(custom_patterns);

        if all_matches.is_empty() {
            return DlpScanResult::clean();
        }

        let severity = all_matches
            .iter()
            .map(|m| m.data_type.severity())
            .max()
            .unwrap_or(DlpSeverity::Low);

        let (action, blocked, policy_violations) =
            self.evaluate_policies(&all_matches, None).await;

        let redacted_content = if self.config.redact_sensitive_data {
            Some(self.redact_content(content, &all_matches))
        } else {
            None
        };

        if self.config.log_violations && !all_matches.is_empty() {
            warn!(
                "DLP scan found {} sensitive data matches, severity: {}, action: {}",
                all_matches.len(),
                severity.as_str(),
                action.as_str()
            );
        }

        DlpScanResult {
            matches: all_matches,
            severity,
            action,
            blocked,
            redacted_content,
            policy_violations,
        }
    }

    pub async fn scan_with_user(
        &self,
        content: &str,
        user_id: Uuid,
        direction: ScanDirection,
    ) -> DlpScanResult {
        if !self.config.enabled {
            return DlpScanResult::clean();
        }

        match direction {
            ScanDirection::Inbound if !self.config.scan_inbound => return DlpScanResult::clean(),
            ScanDirection::Outbound if !self.config.scan_outbound => return DlpScanResult::clean(),
            _ => {}
        }

        let mut result = self.scan(content).await;

        if result.has_sensitive_data() {
            let (action, blocked, violations) =
                self.evaluate_policies(&result.matches, Some(user_id)).await;
            result.action = action;
            result.blocked = blocked;
            result.policy_violations = violations;
        }

        result
    }

    async fn evaluate_policies(
        &self,
        matches: &[DlpMatch],
        user_id: Option<Uuid>,
    ) -> (DlpAction, bool, Vec<String>) {
        let policies = self.policies.read().await;
        let mut highest_action = DlpAction::Allow;
        let mut blocked = false;
        let mut violations = Vec::new();

        for policy in policies.iter().filter(|p| p.enabled) {
            for dlp_match in matches {
                if !policy.matches_data_type(dlp_match.data_type) {
                    continue;
                }

                if dlp_match.data_type.severity() < policy.severity_threshold {
                    continue;
                }

                if policy.is_exception(&dlp_match.matched_text, user_id) {
                    continue;
                }

                violations.push(format!(
                    "Policy '{}' violated: {} detected",
                    policy.name,
                    dlp_match.data_type.as_str()
                ));

                if policy.action as u8 > highest_action as u8 {
                    highest_action = policy.action;
                }

                if policy.action == DlpAction::Block || policy.action == DlpAction::Quarantine {
                    blocked = true;
                }
            }
        }

        if violations.is_empty() && self.config.block_on_violation && !matches.is_empty() {
            let critical_found = matches
                .iter()
                .any(|m| m.data_type.severity() >= DlpSeverity::Critical);

            if critical_found {
                blocked = true;
                highest_action = DlpAction::Block;
                violations.push("Critical sensitive data detected".into());
            }
        }

        (highest_action, blocked, violations)
    }
}
