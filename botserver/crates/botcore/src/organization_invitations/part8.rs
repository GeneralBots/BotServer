use super::*;

pub(super) async fn validate_invitation(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> Result<Json<InvitationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.get_invitation_by_token(&token).await {
        Some(invitation) => {
            if invitation.status != InvitationStatus::Pending {
                return Err((
                    StatusCode::GONE,
                    Json(serde_json::json!({
                        "error": "Invitation is no longer valid",
                        "status": format!("{:?}", invitation.status).to_lowercase()
                    })),
                ));
            }

            if invitation.expires_at < Utc::now() {
                return Err((
                    StatusCode::GONE,
                    Json(serde_json::json!({
                        "error": "Invitation has expired",
                        "expired_at": invitation.expires_at
                    })),
                ));
            }

            Ok(Json(service.to_response(&invitation, "Organization")))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Invalid invitation token"})),
        )),
    }
}
