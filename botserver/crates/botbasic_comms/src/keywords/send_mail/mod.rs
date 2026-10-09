pub(crate) use botbasic_data::keywords::use_account::{get_account_credentials, AccountCredentials};
pub(crate) use log::{info, trace};
pub(crate) use rhai::{Dynamic, Engine};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_apply_template_variables() {
        let template = "Hello {{name}}, your order {{order_id}} is ready!";
        let vars = json!({
            "name": "John",
            "order_id": "12345"
        });

        let result = apply_template_variables(template, &vars, "john@example.com").unwrap();
        assert!(result.contains("John"));
        assert!(result.contains("12345"));
    }

    #[test]
    fn test_extract_template_subject() {
        let content = "Subject: Welcome to our service\n\nHello there!";
        let subject = extract_template_subject(content);
        assert_eq!(subject, Some("Welcome to our service".to_string()));
    }
}

use botcore::shared::UserSession;
use botcore::shared::state::AppState;
use chrono::Utc;
use diesel::prelude::*;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;

pub use part1::*;
use part2::*;
