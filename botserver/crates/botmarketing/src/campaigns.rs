use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::{marketing_campaigns, marketing_contacts, marketing_recipients, marketing_templates};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = marketing_campaigns)]
pub struct CrmCampaign {
    pub id: Uuid,
    pub org_id: Option<Uuid>,
    pub bot_id: Option<Uuid>,
    pub name: String,
    pub status: Option<String>,
    pub campaign_type: String,
    pub content_template: Option<serde_json::Value>,
    pub starts_at: Option<DateTime<Utc>>,
    pub sent_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub metrics: Option<serde_json::Value>,
    pub budget: Option<bigdecimal::BigDecimal>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deal_id: Option<Uuid>,
    pub branch_id: Uuid,
    pub run_offset: Option<i32>,
    pub pause_requested: Option<bool>,
    pub stop_requested: Option<bool>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub name: String,
    pub campaign_type: Option<String>,
    pub scheduled_at: Option<String>,
    pub budget: Option<f64>,
    pub metrics: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub campaign_type: Option<String>,
    pub scheduled_at: Option<String>,
    pub budget: Option<f64>,
    pub metrics: Option<serde_json::Value>,
}

pub async fn list_campaigns(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<CrmCampaign>>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (_, default_branch_id, _) = state.get_scope();
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or(default_branch_id);

    let campaigns: Vec<CrmCampaign> = marketing_campaigns::table
        .filter(marketing_campaigns::branch_id.eq(branch_id))
        .order(marketing_campaigns::created_at.desc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(campaigns))
}

pub async fn get_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<CrmCampaign>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let campaign: CrmCampaign = marketing_campaigns::table
        .filter(marketing_campaigns::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Campaign not found".to_string()))?;

    Ok(Json(campaign))
}

pub async fn create_campaign(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<CreateCampaignRequest>,
) -> Result<Json<CrmCampaign>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let (_, default_branch_id, _) = state.get_scope();
    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or(default_branch_id);
    let id = Uuid::new_v4();
    let now = Utc::now();

    let starts_at = req.scheduled_at.and_then(|s| {
        DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))
    });

    // Campaign content (subject/body) and counters live in the metrics
    // jsonb payload (Campaign Studio WYSIWYG editor persists content here).
    let mut metrics = serde_json::json!({
        "sent": 0,
        "delivered": 0,
        "failed": 0,
        "opened": 0,
        "clicked": 0,
        "replied": 0
    });
    if let Some(extra) = req.metrics {
        if let (Some(base), Some(extra_obj)) = (metrics.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_obj {
                base.insert(k.clone(), v.clone());
            }
        }
    }

    let campaign = CrmCampaign {
        id,
        branch_id,
        deal_id: None,
        name: req.name,
        status: Some("draft".to_string()),
        content_template: Some(serde_json::json!({})),
        sent_at: None,
        campaign_type: req.campaign_type.unwrap_or_else(|| "email".to_string()),
        starts_at,
        ends_at: None,
        budget: req.budget.map(|b| bigdecimal::BigDecimal::try_from(b).unwrap_or_default()),
        metrics: Some(metrics),
        run_offset: Some(0),
        pause_requested: Some(false),
        stop_requested: Some(false),
        started_at: None,
        completed_at: None,
        created_at: Some(now),
        updated_at: Some(now),
        org_id: None,
        bot_id: None,
    };

    diesel::insert_into(marketing_campaigns::table)
        .values(&campaign)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(campaign))
}

