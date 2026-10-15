pub(crate) use anyhow::{anyhow, Result};
pub(crate) use regex::Regex;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::{info, warn};
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpConfig {
    pub enabled: bool,
    pub scan_inbound: bool,
    pub scan_outbound: bool,
    pub block_on_violation: bool,
    pub log_violations: bool,
    pub redact_sensitive_data: bool,
    pub max_content_length: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensitiveDataType {
    Email,
    PhoneNumber,
    CreditCard,
    Ssn,
    IpAddress,
    ApiKey,
    Password,
    AwsAccessKey,
    AwsSecretKey,
    PrivateKey,
    JwtToken,
    BankAccount,
    PassportNumber,
    DriversLicense,
    HealthId,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum DlpSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DlpAction {
    Allow,
    Warn,
    Redact,
    Block,
    Quarantine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpMatch {
    pub data_type: SensitiveDataType,
    pub matched_text: String,
    pub start_position: usize,
    pub end_position: usize,
    pub confidence: f32,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpScanResult {
    pub matches: Vec<DlpMatch>,
    pub severity: DlpSeverity,
    pub action: DlpAction,
    pub blocked: bool,
    pub redacted_content: Option<String>,
    pub policy_violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpPolicy {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub data_types: Vec<SensitiveDataType>,
    pub action: DlpAction,
    pub severity_threshold: DlpSeverity,
    pub exceptions: Vec<DlpException>,
    pub applies_to: PolicyScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyScope {
    All,
    Inbound,
    Outbound,
    Users(Vec<Uuid>),
    Channels(Vec<String>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DlpException {
    User(Uuid),
    Pattern(String),
    Domain(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpViolation {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub policy_id: Uuid,
    pub policy_name: String,
    pub user_id: Option<Uuid>,
    pub data_types: Vec<SensitiveDataType>,
    pub action_taken: DlpAction,
    pub severity: DlpSeverity,
    pub content_hash: String,
    pub match_count: usize,
}

pub struct DlpManager {
    config: DlpConfig,
    patterns: HashMap<SensitiveDataType, Vec<Regex>>,
    policies: Arc<RwLock<Vec<DlpPolicy>>>,
    violations: Arc<RwLock<Vec<DlpViolation>>>,
    custom_patterns: Arc<RwLock<Vec<(String, Regex, SensitiveDataType)>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDirection {
    Inbound,
    Outbound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_detection() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "Contact me at test@example.com for more info";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::Email));
    }

    #[tokio::test]
    async fn test_credit_card_detection() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "My card number is 4111-1111-1111-1111";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::CreditCard));
    }

    #[tokio::test]
    async fn test_ssn_detection() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "SSN: 123-45-6789";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::Ssn));
    }

    #[tokio::test]
    async fn test_aws_key_detection() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "AWS key: AKIAIOSFODNN7EXAMPLE";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::AwsAccessKey));
    }

    #[tokio::test]
    async fn test_jwt_detection() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::JwtToken));
    }

    #[tokio::test]
    async fn test_clean_content() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "This is just a regular message with no sensitive data.";
        let result = manager.scan(content).await;

        assert!(!result.has_sensitive_data());
        assert_eq!(result.action, DlpAction::Allow);
    }

    #[tokio::test]
    async fn test_redaction() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "Email me at user@example.com please";
        let result = manager.scan(content).await;

        assert!(result.redacted_content.is_some());
        let redacted = result.redacted_content.unwrap();
        assert!(redacted.contains("[EMAIL REDACTED]"));
        assert!(!redacted.contains("user@example.com"));
    }

    #[tokio::test]
    async fn test_partial_redaction() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "Card: 4111111111111111";
        let result = manager.scan(content).await;

        let partial = manager.partial_redact_content(content, &result.matches);
        assert!(partial.contains("41"));
        assert!(partial.contains("11"));
        assert!(partial.contains("*"));
    }

    #[tokio::test]
    async fn test_multiple_matches() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let content = "Contact: test@example.com, phone: 555-123-4567, card: 4111111111111111";
        let result = manager.scan(content).await;

        assert!(result.match_count() >= 3);
        assert!(result.data_types_found().contains(&SensitiveDataType::Email));
        assert!(result.data_types_found().contains(&SensitiveDataType::PhoneNumber));
        assert!(result.data_types_found().contains(&SensitiveDataType::CreditCard));
    }

    #[tokio::test]
    async fn test_policy_evaluation() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        let policy = DlpPolicy::new(
            "Block SSN",
            vec![SensitiveDataType::Ssn],
            DlpAction::Block,
        );
        manager.add_policy(policy).await;

        let content = "SSN: 123-45-6789";
        let result = manager.scan(content).await;

        assert!(result.blocked);
        assert_eq!(result.action, DlpAction::Block);
    }

    #[test]
    fn test_credit_card_checksum_valid() {
        assert!(validate_credit_card_checksum("4111111111111111"));
        assert!(validate_credit_card_checksum("5500000000000004"));
    }

    #[test]
    fn test_credit_card_checksum_invalid() {
        assert!(!validate_credit_card_checksum("4111111111111112"));
        assert!(!validate_credit_card_checksum("1234567890123456"));
    }

    #[test]
    fn test_severity_levels() {
        assert_eq!(SensitiveDataType::CreditCard.severity(), DlpSeverity::Critical);
        assert_eq!(SensitiveDataType::Email.severity(), DlpSeverity::Medium);
        assert_eq!(SensitiveDataType::IpAddress.severity(), DlpSeverity::Low);
    }

    #[test]
    fn test_dlp_match_masking() {
        let dlp_match = DlpMatch::new(
            SensitiveDataType::CreditCard,
            "4111111111111111",
            0,
            16,
        );

        let masked = dlp_match.masked_value();
        assert!(masked.starts_with("41"));
        assert!(masked.ends_with("11"));
        assert!(masked.contains("*"));
    }

    #[test]
    fn test_default_policies() {
        let policies = create_default_policies();

        assert!(!policies.is_empty());
        assert!(policies.iter().any(|p| p.name == "Block Credit Cards"));
        assert!(policies.iter().any(|p| p.name == "Block SSN"));
    }

    #[tokio::test]
    async fn test_custom_pattern() {
        let manager = DlpManager::with_defaults().expect("Failed to create manager");

        manager
            .add_custom_pattern("Custom ID", r"CUSTOM-\d{6}", SensitiveDataType::Custom)
            .await
            .expect("Failed to add pattern");

        let content = "ID: CUSTOM-123456";
        let result = manager.scan(content).await;

        assert!(result.has_sensitive_data());
        assert!(result.data_types_found().contains(&SensitiveDataType::Custom));
    }

    #[test]
    fn test_policy_exception() {
        let mut policy = DlpPolicy::new(
            "Test Policy",
            vec![SensitiveDataType::Email],
            DlpAction::Block,
        );

        policy.add_exception(DlpException::Domain("@internal.com".into()));

        assert!(policy.is_exception("user@internal.com", None));
        assert!(!policy.is_exception("user@external.com", None));
    }

    #[test]
    fn test_extract_context() {
        let content = "This is a test email@example.com in the middle";
        let context = extract_context(content, 15, 33, 10);

        assert!(context.contains("email@example.com"));
        assert!(context.len() < content.len() + 10);
    }

    #[tokio::test]
    async fn test_scan_direction() {
        let config = DlpConfig {
            scan_inbound: false,
            ..Default::default()
        };
        let manager = DlpManager::new(config).expect("Failed to create manager");

        let content = "Email: test@example.com";
        let user_id = Uuid::new_v4();

        let inbound_result = manager
            .scan_with_user(content, user_id, ScanDirection::Inbound)
            .await;
        assert!(!inbound_result.has_sensitive_data());

        let outbound_result = manager
            .scan_with_user(content, user_id, ScanDirection::Outbound)
            .await;
        assert!(outbound_result.has_sensitive_data());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;
mod part10;

pub use part10::*;
