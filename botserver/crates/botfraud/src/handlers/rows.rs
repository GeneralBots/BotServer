//! `QueryableByName` row structs backing the raw-SQL queries.

use uuid::Uuid;

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CountRow {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub count: i64,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct RuleRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub description: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub rule_type: String,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub condition_json: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub action: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub severity: String,
    #[diesel(sql_type = diesel::sql_types::Bool)]
    pub is_active: bool,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EventRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub event_type: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub entity_type: String,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub entity_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub risk_score: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub risk_level: String,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub triggered_rules: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Numeric>)]
    pub ml_score: Option<rust_decimal::Decimal>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub action_taken: String,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub details: serde_json::Value,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(diesel::QueryableByName)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BlocklistRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub branch_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub block_type: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub block_value: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub reason: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}