pub async fn update_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCampaignRequest>,
) -> Result<Json<CrmCampaign>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
        .set(marketing_campaigns::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if let Some(name) = req.name {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::name.eq(name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(status) = req.status {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::status.eq(Some(status)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(campaign_type) = req.campaign_type {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::campaign_type.eq(campaign_type))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(scheduled) = req.scheduled_at {
        let dt = DateTime::parse_from_rfc3339(&scheduled)
            .ok()
            .map(|d| d.with_timezone(&Utc));
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::starts_at.eq(dt))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(budget) = req.budget {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::budget.eq(bigdecimal::BigDecimal::try_from(budget).unwrap_or_default()))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }
    if let Some(metrics) = req.metrics {
        let mut stored: serde_json::Value = marketing_campaigns::table
            .filter(marketing_campaigns::id.eq(id))
            .select(marketing_campaigns::metrics)
            .first(&mut conn)
            .optional()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?
            .flatten()
            .unwrap_or_else(|| serde_json::json!({}));
        if let (Some(base), Some(extra_obj)) = (stored.as_object_mut(), metrics.as_object()) {
            for (k, v) in extra_obj {
                base.insert(k.clone(), v.clone());
            }
        }
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
            .set(marketing_campaigns::metrics.eq(Some(stored)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    get_campaign(State(state), Path(id)).await
}

pub async fn delete_campaign(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(marketing_campaigns::table.filter(marketing_campaigns::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

#[derive(Debug, Deserialize)]
pub struct SendCampaignRequest {
    pub list_id: Option<Uuid>,
    pub contact_ids: Option<Vec<Uuid>>,
    pub template_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CampaignSendResult {
    pub campaign_id: Uuid,
    pub total_recipients: i32,
    pub sent: i32,
    pub failed: i32,
    pub pending: i32,
}


pub async fn send_campaign(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    body: Option<Json<SendCampaignRequest>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let req = body.map(|Json(r)| r).unwrap_or(SendCampaignRequest {
        list_id: None,
        contact_ids: None,
        template_id: None,
    });
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let campaign: CrmCampaign = marketing_campaigns::table
        .filter(marketing_campaigns::id.eq(campaign_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Campaign not found".to_string()))?;

    let campaign_type = campaign.campaign_type.clone();
    let (_, _, bot_id) = state.get_scope();

    // Resolve recipients: explicit contact ids, then an explicit list id,
    // then whatever is already linked to this campaign. A misconfigured send
    // is a 400 error, never a 501.
    let contacts: Vec<(Uuid, String, String)> = if let Some(ids) = &req.contact_ids {
        let query = marketing_contacts::table
            .filter(marketing_contacts::id.eq_any(ids))
            .select((marketing_contacts::id, marketing_contacts::email, marketing_contacts::name))
            .load::<(Uuid, String, Option<String>)>(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
        query
            .into_iter()
            .map(|(cid, email, name)| (cid, email, name.unwrap_or_default()))
            .collect()
    } else if let Some(list_id) = req.list_id {
        let query = marketing_contacts::table
            .filter(marketing_contacts::list_id.eq(list_id))
            .select((marketing_contacts::id, marketing_contacts::email, marketing_contacts::name))
            .load::<(Uuid, String, Option<String>)>(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
        query
            .into_iter()
            .map(|(cid, email, name)| (cid, email, name.unwrap_or_default()))
            .collect()
    } else {
        let query = marketing_recipients::table
            .filter(marketing_recipients::campaign_id.eq(campaign_id))
            .select((
                marketing_recipients::contact_id,
                marketing_recipients::email,
                marketing_recipients::name,
            ))
            .load::<(Option<Uuid>, Option<String>, Option<String>)>(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
        query
            .into_iter()
            .map(|(rcid, email, name)| (rcid.unwrap_or_else(Uuid::new_v4), email.unwrap_or_default(), name.unwrap_or_default()))
            .collect()
    };

    if contacts.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No recipients resolved for campaign".to_string(),
        ));
    }

    // Resolve a template (subject + body) when requested; otherwise use the
    // campaign's stored settings payload.
    let (subject, body_html) = if let Some(template_id) = req.template_id {
        let row: Option<(Option<String>, Option<String>)> = marketing_templates::table
            .filter(marketing_templates::id.eq(template_id))
            .select((marketing_templates::subject, marketing_templates::body))
            .first(&mut conn)
            .optional()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;
        row.map(|(s, b)| (s.unwrap_or_default(), b))
            .unwrap_or_else(|| ("Newsletter".to_string(), None))
    } else {
        let default_metrics = serde_json::json!({});
        let template_data = campaign.metrics.as_ref().unwrap_or(&default_metrics);
        let subject = template_data
            .get("subject")
            .and_then(|s| s.as_str())
            .unwrap_or("Newsletter")
            .to_string();
        let body_html = template_data.get("body").and_then(|b| b.as_str()).map(String::from);
        (subject, body_html)
    };

    let mut sent = 0i32;
    let mut failed = 0i32;

    let is_whatsapp = campaign_type == "whatsapp";

    for (contact_id, email, name) in contacts {
        if is_whatsapp {
            let message = body_html.clone().unwrap_or_default();
            match (state.send_whatsapp)(bot_id, &email, &message, Some(&name), None) {
                Ok(_) => {
                    set_recipient_status(&mut conn, campaign_id, contact_id, "sent", None).ok();
                    sent += 1;
                }
                Err(e) => {
                    set_recipient_status(&mut conn, campaign_id, contact_id, "failed", Some(e)).ok();
                    failed += 1;
                }
            }
        } else {
            let personalized_body = body_html.as_ref().map(|html| {
                html.replace("{{name}}", &name).replace("{{email}}", &email)
            });
            let payload = crate::email::EmailCampaignPayload {
                to: email.clone(),
                subject: subject.clone(),
                body_html: personalized_body,
                body_text: None,
                campaign_id: Some(campaign_id),
                recipient_id: Some(contact_id),
            };
            let result = crate::email::send_campaign_email(&state, bot_id, payload)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
            if result.success {
                sent += 1;
            } else {
                failed += 1;
            }
        }
    }

    diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(campaign_id)))
        .set(marketing_campaigns::metrics.eq(Some(serde_json::json!({
            "sent": sent,
            "failed": failed,
            "status": "sent"
        }))))
        .execute(&mut conn)
        .ok();

    Ok(Json(serde_json::json!({
        "campaign_id": campaign_id,
        "total": sent + failed,
        "sent": sent,
        "failed": failed,
        "status": "completed"
    })))
}

/// Marks a campaign recipient as sent or failed, recording the error message
/// and timestamp when relevant.
fn set_recipient_status(
    conn: &mut diesel::PgConnection,
    campaign_id: Uuid,
    contact_id: Uuid,
    status: &str,
    error_message: Option<String>,
) -> diesel::QueryResult<usize> {
    diesel::update(
        marketing_recipients::table
            .filter(marketing_recipients::campaign_id.eq(campaign_id))
            .filter(marketing_recipients::contact_id.eq(contact_id)),
    )
    .set((
        marketing_recipients::status.eq(status),
        marketing_recipients::failed_at.eq(if status == "failed" {
            Some(chrono::Utc::now())
        } else {
            None
        }),
        marketing_recipients::error_message.eq(error_message),
    ))
    .execute(conn)
}
