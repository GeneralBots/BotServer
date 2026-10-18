pub(crate) use axum::{ extract::{Path, Query, State}, routing::{get, post, put}, Json, Router, };
pub(crate) use bigdecimal::{BigDecimal, ToPrimitive};
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use log::info;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::goals_types::*;
pub(crate) use crate::schema::{okr_checkins, okr_key_results, okr_objectives, okr_templates};
pub(crate) use crate::{AuthenticatedUser, DbPool, GetBotContextFn};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_objective_record_creation() {
        let now = Utc::now();
        let objective = ObjectiveRecord {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            branch_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            parent_id: None,
            title: "Test Objective".to_string(),
            description: Some("Test description".to_string()),
            period: "Q1 2025".to_string(),
            period_start: None,
            period_end: None,
            status: "draft".to_string(),
            progress: BigDecimal::from(0),
            visibility: "team".to_string(),
            weight: BigDecimal::from(1),
            tags: vec![Some("test".to_string())],
            created_at: now,
            updated_at: now,
            bot_id: None,
        };

        assert_eq!(objective.title, "Test Objective");
        assert_eq!(objective.status, "draft");
        assert_eq!(objective.progress, BigDecimal::from(0));
    }

    #[test]
    fn test_key_result_record_creation() {
        let now = Utc::now();
        let key_result = KeyResultRecord {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            branch_id: Uuid::new_v4(),
            objective_id: Uuid::new_v4(),
            owner_id: Uuid::new_v4(),
            title: "Test Key Result".to_string(),
            description: Some("Test KR description".to_string()),
            metric_type: "numeric".to_string(),
            start_value: BigDecimal::from(0),
            target_value: BigDecimal::from(100),
            current_value: BigDecimal::from(0),
            unit: Some("units".to_string()),
            weight: BigDecimal::from(1),
            status: "not_started".to_string(),
            due_date: None,
            scoring_type: "linear".to_string(),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(key_result.title, "Test Key Result");
        assert_eq!(key_result.metric_type, "numeric");
        assert_eq!(key_result.target_value, BigDecimal::from(100));
        assert_eq!(key_result.status, "not_started");
    }

    #[test]
    fn test_check_in_record_creation() {
        let now = Utc::now();
        let check_in = CheckInRecord {
            id: Uuid::new_v4(),
            org_id: Uuid::new_v4(),
            branch_id: Uuid::new_v4(),
            key_result_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            previous_value: Some(BigDecimal::from(0)),
            new_value: BigDecimal::from(50),
            note: Some("Progress update".to_string()),
            confidence: Some("high".to_string()),
            blockers: Some("No blockers".to_string()),
            created_at: now,
        };

        assert_eq!(check_in.new_value, BigDecimal::from(50));
        assert_eq!(check_in.confidence, Some("high".to_string()));
    }

    #[test]
    fn test_goals_error_display() {
        let db_error = GoalsError::Database("Connection failed".to_string());
        assert!(format!("{}", db_error).contains("Database error"));

        let not_found = GoalsError::NotFound("Objective not found".to_string());
        assert!(format!("{}", not_found).contains("not found"));

        let validation = GoalsError::Validation("Invalid input".to_string());
        assert!(format!("{}", validation).contains("Validation error"));
    }
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
pub use part3::*;
