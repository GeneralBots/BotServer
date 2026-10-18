//! Multi-channel campaign publish (Campaign Studio fan-out).
//!
//! One designed campaign (metrics.subject/body/images jsonb) distributes to
//! email, WhatsApp and Instagram recipients from a chosen list. Recipient
//! tracking rows are written first so the unified monitor reflects every
//! per-recipient status (sent/failed/delivered/opened/clicked).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::email::EmailCampaignPayload;
use crate::schema::{marketing_campaigns, marketing_contacts, marketing_recipients};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub list_id: Option<Uuid>,
    pub channels: Option<Vec<String>>,
}

/// Minimal HTML tag stripper for WhatsApp/Instagram plain-text payloads.
fn plain_text(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for c in html.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            out.push(' ');
        } else if !in_tag {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub async fn publish_campaign(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Json(req): Json<PublishRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = state.conn.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?;

    let campaign: crate::campaigns::CrmCampaign = marketing_campaigns::table
        .filter(marketing_campaigns::id.eq(campaign_id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Campaign not found".to_string()))?;

    let (_, _, bot_id) = state.get_scope();
    let metrics = campaign.metrics.clone().unwrap_or_else(|| serde_json::json!({}));
    let subject = metrics
        .get("subject")
        .and_then(|s| s.as_str())
        .unwrap_or("Newsletter")
        .to_string();
    let body_html = metrics.get("body").and_then(|b| b.as_str()).unwrap_or("").to_string();
    let images = metrics.get("images").cloned().unwrap_or_else(|| serde_json::json!({}));

    let list_id = req.list_id.ok_or_else(|| {
        (StatusCode::BAD_REQUEST, "list_id is required — choose a marketing list".to_string())
    })?;

    // Resolve recipients from the list's marketing_contacts rows.
    let contacts: Vec<(Uuid, String, Option<String>, Option<String>)> = marketing_contacts::table
        .filter(marketing_contacts::list_id.eq(list_id))
        .select((
            marketing_contacts::id,
            marketing_contacts::email,
            marketing_contacts::name,
            marketing_contacts::phone,
        ))
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Recipients query: {e}")))?;

    if contacts.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "No recipients resolved for this list".to_string()));
    }

    let channels: Vec<String> = match req.channels {
        Some(c) if !c.is_empty() => c,
        _ => match campaign.campaign_type.as_str() {
            "multi" => vec!["email".into(), "whatsapp".into(), "instagram".into()],
            "whatsapp" => vec!["whatsapp".into()],
            "instagram" => vec!["instagram".into()],
            _ => vec!["email".into()],
        },
    };

    // Rewrite tracking rows for this send: fresh queued state per recipient.
    diesel::delete(
        marketing_recipients::table.filter(marketing_recipients::campaign_id.eq(campaign_id)),
    )
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Reset recipients: {e}")))?;

    let now = chrono::Utc::now();
    for (cid, email, name, _phone) in &contacts {
        diesel::insert_into(marketing_recipients::table)
            .values((
                marketing_recipients::id.eq(Uuid::new_v4()),
                marketing_recipients::branch_id.eq(campaign.branch_id),
                marketing_recipients::campaign_id.eq(campaign_id),
                marketing_recipients::list_id.eq(list_id),
                marketing_recipients::contact_id.eq(*cid),
                marketing_recipients::email.eq(email.clone()),
                marketing_recipients::name.eq(name.clone()),
                marketing_recipients::status.eq("queued"),
                marketing_recipients::created_at.eq(now),
            ))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Queue recipients: {e}")))?;
    }

    let mut totals = serde_json::Map::new();

    for channel in &channels {
        let mut sent = 0i64;
        let mut failed = 0i64;
        let mut skipped = 0i64;
        match channel.as_str() {
            "email" => {
                for (cid, email, name, _phone) in &contacts {
                    if email.trim().is_empty() {
                        skipped += 1;
                        continue;
                    }
                    let personalized = body_html
                        .replace("{{name}}", name.as_deref().unwrap_or(""))
                        .replace("{{email}}", email);
                    let result = crate::email::send_campaign_email(
                        &state,
                        bot_id,
                        EmailCampaignPayload {
                            to: email.clone(),
                            subject: subject.clone(),
                            body_html: Some(personalized),
                            body_text: None,
                            campaign_id: Some(campaign_id),
                            recipient_id: Some(*cid),
                        },
                    )
                    .await;
                    match result {
                        Ok(r) if r.success => {
                            sent += 1;
                            set_recipient_state(&mut conn, campaign_id, *cid, "sent", None).ok();
                        }
                        Ok(_) => {
                            failed += 1;
                            set_recipient_state(&mut conn, campaign_id, *cid, "failed", None).ok();
                        }
                        Err(e) => {
                            failed += 1;
                            log::warn!("email send failed for {email}: {e}");
                            set_recipient_state(&mut conn, campaign_id, *cid, "failed", Some(&e)).ok();
                        }
                    }
                }
            }
            "whatsapp" => {
                for (cid, _email, name, phone) in &contacts {
                    let number = phone.as_deref().unwrap_or("").trim();
                    if number.is_empty() {
                        skipped += 1;
                        continue;
                    }
                    let message = plain_text(&body_html);
                    match (state.send_whatsapp)(bot_id, number, &message, name.as_deref(), None) {
                        Ok(_) => {
                            sent += 1;
                            set_recipient_state(&mut conn, campaign_id, *cid, "sent", None).ok();
                        }
                        Err(e) => {
                            failed += 1;
                            log::warn!("whatsapp send failed for {number}: {e}");
                            set_recipient_state(&mut conn, campaign_id, *cid, "failed", Some(&e)).ok();
                        }
                    }
                }
            }
            "instagram" => {
                let image_url = images
                    .get("instagram")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .or_else(|| {
                        images
                            .get("email")
                            .and_then(|v| v.as_array())
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_str())
                    })
                    .map(String::from);
                let caption = plain_text(&body_html);
                match image_url {
                    Some(url) => {
                        let result = publish_instagram(&state, &bot_id, &url, &caption).await;
                        match result {
                            Ok(_) => {
                                sent += 1;
                                let _ = set_recipient_state(&mut conn, campaign_id, Uuid::nil(), "sent", None);
                            }
                            Err(e) => {
                                failed += 1;
                                log::warn!("instagram publish failed: {e}");
                                let _ = set_recipient_state(&mut conn, campaign_id, Uuid::nil(), "failed", Some(&e));
                            }
                        }
                    }
                    None => {
                        skipped += 1;
                        log::warn!("instagram publish skipped: no generated image in metrics.images");
                    }
                }
            }
            _ => {
                skipped += 1;
                log::warn!("channel {channel} has no provider — skipped");
            }
        }
        totals.insert(
            channel.clone(),
            serde_json::json!({ "sent": sent, "failed": failed, "skipped": skipped }),
        );
    }

    // Merge fan-out totals into the campaign metrics (counters preserved).
    merge_metrics(&mut conn, campaign_id, &totals).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Metrics update: {e}"))
    })?;

    let total_sent: i64 = totals.values().map(|v| v["sent"].as_i64().unwrap_or(0)).sum();
    let total_failed: i64 = totals.values().map(|v| v["failed"].as_i64().unwrap_or(0)).sum();

    Ok(Json(serde_json::json!({
        "success": true,
        "sent": total_sent,
        "failed": total_failed,
        "channels": totals,
    })))
}

