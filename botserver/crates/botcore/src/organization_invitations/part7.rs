use super::*;

pub fn configure() -> Router<Arc<AppState>> {
    Router::new()
        .route("/organizations/:org_id/invitations", get(list_invitations))
        .route(
            "/organizations/:org_id/invitations",
            post(create_invitation),
        )
        .route("/organizations/:org_id/invitations/bulk", post(bulk_invite))
        .route(
            "/organizations/:org_id/invitations/:invitation_id",
            get(get_invitation),
        )
        .route(
            "/organizations/:org_id/invitations/:invitation_id",
            delete(revoke_invitation),
        )
        .route(
            "/organizations/:org_id/invitations/:invitation_id/resend",
            post(resend_invitation),
        )
        .route("/invitations/accept", post(accept_invitation))
        .route("/invitations/decline", post(decline_invitation))
        .route("/invitations/validate/:token", get(validate_invitation))
}
pub(super) async fn list_invitations(
    State(state): State<Arc<AppState>>,
    Path(org_id): Path<Uuid>,
    Query(params): Query<ListInvitationsQuery>,
) -> Result<Json<InvitationListResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let status_filter = params.status.and_then(|s| match s.to_lowercase().as_str() {
        "pending" => Some(InvitationStatus::Pending),
        "accepted" => Some(InvitationStatus::Accepted),
        "declined" => Some(InvitationStatus::Declined),
        "expired" => Some(InvitationStatus::Expired),
        "revoked" => Some(InvitationStatus::Revoked),
        _ => None,
    });

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).min(100);

    let response = service
        .list_invitations(org_id, status_filter, page, per_page)
        .await;

    Ok(Json(response))
}
pub(super) async fn create_invitation(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let role: InvitationRole = req.role.parse().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid role"})),
        )
    })?;

    let expires_in_days = req.expires_in_days.unwrap_or(7).clamp(1, 30);

    // The inviter must be a real user (FK users(id)); resolve from headers
    // like the accept path instead of a random UUID.
    let invited_by = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            botsecurity_core::tenant::user_id_from_claims(&headers).and_then(|sub| {
                match Uuid::parse_str(&sub) {
                    Ok(u) => Some(u),
                    Err(_) => Some(Uuid::new_v5(
                        &Uuid::NAMESPACE_DNS,
                        format!("zitadel:{sub}").as_bytes(),
                    )),
                }
            })
        })
        .unwrap_or(Uuid::nil());

    match service
        .create_invitation(CreateInvitationParams {
            organization_id: org_id,
            organization_name: "Organization",
            email: &req.email,
            role,
            groups: req.groups,
            invited_by,
            invited_by_name: "Admin User",
            message: req.message,
            expires_in_days,
        })
        .await
    {
        Ok(invitation) => Ok(Json(service.to_response(&invitation, "Organization"))),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error})),
        )),
    }
}
pub(super) async fn bulk_invite(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(org_id): Path<Uuid>,
    Json(req): Json<BulkInviteRequest>,
) -> Result<Json<BulkInviteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let role = req.role.parse::<InvitationRole>().map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid role"})),
        )
    })?;

    if req.emails.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No emails provided"})),
        ));
    }

    if req.emails.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Maximum 100 invitations per request"})),
        ));
    }

    let invited_by = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            botsecurity_core::tenant::user_id_from_claims(&headers).and_then(|sub| {
                match Uuid::parse_str(&sub) {
                    Ok(u) => Some(u),
                    Err(_) => Some(Uuid::new_v5(
                        &Uuid::NAMESPACE_DNS,
                        format!("zitadel:{sub}").as_bytes(),
                    )),
                }
            })
        })
        .unwrap_or(Uuid::nil());

    let response = service
        .bulk_invite(BulkInviteParams {
            organization_id: org_id,
            organization_name: "Organization",
            emails: req.emails,
            role,
            groups: req.groups,
            invited_by,
            invited_by_name: "Admin User",
            message: req.message,
        })
        .await;

    Ok(Json(response))
}
pub(super) async fn get_invitation(
    State(state): State<Arc<AppState>>,
    Path((org_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.get_invitation(invitation_id).await {
        Some(invitation) if invitation.organization_id == org_id => {
            Ok(Json(service.to_response(&invitation, "Organization")))
        }
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Invitation belongs to different organization"})),
        )),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Invitation not found"})),
        )),
    }
}
pub(super) async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    Path((_org_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.revoke_invitation(invitation_id).await {
        Ok(()) => Ok(Json(
            serde_json::json!({"success": true, "message": "Invitation revoked"}),
        )),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error})),
        )),
    }
}
pub(super) async fn resend_invitation(
    State(state): State<Arc<AppState>>,
    Path((_org_id, invitation_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ResendInvitationRequest>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let extend_expiry = req.extend_expiry.unwrap_or(true);

    match service
        .resend_invitation(invitation_id, "Organization", extend_expiry)
        .await
    {
        Ok(invitation) => Ok(Json(service.to_response(&invitation, "Organization"))),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error})),
        )),
    }
}
pub(super) async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AcceptInvitationRequest>,
) -> Result<Json<AcceptInvitationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    // Resolve the accepting user from the request: X-User-ID header first
    // (loopback/chat executor), then the JWT sub claim (derived to the stable
    // UUID the RBAC layer uses). Never a random UUID — the org binding must
    // point at the real user or the accept is meaningless.
    let user_id = headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .or_else(|| {
            botsecurity_core::tenant::user_id_from_claims(&headers).and_then(|sub| {
                match Uuid::parse_str(&sub) {
                    Ok(u) => Some(u),
                    Err(_) => Some(Uuid::new_v5(
                        &Uuid::NAMESPACE_DNS,
                        format!("zitadel:{sub}").as_bytes(),
                    )),
                }
            })
        })
        .or(req.user_id)
        .unwrap_or(Uuid::nil());

    if user_id == Uuid::nil() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Authentication required to accept an invitation"})),
        ));
    }

    match service.accept_invitation(&req.token, user_id).await {
        Ok(response) => Ok(Json(response)),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error})),
        )),
    }
}
pub(super) async fn decline_invitation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AcceptInvitationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.decline_invitation(&req.token).await {
        Ok(()) => Ok(Json(
            serde_json::json!({"success": true, "message": "Invitation declined"}),
        )),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": error})),
        )),
    }
}
