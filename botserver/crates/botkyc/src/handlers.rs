use axum::extract::{Json, Path};
use axum::http::{HeaderMap, StatusCode};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use diesel::RunQueryDsl;

use crate::db;
use crate::storage::ensure_schema_sync;

use botcore::shared::tenant::branch_from_claims;

fn resolve_branch(headers: &HeaderMap) -> Uuid {
    branch_from_claims(headers).unwrap_or_else(Uuid::nil)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub kind: String,
    pub status: String,
    pub documents: Vec<String>,
    pub reviewed_by: Option<Uuid>,
    pub created_at: chrono::DateTime<Utc>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub id: Uuid,
    pub document_id: Uuid,
    pub signer_name: String,
    pub signer_email: String,
    pub status: String,
    pub signed_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Certificate {
    pub id: Uuid,
    pub subject: String,
    pub issuer: String,
    pub serial: String,
    pub valid_from: chrono::DateTime<Utc>,
    pub valid_until: chrono::DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVerification {
    pub status: String,
    pub reviewed_by: Option<Uuid>,
    pub documents: Option<Vec<String>>,
}

pub async fn list_verifications(headers: HeaderMap) -> Result<Json<Vec<Verification>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] profile_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] workflow_name: String,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Jsonb)] steps_completed: serde_json::Value,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] started_at: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] completed_at: Option<chrono::DateTime<Utc>>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, profile_id, workflow_name, status, steps_completed, started_at, completed_at
         FROM identity_kyc_workflows WHERE branch_id = $1 ORDER BY started_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| Verification {
        id: r.id,
        user_id: r.profile_id,
        kind: r.workflow_name,
        status: r.status,
        documents: r
            .steps_completed
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        reviewed_by: None,
        created_at: r.started_at,
        completed_at: r.completed_at,
    }).collect()))
}

pub async fn update_verification(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<UpdateVerification>,
) -> Result<Json<Verification>, (StatusCode, String)> {
    let parsed = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid id '{id}': {e}")))?;
    // The biometric KYC state machine owns the lifecycle (#1502): unknown
    // statuses are rejected here and terminal cases cannot be reopened.
    let state = botbiometry::kyc::KycState::from_status(&req.status).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            format!("Unknown KYC status '{}'", req.status),
        )
    })?;
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct CurrentRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        status: String,
    }
    let current: Vec<CurrentRow> = diesel::sql_query(
        "SELECT status FROM identity_kyc_workflows WHERE id = $1 AND branch_id = $2",
    )
    .bind::<diesel::sql_types::Uuid, _>(parsed)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    let current = match current.into_iter().next() {
        Some(row) => row.status,
        None => return Err((StatusCode::NOT_FOUND, format!("Verification {id} not found"))),
    };
    if let Some(current_state) = botbiometry::kyc::KycState::from_status(&current) {
        if current_state.is_terminal() && current_state != state {
            return Err((
                StatusCode::CONFLICT,
                format!(
                    "KYC case {id} is terminal ({}) and cannot transition to '{}'",
                    current_state.as_status(),
                    state.as_status()
                ),
            ));
        }
    }
    let completed_at = if state.is_terminal() {
        Some(Utc::now())
    } else {
        None
    };
    let n = diesel::sql_query(
        "UPDATE identity_kyc_workflows SET status = $1, completed_at = COALESCE($2, completed_at) WHERE id = $3 AND branch_id = $4",
    )
    .bind::<diesel::sql_types::Text, _>(state.as_status())
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(completed_at)
    .bind::<diesel::sql_types::Uuid, _>(parsed)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    if n == 0 { return Err((StatusCode::NOT_FOUND, format!("Verification {id} not found"))); }
    // Structured biometric audit trail for the state change.
    let event = botbiometry::audit::BiometricAuditEvent {
        id: Uuid::new_v4(),
        tenant_id: branch.to_string(),
        actor_id: req
            .reviewed_by
            .map(|reviewer| reviewer.to_string())
            .unwrap_or_else(|| "system".to_string()),
        action: botbiometry::audit::AuditAction::KycStateChanged,
        payload_json: format!(
            "{{\"verification_id\":\"{parsed}\",\"from\":\"{current}\",\"to\":\"{}\"}}",
            state.as_status()
        ),
        at: Utc::now(),
    };
    log::info!(
        "biometric audit: action={:?} id={} tenant={} actor={} payload={}",
        event.action,
        event.id,
        event.tenant_id,
        event.actor_id,
        event.payload_json
    );
    Ok(Json(Verification {
        id: parsed, user_id: Uuid::nil(), kind: "identity".to_string(), status: state.as_status().to_string(),
        documents: req.documents.unwrap_or_default(), reviewed_by: req.reviewed_by,
        created_at: Utc::now(), completed_at,
    }))
}

pub async fn list_signatures(headers: HeaderMap) -> Result<Json<Vec<Signature>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Uuid)] document_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] signature_data: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] signed_at: chrono::DateTime<Utc>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, document_id, signature_data, signed_at
         FROM identity_signatures WHERE branch_id = $1 ORDER BY signed_at DESC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| Signature {
        id: r.id,
        document_id: r.document_id,
        signer_name: r.signature_data,
        signer_email: String::new(),
        status: "signed".to_string(),
        signed_at: Some(r.signed_at),
        created_at: r.signed_at,
    }).collect()))
}

pub async fn sign_document(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<Signature>, (StatusCode, String)> {
    let parsed = Uuid::parse_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid id '{id}': {e}")))?;
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let now = Utc::now();
    let n = diesel::sql_query(
        "UPDATE identity_signatures SET signed_at = $1 WHERE id = $2 AND branch_id = $3",
    )
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Uuid, _>(parsed)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    if n == 0 {
        return Err((StatusCode::NOT_FOUND, format!("Signature {id} not found")));
    }
    Ok(Json(Signature {
        id: parsed, document_id: Uuid::nil(), signer_name: String::new(), signer_email: String::new(),
        status: "signed".to_string(), signed_at: Some(now), created_at: now,
    }))
}

pub async fn list_certificates(headers: HeaderMap) -> Result<Json<Vec<Certificate>>, (StatusCode, String)> {
    ensure_schema_sync()?;
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    // The identity_certificates table is not created by any migration yet,
    // so guard with to_regclass and return an empty list instead of 500ing.
    #[derive(diesel::QueryableByName)]
    struct RegRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
        exists: Option<String>,
    }
    let rows: Vec<RegRow> = diesel::sql_query(
        "SELECT to_regclass('public.identity_certificates')::text AS exists",
    )
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    if rows.is_empty() || rows[0].exists.is_none() {
        return Ok(Json(Vec::<Certificate>::new()));
    }
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Text)] subject: String,
        #[diesel(sql_type = diesel::sql_types::Text)] issuer: String,
        #[diesel(sql_type = diesel::sql_types::Text)] serial: String,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] valid_from: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] valid_until: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT id, subject, issuer, serial, valid_from, valid_until, status
         FROM identity_certificates WHERE branch_id = $1 ORDER BY subject ASC LIMIT 500",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;
    Ok(Json(rows.into_iter().map(|r| Certificate {
        id: r.id, subject: r.subject, issuer: r.issuer, serial: r.serial,
        valid_from: r.valid_from, valid_until: r.valid_until, status: r.status,
    }).collect()))
}
