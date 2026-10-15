use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
    Json,
};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use diesel::RunQueryDsl;
use std::sync::Arc;
use uuid::Uuid;

use botcore::organization_invitations::{
    BulkInviteParams, CreateInvitationParams, InvitationRole, InvitationService,
};
use botcore::shared::state::AppState;

use super::get_conn;

/// Renders the "Pending Invitations" card. Reads the real, DB-persisted
/// `organization_invitations` table (no demo rows) and emits resend/revoke
/// buttons whose actions are handled by the real `InvitationService`.
pub async fn dashboard_invitations(State(state): State<Arc<AppState>>) -> Html<String> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return Html(String::new()),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct InviteRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        email: String,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        role: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)]
        expires_at: Option<DateTime<Utc>>,
    }

    let invites: Vec<InviteRow> = diesel::sql_query(
        "SELECT id, email, role, created_at, expires_at FROM organization_invitations
         WHERE status = 'pending' ORDER BY created_at DESC LIMIT 10",
    )
    .load::<InviteRow>(&mut conn)
    .unwrap_or_default();

    if invites.is_empty() {
        return Html(
            r##"<div class="invitation-empty"><p>No pending invitations</p></div>"##.to_string(),
        );
    }

    let now = Utc::now();
    let mut items = String::new();
    for invite in invites {
        let expires = invite
            .expires_at
            .unwrap_or(invite.created_at + ChronoDuration::days(7));
        let expires_in = (expires - now).num_days().max(0);
        let exp_class = if expires_in <= 2 { " warning" } else { "" };
        let sent_label = format!("Sent {}", relative_time(invite.created_at));
        items.push_str(&format!(
            r##"<div class="invitation-item">
    <div class="invitation-info">
        <span class="invitation-email">{email}</span>
        <span class="invitation-role">{role}</span>
    </div>
    <div class="invitation-meta">
        <span class="invitation-sent">{sent}</span>
        <span class="invitation-expires{exp_class}">Expires in {expires_in} days</span>
    </div>
    <div class="invitation-actions">
        <button class="btn-icon" title="Resend invitation" hx-post="/api/admin/invitations/resend/{id}" hx-swap="none" hx-on::after-request="htmx.ajax('GET', '/api/admin/dashboard/invitations', '.invitation-list')">
            <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none">
                <polyline points="23 4 23 10 17 10"></polyline>
                <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
            </svg>
        </button>
        <button class="btn-icon danger" title="Revoke invitation" hx-delete="/api/admin/invitations/{id}" hx-swap="none" hx-confirm="Revoke this invitation?" hx-on::after-request="htmx.ajax('GET', '/api/admin/dashboard/invitations', '.invitation-list')">
            <svg viewBox="0 0 24 24" width="14" height="14" stroke="currentColor" stroke-width="2" fill="none">
                <line x1="18" y1="6" x2="6" y2="18"></line>
                <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
        </button>
    </div>
</div>"##,
            email = invite.email,
            role = invite.role,
            sent = sent_label,
            id = invite.id,
        ));
    }

    Html(items)
}

/// Resends the invitation email through the real `InvitationService`
/// (refreshes token/expiry and re-delivers via SMTP) instead of merely
/// bumping `updated_at`.
pub async fn resend_invitation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.resend_invitation(id, "Organization", true).await {
        Ok(invitation) => Ok(Json(serde_json::json!({
            "success": true,
            "id": invitation.id,
            "email": invitation.email,
        }))),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )),
    }
}

/// Revokes an invitation through the real `InvitationService`, which enforces
/// the pending-only state transition.
pub async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    match service.revoke_invitation(id).await {
        Ok(()) => Ok(Json(serde_json::json!({ "success": true }))),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )),
    }
}

/// Creates a single invitation (org-scoped) through the real service. The
/// inviter is resolved from the `x-user-id` header so the persisted
/// `invited_by` column points at a real user.
pub async fn create_invitation(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AdminCreateInvitationRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let role: InvitationRole = match req.role.parse() {
        Ok(role) => role,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid role" })),
            ))
        }
    };

    let invited_by = resolve_admin_user(&headers);

    match service
        .create_invitation(CreateInvitationParams {
            organization_id: req.org_id,
            organization_name: "Organization",
            email: &req.email,
            role,
            groups: req.groups,
            invited_by,
            invited_by_name: "Admin User",
            message: req.message,
            expires_in_days: 7,
        })
        .await
    {
        Ok(invitation) => Ok(Json(serde_json::json!({
            "success": true,
            "id": invitation.id,
            "email": invitation.email,
        }))),
        Err(error) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        )),
    }
}

/// Bulk-invites a list of emails through the real service.
pub async fn bulk_invite(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AdminBulkInviteRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let service = InvitationService::new(state.conn.clone());

    let role: InvitationRole = match req.role.parse() {
        Ok(role) => role,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "Invalid role" })),
            ))
        }
    };

    if req.emails.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "No emails provided" })),
        ));
    }

    if req.emails.len() > 100 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "Maximum 100 invitations per request" })),
        ));
    }

    let invited_by = resolve_admin_user(&headers);

    let response = service
        .bulk_invite(BulkInviteParams {
            organization_id: req.org_id,
            organization_name: "Organization",
            emails: req.emails,
            role,
            groups: req.groups,
            invited_by,
            invited_by_name: "Admin User",
            message: req.message,
        })
        .await;

    Ok(Json(serde_json::json!({
        "sent": response.successful.len(),
        "failed": response.failed.len(),
    })))
}

pub async fn admin_invitations(State(state): State<Arc<AppState>>) -> axum::Json<serde_json::Value> {
    let mut conn = match get_conn(&state) {
        Some(c) => c,
        None => return axum::Json(serde_json::json!([])),
    };

    #[derive(Debug, diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct InviteRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        email: String,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        role: String,
        #[diesel(sql_type = diesel::sql_types::Varchar)]
        status: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)]
        created_at: DateTime<Utc>,
    }

    let rows: Vec<InviteRow> = diesel::sql_query(
        "SELECT id, email, role, status, created_at FROM organization_invitations ORDER BY created_at DESC LIMIT 50",
    )
    .load::<InviteRow>(&mut conn)
    .unwrap_or_default();

    let items: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "email": i.email,
                "role": i.role,
                "status": i.status,
                "created_at": i.created_at,
            })
        })
        .collect();

    axum::Json(serde_json::json!(items))
}

#[derive(serde::Deserialize)]
pub struct AdminCreateInvitationRequest {
    org_id: Uuid,
    email: String,
    role: String,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct AdminBulkInviteRequest {
    org_id: Uuid,
    emails: Vec<String>,
    role: String,
    #[serde(default)]
    groups: Vec<String>,
    #[serde(default)]
    message: Option<String>,
}

fn resolve_admin_user(headers: &axum::http::HeaderMap) -> Uuid {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::nil)
}

fn relative_time(time: DateTime<Utc>) -> String {
    let diff = Utc::now() - time;
    if diff.num_days() > 0 {
        format!("{}d ago", diff.num_days())
    } else if diff.num_hours() > 0 {
        format!("{}h ago", diff.num_hours())
    } else if diff.num_minutes() > 0 {
        format!("{}m ago", diff.num_minutes())
    } else {
        "just now".to_string()
    }
}
