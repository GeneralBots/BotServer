use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookieInfo {
    pub name: String,
    pub provider: String,
    pub purpose: String,
    pub expiry: String,
    pub cookie_type: CookieType,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CookieType {
    Session,
    Persistent,
    FirstParty,
    ThirdParty,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegalDocument {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub version: String,
    pub effective_date: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub document_type: LegalDocumentType,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LegalDocumentType {
    PrivacyPolicy,
    TermsOfService,
    CookiePolicy,
    AcceptableUsePolicy,
    DataProcessingAgreement,
    GdprCompliance,
    CcpaCompliance,
}
impl std::fmt::Display for LegalDocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::PrivacyPolicy => "privacy_policy",
            Self::TermsOfService => "terms_of_service",
            Self::CookiePolicy => "cookie_policy",
            Self::AcceptableUsePolicy => "acceptable_use_policy",
            Self::DataProcessingAgreement => "data_processing_agreement",
            Self::GdprCompliance => "gdpr_compliance",
            Self::CcpaCompliance => "ccpa_compliance",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for LegalDocumentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "privacy_policy" => Ok(Self::PrivacyPolicy),
            "terms_of_service" => Ok(Self::TermsOfService),
            "cookie_policy" => Ok(Self::CookiePolicy),
            "acceptable_use_policy" => Ok(Self::AcceptableUsePolicy),
            "data_processing_agreement" => Ok(Self::DataProcessingAgreement),
            "gdpr_compliance" => Ok(Self::GdprCompliance),
            "ccpa_compliance" => Ok(Self::CcpaCompliance),
            _ => Err(format!("Unknown document type: {s}")),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct DataDeletionResult {
    pub user_id: Uuid,
    pub consents_deleted: i32,
    pub deleted_at: DateTime<Utc>,
    pub confirmation_token: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct UserDataExport {
    pub user_id: Uuid,
    pub exported_at: DateTime<Utc>,
    pub consents: Vec<CookieConsent>,
    pub format: String,
}
#[derive(Debug, Deserialize)]
pub struct ListDocumentsQuery {
    pub document_type: Option<String>,
    pub active_only: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct CreateDocumentRequest {
    pub slug: String,
    pub title: String,
    pub content: String,
    pub document_type: LegalDocumentType,
    pub version: Option<String>,
    pub requires_acceptance: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct UpdateDocumentRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub is_active: Option<bool>,
    pub requires_acceptance: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct DataDeletionRequest {
    pub reason: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct DataExportRequest {
    pub format: Option<String>,
    pub sections: Option<Vec<String>>,
}
#[derive(Debug, thiserror::Error)]
pub enum LegalError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Database error: {0}")]
    Database(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
impl IntoResponse for LegalError {
    fn into_response(self) -> axum::response::Response {
        use axum::http::StatusCode;
        let (status, message) = match &self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            Self::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            Self::Database(msg) | Self::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
pub(super) fn db_consent_to_consent(db: DbCookieConsent) -> CookieConsent {
    let mut consents = HashMap::new();
    consents.insert(CookieCategory::Necessary, db.consent_necessary);
    consents.insert(CookieCategory::Analytics, db.consent_analytics);
    consents.insert(CookieCategory::Marketing, db.consent_marketing);
    consents.insert(CookieCategory::Preferences, db.consent_preferences);
    consents.insert(CookieCategory::Functional, db.consent_functional);

    CookieConsent {
        id: db.id,
        user_id: db.user_id,
        session_id: db.session_id,
        ip_address: db.ip_address,
        consents,
        consent_given_at: db.consent_given_at,
        consent_updated_at: db.consent_updated_at,
        consent_version: db.consent_version,
        user_agent: db.user_agent,
        country_code: db.country_code,
    }
}
pub(super) fn db_document_to_document(db: DbLegalDocument) -> LegalDocument {
    let document_type: LegalDocumentType = db.document_type.parse().unwrap_or(LegalDocumentType::PrivacyPolicy);

    LegalDocument {
        id: db.id.to_string(),
        title: db.title,
        slug: db.slug,
        content: db.content,
        version: db.version,
        effective_date: db.effective_date,
        last_updated: db.updated_at,
        document_type,
    }
}
pub(super) fn get_default_cookie_policy() -> CookiePolicy {
    let now = Utc::now();
    CookiePolicy {
        version: "1.0.0".to_string(),
        effective_date: now,
        last_updated: now,
        categories: vec![
            CookieCategoryInfo {
                category: CookieCategory::Necessary,
                name: "Necessary".to_string(),
                description: "Essential cookies required for the website to function properly.".to_string(),
                is_required: true,
                cookies: vec![
                    CookieInfo {
                        name: "session_id".to_string(),
                        provider: "General Bots".to_string(),
                        purpose: "Session management".to_string(),
                        expiry: "Session".to_string(),
                        cookie_type: CookieType::Session,
                    },
                ],
            },
            CookieCategoryInfo {
                category: CookieCategory::Analytics,
                name: "Analytics".to_string(),
                description: "Cookies that help us understand how visitors interact with our website.".to_string(),
                is_required: false,
                cookies: vec![],
            },
            CookieCategoryInfo {
                category: CookieCategory::Marketing,
                name: "Marketing".to_string(),
                description: "Cookies used to deliver relevant advertisements.".to_string(),
                is_required: false,
                cookies: vec![],
            },
            CookieCategoryInfo {
                category: CookieCategory::Preferences,
                name: "Preferences".to_string(),
                description: "Cookies that remember your preferences and settings.".to_string(),
                is_required: false,
                cookies: vec![],
            },
            CookieCategoryInfo {
                category: CookieCategory::Functional,
                name: "Functional".to_string(),
                description: "Cookies that enable enhanced functionality.".to_string(),
                is_required: false,
                cookies: vec![],
            },
        ],
    }
}
pub async fn handle_record_consent(
    State(pool): State<Arc<DbPool>>,
    Json(req): Json<CookieConsentRequest>,
) -> Result<Json<CookieConsentResponse>, LegalError> {
    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| LegalError::Database(e.to_string()))?;
        let branch_id = get_branch_id(&mut conn);
        let now = Utc::now();

        let mut consents = req.consents;
        consents.insert(CookieCategory::Necessary, true);

        let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
        let db_consent = DbCookieConsent {
            id: Uuid::new_v4(),
            org_id,
            bot_id,
            user_id: None,
            session_id: req.session_id,
            ip_address: None,
            user_agent: req.user_agent,
            country_code: None,
            consent_necessary: true,
            consent_analytics: *consents.get(&CookieCategory::Analytics).unwrap_or(&false),
            consent_marketing: *consents.get(&CookieCategory::Marketing).unwrap_or(&false),
            consent_preferences: *consents.get(&CookieCategory::Preferences).unwrap_or(&false),
            consent_functional: *consents.get(&CookieCategory::Functional).unwrap_or(&false),
            consent_version: "1.0.0".to_string(),
            consent_given_at: now,
            consent_updated_at: now,
            consent_withdrawn_at: None,
            created_at: now,
            branch_id,
        };

        diesel::insert_into(cookie_consents::table)
            .values(&db_consent)
            .execute(&mut conn)
            .map_err(|e| LegalError::Database(e.to_string()))?;

        Ok::<_, LegalError>(CookieConsentResponse {
            id: db_consent.id,
            consents,
            consent_given_at: db_consent.consent_given_at,
            consent_version: db_consent.consent_version,
        })
    })
    .await
    .map_err(|e| LegalError::Internal(e.to_string()))??;

    Ok(Json(result))
}
