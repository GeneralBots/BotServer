pub(crate) use anyhow::{anyhow, Result};
pub(crate) use argon2::{ password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Algorithm, Argon2, Params, Version, };
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashSet;
pub(crate) use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digit: bool,
    pub require_special: bool,
    pub min_unique_chars: usize,
    pub max_consecutive_chars: usize,
    pub password_history_count: usize,
    pub expiration_days: Option<u32>,
    pub lockout_threshold: u32,
    pub lockout_duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2Config {
    pub memory_cost_kib: u32,
    pub time_cost: u32,
    pub parallelism: u32,
    pub output_length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PasswordStrength {
    VeryWeak,
    Weak,
    Fair,
    Strong,
    VeryStrong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordValidationResult {
    pub is_valid: bool,
    pub strength: PasswordStrength,
    pub score: u8,
    pub issues: Vec<PasswordIssue>,
    pub suggestions: Vec<String>,
    pub crack_time_display: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PasswordIssue {
    TooShort { min: usize, actual: usize },
    TooLong { max: usize, actual: usize },
    MissingUppercase,
    MissingLowercase,
    MissingDigit,
    MissingSpecial,
    InsufficientUniqueChars { min: usize, actual: usize },
    TooManyConsecutiveChars { max: usize },
    CommonPassword,
    ContainsUsername,
    ContainsEmail,
    RecentlyUsed,
    Compromised,
}

pub struct PasswordHasher2 {
    argon2: Argon2<'static>,
    config: PasswordConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::part5::*;

    #[test]
    fn test_hash_and_verify() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let password = "SecureP@ssw0rd123!";
        let hash = hasher.hash(password).expect("Failed to hash");

        assert!(hasher.verify(password, &hash).expect("Verify failed"));
        assert!(!hasher
            .verify("WrongPassword", &hash)
            .expect("Verify failed"));
    }

    #[test]
    fn test_password_validation_success() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let result = hasher.validate("Tr0ub4dor!&3v", None, None, &[]);

        assert!(result.is_valid);
        assert!(result.issues.is_empty());
        assert!(result.strength.is_acceptable());
    }

    #[test]
    fn test_password_too_short() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let result = hasher.validate("Short1!", None, None, &[]);

        assert!(!result.is_valid);
        assert!(result.issues.iter().any(|i| matches!(i, PasswordIssue::TooShort { .. })));
    }

    #[test]
    fn test_common_password_detection() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let result = hasher.validate("password123", None, None, &[]);

        assert!(!result.is_valid);
        assert!(result
            .issues
            .iter()
            .any(|i| matches!(i, PasswordIssue::CommonPassword)));
    }

    #[test]
    fn test_username_in_password() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let result = hasher.validate("JohnDoe2024!Secure", Some("johndoe"), None, &[]);

        assert!(result
            .issues
            .iter()
            .any(|i| matches!(i, PasswordIssue::ContainsUsername)));
    }

    #[test]
    fn test_password_strength_levels() {
        assert_eq!(PasswordStrength::VeryWeak.score(), 0);
        assert_eq!(PasswordStrength::Weak.score(), 1);
        assert_eq!(PasswordStrength::Fair.score(), 2);
        assert_eq!(PasswordStrength::Strong.score(), 3);
        assert_eq!(PasswordStrength::VeryStrong.score(), 4);
    }

    #[test]
    fn test_generate_secure_password() {
        let password = generate_secure_password(20);

        assert_eq!(password.len(), 20);
        assert!(password.chars().any(|c| c.is_uppercase()));
        assert!(password.chars().any(|c| c.is_lowercase()));
        assert!(password.chars().any(|c| c.is_ascii_digit()));
        assert!(password.chars().any(|c| !c.is_alphanumeric()));
    }

    #[test]
    fn test_generate_recovery_codes() {
        let codes = generate_recovery_codes(10);

        assert_eq!(codes.len(), 10);
        for code in &codes {
            assert_eq!(code.len(), 9);
            assert!(code.contains('-'));
        }
    }

    #[test]
    fn test_needs_rehash() {
        let hasher = PasswordHasher2::with_defaults().expect("Failed to create hasher");
        let hash = hasher.hash("TestPassword123!").expect("Failed to hash");

        assert!(!hasher.needs_rehash(&hash).expect("Rehash check failed"));
    }

    #[test]
    fn test_consecutive_chars_detection() {
        assert!(has_consecutive_chars("aaaa", 3));
        assert!(!has_consecutive_chars("aaa", 3));
        assert!(!has_consecutive_chars("abcd", 3));
    }

    #[test]
    fn test_helper_functions() {
        let test_pass = String::from_utf8_lossy(b"TestP@ssw0rd!").to_string();
        let hash = hash_password(&test_pass).expect("Hash failed");
        assert!(verify_password(&test_pass, &hash).expect("Verify failed"));
 
        let weak_pass = String::from_utf8_lossy(b"V!ctoryL4mpSt0ne").to_string();
        let result = validate_password(&weak_pass);
        assert!(result.strength.score() > 0, "score should be positive");
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
