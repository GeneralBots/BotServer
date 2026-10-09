pub(crate) use botbasic_types::schema::bot_memories;
pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{debug, info, trace};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_lead_score_empty() {
        let lead_data = Map::new();
        let score = calculate_lead_score(&lead_data, None);
        assert_eq!(score, 0);
    }

    #[test]
    fn test_calculate_lead_score_basic() {
        let mut lead_data = Map::new();
        lead_data.insert("job_title".into(), Dynamic::from("CEO"));
        lead_data.insert("company_size".into(), Dynamic::from(500_i64));
        lead_data.insert("email".into(), Dynamic::from("ceo@company.com"));

        let score = calculate_lead_score(&lead_data, None);
        assert!(score > 30);
    }

    #[test]
    fn test_calculate_lead_score_with_title() {
        let mut lead_data = Map::new();
        lead_data.insert("job_title".into(), Dynamic::from("CTO"));

        let score = calculate_lead_score(&lead_data, None);
        assert!(score >= 30);
    }

    #[test]
    fn test_determine_priority() {
        assert_eq!(determine_priority(95), "CRITICAL");
        assert_eq!(determine_priority(75), "HIGH");
        assert_eq!(determine_priority(55), "MEDIUM");
        assert_eq!(determine_priority(35), "LOW");
        assert_eq!(determine_priority(10), "MINIMAL");
    }

    #[test]
    fn test_score_clamping() {
        let mut lead_data = Map::new();
        lead_data.insert("budget".into(), Dynamic::from(1_000_000_i64));
        lead_data.insert("job_title".into(), Dynamic::from("CEO"));
        lead_data.insert("company_size".into(), Dynamic::from(5000_i64));
        lead_data.insert("email".into(), Dynamic::from("ceo@enterprise.com"));
        lead_data.insert("industry".into(), Dynamic::from("technology"));

        let score = calculate_lead_score(&lead_data, None);
        assert!(
            score <= 100,
            "Score should be clamped to 100, got {}",
            score
        );
    }

    #[test]
    fn test_get_recommendation() {
        let rec_critical = get_recommendation(95);
        assert!(rec_critical.contains("immediately"));

        let rec_high = get_recommendation(75);
        assert!(rec_high.contains("48 hours"));

        let rec_medium = get_recommendation(55);
        assert!(rec_medium.contains("Nurture"));
    }

    #[test]
    fn test_get_suggested_action() {
        let action_critical = get_suggested_action(95);
        assert!(action_critical.contains("demo"));

        let action_high = get_suggested_action(75);
        assert!(action_high.contains("email"));
    }

    #[test]
    fn test_calculate_lead_score_with_custom_rules() {
        let mut lead_data = Map::new();
        lead_data.insert("job_title".into(), Dynamic::from("Manager"));

        let mut custom_rules = Map::new();
        custom_rules.insert("bonus".into(), Dynamic::from(10_i64));

        let score_without = calculate_lead_score(&lead_data, None);
        let score_with = calculate_lead_score(&lead_data, Some(&custom_rules));

        assert_eq!(score_with, score_without + 10);
    }
}

mod part1;
mod part2;

pub use part1::*;
pub use part2::*;
