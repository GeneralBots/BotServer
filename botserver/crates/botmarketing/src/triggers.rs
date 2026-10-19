use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::schema::{email_tracking, marketing_campaigns, marketing_recipients, system_automations};
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKind {
    DealStageChange = 0,
    ContactChange = 1,
    EmailOpened = 2,
}

/// Row of the canonical `system_automations` table (`botschema::system_automations`).
#[derive(Debug, Clone, Queryable)]
#[diesel(table_name = system_automations)]
pub struct Automation {
    pub id: Uuid,
    pub bot_id: Uuid,
    pub kind: i32,
    pub target: Option<String>,
    pub schedule: Option<String>,
    pub param: String,
    pub is_active: bool,
    pub last_triggered: Option<DateTime<Utc>>,
    pub branch_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub name: Option<String>,
}

pub fn trigger_deal_stage_change(
    conn: &mut diesel::PgConnection,
    deal_id: Uuid,
    _old_stage: &str,
    new_stage: &str,
    _bot_id: Uuid,
) {
    let automations: Vec<Automation> = system_automations::table
        .filter(system_automations::kind.eq(TriggerKind::DealStageChange as i32))
        .filter(system_automations::is_active.eq(true))
        .filter(system_automations::bot_id.eq(_bot_id))
        .load(conn)
        .unwrap_or_default();

    for automation in automations {
        let target_stage = automation.target.as_deref().unwrap_or("");
        let param = automation.param.as_str();
        if target_stage.is_empty() || target_stage == new_stage {
            if !param.is_empty() {
                if let Err(e) = execute_campaign_for_deal(conn, param, deal_id) {
                    log::error!("Failed to trigger campaign for deal stage change: {}", e);
                }
            }
        }
    }
}

pub fn trigger_contact_change(
    conn: &mut diesel::PgConnection,
    contact_id: Uuid,
    change_type: &str,
    _bot_id: Uuid,
) {
    let automations: Vec<Automation> = system_automations::table
        .filter(system_automations::kind.eq(TriggerKind::ContactChange as i32))
        .filter(system_automations::is_active.eq(true))
        .filter(system_automations::bot_id.eq(_bot_id))
        .load(conn)
        .unwrap_or_default();

    for automation in automations {
        let target_value = automation.target.as_deref().unwrap_or("");
        let param = automation.param.as_str();
        if target_value.is_empty() || target_value == change_type {
            if !param.is_empty() {
                if let Err(e) = execute_campaign_for_contact(conn, param, contact_id) {
                    log::error!("Failed to trigger campaign for contact change: {}", e);
                }
            }
        }
    }
}

pub fn trigger_email_opened(
    conn: &mut diesel::PgConnection,
    campaign_id: Uuid,
    contact_id: Uuid,
    _bot_id: Uuid,
) {
    let automations: Vec<Automation> = system_automations::table
        .filter(system_automations::kind.eq(TriggerKind::EmailOpened as i32))
        .filter(system_automations::is_active.eq(true))
        .filter(system_automations::bot_id.eq(_bot_id))
        .load(conn)
        .unwrap_or_default();

    for automation in automations {
        let target_campaign = automation.target.as_deref().unwrap_or("");
        let param = automation.param.as_str();
        if target_campaign.is_empty() || target_campaign == campaign_id.to_string() {
            if !param.is_empty() {
                if let Err(e) = execute_campaign_for_contact(conn, param, contact_id) {
                    log::error!("Failed to trigger campaign for email opened: {}", e);
                }
            }
        }
    }
}

fn execute_campaign_for_deal(
    conn: &mut diesel::PgConnection,
    campaign_id_str: &str,
    deal_id_val: Uuid,
) -> Result<(), diesel::result::Error> {
    if let Ok(cid) = Uuid::parse_str(campaign_id_str) {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(cid)))
            .set((
                marketing_campaigns::status.eq(Some("triggered")),
                marketing_campaigns::ends_at.eq(Some(chrono::Utc::now())),
            ))
            .execute(conn)?;
        log::info!("Campaign {} triggered for deal {}", campaign_id_str, deal_id_val);
    }
    Ok(())
}

fn execute_campaign_for_contact(
    conn: &mut diesel::PgConnection,
    campaign_id_str: &str,
    contact_id: Uuid,
) -> Result<(), diesel::result::Error> {
    if let Ok(cid) = Uuid::parse_str(campaign_id_str) {
        diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(cid)))
            .set((
                marketing_campaigns::status.eq("triggered"),
                marketing_campaigns::ends_at.eq(Some(chrono::Utc::now())),
            ))
            .execute(conn)?;

        diesel::insert_into(marketing_recipients::table)
            .values((
                marketing_recipients::id.eq(Uuid::new_v4()),
                marketing_recipients::branch_id.eq(uuid::Uuid::nil()),
                marketing_recipients::campaign_id.eq(Some(cid)),
                marketing_recipients::contact_id.eq(Some(contact_id)),
                marketing_recipients::email.eq(""),
                marketing_recipients::channel.eq("automation"),
                marketing_recipients::status.eq("pending"),
                marketing_recipients::created_at.eq(chrono::Utc::now()),
            ))
            .execute(conn)?;

        log::info!("Campaign {} triggered for contact {}", campaign_id_str, contact_id);
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct EmailOpenRequest {
    pub message_id: Option<String>,
    pub token: Option<String>,
}

pub async fn track_email_open(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EmailOpenRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    if let Some(token_str) = req.token {
        if let Ok(token) = Uuid::parse_str(&token_str) {
            let record: Option<(Uuid, Option<Uuid>)> = email_tracking::table
                .filter(email_tracking::open_token.eq(Some(token)))
                .select((email_tracking::id, email_tracking::recipient_id))
                .first(&mut conn)
                .ok();

            if let Some((id, recipient_id)) = record {
                diesel::update(email_tracking::table.filter(email_tracking::id.eq(id)))
                    .set((
                        email_tracking::opened.eq(true),
                        email_tracking::opened_at.eq(Some(now)),
                    ))
                    .execute(&mut conn)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

                log::info!("Email opened: tracking_id={}", id);

                if let Some(contact_id) = recipient_id {
                    let (bot_id, _) = (state.get_default_bot)(&mut conn);
                    trigger_email_opened(&mut conn, Uuid::nil(), contact_id, bot_id);
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "tracked" })))
}
