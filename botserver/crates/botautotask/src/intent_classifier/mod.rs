mod types;

pub use types::*;

use crate::types::{AutoTaskState, ConfigOps, DbPool, LlmProviderOps, TaskProgressEvent, UserSession};
use chrono::Utc;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::{Float8, Text, Uuid as DieselUuid};
use log::{info, trace, warn};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

pub struct IntentClassifier {
    pool: DbPool,
    config_ops: Arc<dyn ConfigOps>,
    llm_ops: Arc<dyn LlmProviderOps>,
    state: Arc<dyn AutoTaskState>,
}

#[cfg(test)]
mod tests {
    use super::types::*;

    #[test]
    fn test_intent_type_from_str() {
        assert_eq!(IntentType::from("APP_CREATE"), IntentType::AppCreate);
        assert_eq!(IntentType::from("app"), IntentType::AppCreate);
        assert_eq!(IntentType::from("TODO"), IntentType::Todo);
        assert_eq!(IntentType::from("unknown_value"), IntentType::Unknown);
    }

    #[test]
    fn test_intent_type_display() {
        assert_eq!(IntentType::AppCreate.to_string(), "APP_CREATE");
        assert_eq!(IntentType::Todo.to_string(), "TODO");
    }
}

mod part1;
mod part2;
mod part3;

use part3::*;
