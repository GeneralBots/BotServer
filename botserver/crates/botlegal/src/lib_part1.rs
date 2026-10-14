use super::*;

pub use botlib::db_pool::DbPool;

diesel::table! {
    branches (id) {
        id -> Uuid,
    }
}

diesel::table! {
    legal_documents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        slug -> Varchar,
        title -> Varchar,
        content -> Text,
        document_type -> Varchar,
        version -> Varchar,
        effective_date -> Timestamptz,
        is_active -> Bool,
        requires_acceptance -> Bool,
        metadata -> Jsonb,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    legal_document_versions (id) {
        id -> Uuid,
        document_id -> Uuid,
        version -> Varchar,
        content -> Text,
        change_summary -> Nullable<Text>,
        created_by -> Nullable<Uuid>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    cookie_consents (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Nullable<Uuid>,
        session_id -> Nullable<Varchar>,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        country_code -> Nullable<Varchar>,
        consent_necessary -> Bool,
        consent_analytics -> Bool,
        consent_marketing -> Bool,
        consent_preferences -> Bool,
        consent_functional -> Bool,
        consent_version -> Varchar,
        consent_given_at -> Timestamptz,
        consent_updated_at -> Timestamptz,
        consent_withdrawn_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    consent_history (id) {
        id -> Uuid,
        consent_id -> Uuid,
        action -> Varchar,
        previous_consents -> Jsonb,
        new_consents -> Jsonb,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    legal_acceptances (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        document_id -> Uuid,
        document_version -> Varchar,
        accepted_at -> Timestamptz,
        ip_address -> Nullable<Varchar>,
        user_agent -> Nullable<Text>,
    }
}

diesel::table! {
    data_deletion_requests (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        request_type -> Varchar,
        status -> Varchar,
        reason -> Nullable<Text>,
        requested_at -> Timestamptz,
        scheduled_for -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        confirmation_token -> Varchar,
        confirmed_at -> Nullable<Timestamptz>,
        processed_by -> Nullable<Uuid>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    data_export_requests (id) {
        id -> Uuid,
        branch_id -> Uuid,
        user_id -> Uuid,
        status -> Varchar,
        format -> Varchar,
        include_sections -> Jsonb,
        requested_at -> Timestamptz,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        file_url -> Nullable<Text>,
        file_size -> Nullable<Int4>,
        expires_at -> Nullable<Timestamptz>,
        error_message -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::joinable!(legal_documents -> branches (branch_id));
diesel::joinable!(legal_document_versions -> legal_documents (document_id));
diesel::joinable!(cookie_consents -> branches (branch_id));
diesel::joinable!(consent_history -> cookie_consents (consent_id));
diesel::joinable!(legal_acceptances -> branches (branch_id));
diesel::joinable!(legal_acceptances -> legal_documents (document_id));
diesel::joinable!(data_deletion_requests -> branches (branch_id));
diesel::joinable!(data_export_requests -> branches (branch_id));
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
