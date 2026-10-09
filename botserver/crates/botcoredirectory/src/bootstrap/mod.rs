pub(crate) use anyhow::Result;
pub(crate) use log::{info, warn};
pub(crate) use rand::Rng;
pub(crate) use std::fs;
pub(crate) use botcore::shared::utils::get_stack_path;
pub(crate) use uuid::Uuid;
pub(crate) use super::client::ZitadelClient;

const ADMIN_USERNAME: &str = "admin";
const DEFAULT_ORG_NAME: &str = "General Bots";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BootstrapState {
    pub user_id: String,
    pub organization_id: Option<String>,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

pub struct BootstrapResult {
    pub user_id: String,
    pub organization_id: Option<String>,
    pub username: String,
    pub email: String,
    pub initial_password: String,
    pub setup_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_password() {
        let password = generate_secure_password();

        assert!(password.len() >= 14);

        let has_lower = password.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = password.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| "!@#$%&*".contains(c));

        assert!(has_lower, "Password should contain lowercase");
        assert!(has_upper, "Password should contain uppercase");
        assert!(has_digit, "Password should contain digits");
        assert!(has_special, "Password should contain special chars");
    }

    #[test]
    fn test_password_uniqueness() {
        let passwords: Vec<String> = (0..10).map(|_| generate_secure_password()).collect();

        for i in 0..passwords.len() {
            for j in (i + 1)..passwords.len() {
                assert_ne!(
                    passwords[i], passwords[j],
                    "Generated passwords should be unique"
                );
            }
        }
    }
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