async fn publish_instagram(
    state: &Arc<AppState>,
    bot_id: &Uuid,
    image_url: &str,
    caption: &str,
) -> Result<String, String> {
    let inner = state.get_config.clone();
    let get_config: botinstagram::state::GetConfigFn = std::sync::Arc::new(move |bot_id_str, key, default| {
        inner(&Uuid::parse_str(bot_id_str).unwrap_or_else(|_| Uuid::nil()), key, default)
    });
    let adapter = botinstagram::adapter::InstagramAdapter::with_config(&get_config, &bot_id.to_string());
    adapter
        .post_to_instagram(image_url, caption)
        .await
        .map_err(|e| e.to_string())
}

fn set_recipient_state(
    conn: &mut diesel::PgConnection,
    campaign_id: Uuid,
    contact_id: Uuid,
    status: &str,
    error_message: Option<&str>,
) -> diesel::QueryResult<usize> {
    let target = marketing_recipients::table
        .filter(marketing_recipients::campaign_id.eq(campaign_id))
        .filter(marketing_recipients::contact_id.eq(contact_id));
    if status == "failed" {
        diesel::update(target)
            .set((
                marketing_recipients::status.eq(status),
                marketing_recipients::failed_at.eq(Some(chrono::Utc::now())),
                marketing_recipients::error_message.eq(error_message.map(String::from)),
            ))
            .execute(conn)
    } else {
        diesel::update(target)
            .set((
                marketing_recipients::status.eq(status),
                marketing_recipients::sent_at.eq(Some(chrono::Utc::now())),
                marketing_recipients::error_message.eq(error_message.map(String::from)),
            ))
            .execute(conn)
    }
}

fn merge_metrics(
    conn: &mut diesel::PgConnection,
    campaign_id: Uuid,
    totals: &serde_json::Map<String, serde_json::Value>,
) -> diesel::QueryResult<()> {
    let mut stored: serde_json::Value = marketing_campaigns::table
        .filter(marketing_campaigns::id.eq(campaign_id))
        .select(marketing_campaigns::metrics)
        .first::<Option<serde_json::Value>>(conn)
        .optional()?
        .flatten()
        .unwrap_or_else(|| serde_json::json!({}));

    let total_sent: i64 = totals.values().map(|v| v["sent"].as_i64().unwrap_or(0)).sum();
    let total_failed: i64 = totals.values().map(|v| v["failed"].as_i64().unwrap_or(0)).sum();

    if let Some(obj) = stored.as_object_mut() {
        let sent = obj.get("sent").and_then(|v| v.as_i64()).unwrap_or(0) + total_sent;
        let failed = obj.get("failed").and_then(|v| v.as_i64()).unwrap_or(0) + total_failed;
        obj.insert("sent".into(), serde_json::json!(sent));
        obj.insert("failed".into(), serde_json::json!(failed));
        obj.insert("channels".into(), serde_json::json!(totals));
    }

    diesel::update(marketing_campaigns::table.filter(marketing_campaigns::id.eq(campaign_id)))
        .set((
            marketing_campaigns::metrics.eq(Some(stored)),
            marketing_campaigns::status.eq(Some(if total_failed > 0 && total_sent == 0 { "failed" } else { "completed" })),
        ))
        .execute(conn)?;

    Ok(())
}