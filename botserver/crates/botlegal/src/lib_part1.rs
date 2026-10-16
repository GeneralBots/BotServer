use super::*;

pub use botlib::db_pool::DbPool;

pub use botschema::branches;

pub use botschema::legal_documents;

pub use botschema::legal_document_versions;

pub use botschema::cookie_consents;

pub use botschema::consent_history;

pub use botschema::legal_acceptances;

pub use botschema::data_deletion_requests;

pub use botschema::data_export_requests;

#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = legal_documents)]
pub struct DbLegalDocument {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub slug: String,
    pub title: String,
    pub content: String,
    pub document_type: String,
    pub version: String,
    pub effective_date: DateTime<Utc>,
    pub is_active: bool,
    pub requires_acceptance: bool,
    pub metadata: serde_json::Value,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = legal_document_versions)]
pub struct DbDocumentVersion {
    pub id: Uuid,
    pub document_id: Uuid,
    pub version: String,
    pub content: String,
    pub change_summary: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = cookie_consents)]
pub struct DbCookieConsent {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub country_code: Option<String>,
    pub consent_necessary: bool,
    pub consent_analytics: bool,
    pub consent_marketing: bool,
    pub consent_preferences: bool,
    pub consent_functional: bool,
    pub consent_version: String,
    pub consent_given_at: DateTime<Utc>,
    pub consent_updated_at: DateTime<Utc>,
    pub consent_withdrawn_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = consent_history)]
pub struct DbConsentHistory {
    pub id: Uuid,
    pub consent_id: Uuid,
    pub action: String,
    pub previous_consents: serde_json::Value,
    pub new_consents: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = legal_acceptances)]
pub struct DbLegalAcceptance {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Uuid,
    pub document_id: Uuid,
    pub document_version: String,
    pub accepted_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = data_deletion_requests)]
pub struct DbDeletionRequest {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Uuid,
    pub request_type: String,
    pub status: String,
    pub reason: Option<String>,
    pub requested_at: DateTime<Utc>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub confirmation_token: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub processed_by: Option<Uuid>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Queryable, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = data_export_requests)]
pub struct DbExportRequest {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub format: String,
    pub include_sections: serde_json::Value,
    pub requested_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub file_url: Option<String>,
    pub file_size: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CookieCategory {
    Necessary,
    Analytics,
    Marketing,
    Preferences,
    Functional,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieConsent {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub consents: HashMap<CookieCategory, bool>,
    pub consent_given_at: DateTime<Utc>,
    pub consent_updated_at: DateTime<Utc>,
    pub consent_version: String,
    pub user_agent: Option<String>,
    pub country_code: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct CookieConsentRequest {
    pub session_id: Option<String>,
    pub consents: HashMap<CookieCategory, bool>,
    pub user_agent: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct CookieConsentResponse {
    pub id: Uuid,
    pub consents: HashMap<CookieCategory, bool>,
    pub consent_given_at: DateTime<Utc>,
    pub consent_version: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiePolicy {
    pub version: String,
    pub effective_date: DateTime<Utc>,
    pub categories: Vec<CookieCategoryInfo>,
    pub last_updated: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieCategoryInfo {
    pub category: CookieCategory,
    pub name: String,
    pub description: String,
    pub is_required: bool,
    pub cookies: Vec<CookieInfo>,
}
