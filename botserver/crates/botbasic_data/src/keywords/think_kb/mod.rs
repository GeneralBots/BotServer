pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::QueryableByName;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use log::{info, warn};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use serde_json::json;
pub(crate) use std::sync::Arc;

#[derive(QueryableByName)]
struct GroupIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    group_id: uuid::Uuid,
}

#[derive(QueryableByName)]
struct KbIdRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: uuid::Uuid,
}

#[derive(QueryableByName, Debug, Clone)]
struct KbCollectionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    qdrant_collection: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    document_count: i32,
}

#[derive(QueryableByName, Debug, Clone)]
struct ConsolidationRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    insight: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    summary: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_calculation() {
        let confidence = calculate_confidence(0.8, 5, 3);
        assert!((0.0..=1.0).contains(&confidence));
    }

    #[test]
    fn test_json_to_dynamic_conversion() {
        let test_json = json!({
            "string_field": "test",
            "number_field": 42,
            "bool_field": true,
            "array_field": [1, 2, 3],
            "object_field": { "nested": "value" }
        });
        let dynamic_result = json_to_dynamic(test_json);
        assert!(!dynamic_result.is_unit());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
use part2::*;
use part3::*;
use part4::*;
