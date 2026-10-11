use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Serialize)]
#[diesel(table_name = crate::schema::connector_connections)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ConnectionRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub org_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub kind: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub display_name: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub vault_token_ref: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub status: String,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub cursors: Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub last_sync_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub created_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, QueryableByName, Serialize)]
#[diesel(table_name = crate::schema::indexed_items)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ItemRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub connection_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub external_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub title: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub body_tsv: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub vector_ref: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Jsonb)]
    pub acl: Value,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub container: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub external_url: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Timestamptz)]
    pub updated_at: DateTime<Utc>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectBody {
    pub kind: String,
    pub display_name: Option<String>,
    pub credentials: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default = "default_search_limit")]
    pub limit: i64,
}

fn default_search_limit() -> i64 {
    25
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self { q: String::new(), sources: Vec::new(), limit: default_search_limit() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_roundtrip_with_defaults() {
        let original = SearchQuery { q: "quarterly report".to_string(), sources: vec!["drive".to_string()], limit: 10 };
        let json = serde_json::to_string(&original).unwrap();
        assert!(json.contains("quarterly report"));
        let parsed: SearchQuery = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.q, "quarterly report");
        assert_eq!(parsed.sources, vec!["drive".to_string()]);
        assert_eq!(parsed.limit, 10);
    }

    #[test]
    fn search_query_sources_and_limit_optional() {
        let parsed: SearchQuery =
            serde_json::from_str(r#"{"q": "invoice"}"#).unwrap();
        assert!(parsed.sources.is_empty());
        assert_eq!(parsed.limit, 25);
    }

    #[test]
    fn connect_body_roundtrip() {
        let body = ConnectBody {
            kind: "chat".to_string(),
            display_name: Some("Corp Chat".to_string()),
            credentials: serde_json::json!({ "base_url": "https://chat.example.com", "token": "t" }),
        };
        let json = serde_json::to_string(&body).unwrap();
        let parsed: ConnectBody = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.kind, "chat");
        assert_eq!(parsed.display_name.as_deref(), Some("Corp Chat"));
        assert_eq!(parsed.credentials["base_url"], "https://chat.example.com");
    }

    #[test]
    fn item_row_serializes_acl_as_json() {
        let row = ItemRow {
            id: Uuid::nil(),
            connection_id: Uuid::nil(),
            external_id: "e1".to_string(),
            title: "Title".to_string(),
            body_tsv: None,
            vector_ref: None,
            acl: serde_json::json!(["user:00000000-0000-0000-0000-000000000001"]),
            container: Some("general".to_string()),
            external_url: None,
            updated_at: Utc::now(),
            deleted_at: None,
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(row.acl, json["acl"]);
        assert_eq!(serde_json::Value::String("e1".to_string()), json["external_id"]);
    }
}
