use super::*;

impl BadRequestResponse {
    pub(super) fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl IntoResponse for BadRequestResponse {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "bad_request",
            "message": self.message,
            "code": "BAD_REQUEST"
        });

        (StatusCode::BAD_REQUEST, Json(body)).into_response()
    }
}

// ============================================================================
// Axum Extractors
// ============================================================================
/// Axum extractor for RequestContext
#[axum::async_trait]
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts.extensions.get::<RequestContext>().cloned().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "Request context not available"
            })),
        ))
    }
}
/// Axum extractor for AuthenticatedUser
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Authentication required"
                })),
            ))
    }
}
/// Axum extractor for OrganizationContext
#[axum::async_trait]
impl<S> FromRequestParts<S> for OrganizationContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<OrganizationContext>()
            .cloned()
            .ok_or((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "Organization context required"
                })),
            ))
    }
}

// ============================================================================
// Utility Functions
// ============================================================================
/// Create middleware state with database pool
pub fn create_context_middleware_state(
    db_pool: DbPool,
    jwt_secret: String,
) -> Arc<ContextMiddlewareState> {
    Arc::new(ContextMiddlewareState::new(db_pool, jwt_secret))
}
/// Check if user can access a specific resource
pub fn can_access_resource(
    context: &RequestContext,
    resource_org_id: Option<Uuid>,
    required_permission: Option<&str>,
) -> bool {
    // Super admin can access everything
    if context.user.is_super_admin {
        return true;
    }

    // Check organization match
    if let Some(res_org_id) = resource_org_id {
        if !context.can_access_organization(res_org_id) {
            return false;
        }
    }

    // Check permission if required
    if let Some(permission) = required_permission {
        if !context.has_org_permission(permission) {
            return false;
        }
    }

    true
}
/// Build permission filter for Qdrant searches based on user context
#[cfg(any(feature = "research", feature = "llm"))]
pub fn build_search_permission_filter(context: &RequestContext) -> serde_json::Value {
    context.user.get_qdrant_filter()
}
pub async fn validate_org_membership(
    _db_pool: &DbPool,
    _user_id: Uuid,
    _org_id: Uuid,
) -> Result<bool, String> {
    Ok(true)
}
pub async fn get_user_org_role(
    _db_pool: &DbPool,
    _user_id: Uuid,
    _org_id: Uuid,
) -> Result<Option<String>, String> {
    Ok(Some("member".to_string()))
}
