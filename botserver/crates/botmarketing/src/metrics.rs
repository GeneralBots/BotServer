use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::campaigns::CrmCampaign;
use crate::schema::{email_tracking, marketing_campaigns, marketing_recipients};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignMetrics {
    pub campaign_id: Uuid,
    pub channel: String,
    pub total_recipients: i64,
    pub sent: i64,
    pub delivered: i64,
    pub failed: i64,
    pub opened: i64,
    pub clicked: i64,
    pub replied: i64,
    pub open_rate: f64,
    pub click_rate: f64,
    pub conversion_rate: f64,
    pub cost_per_result: f64,
    pub total_spend: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelBreakdown {
    pub channel: String,
    pub recipients: i64,
    pub sent: i64,
    pub delivered: i64,
    pub opened: i64,
    pub clicked: i64,
    pub open_rate: f64,
    pub click_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetric {
    pub timestamp: DateTime<Utc>,
    pub sent: i64,
    pub delivered: i64,
    pub opened: i64,
    pub clicked: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateMetrics {
    pub total_campaigns: i64,
    pub active_campaigns: i64,
    pub total_recipients: i64,
    pub total_sent: i64,
    pub total_delivered: i64,
    pub total_opened: i64,
    pub total_clicked: i64,
    pub avg_open_rate: f64,
    pub avg_click_rate: f64,
    pub channel_breakdown: Vec<ChannelBreakdown>,
}

pub fn calculate_open_rate(delivered: i64, opened: i64) -> f64 {
    if delivered > 0 {
        (opened as f64 / delivered as f64) * 100.0
    } else {
        0.0
    }
}

pub fn calculate_click_rate(delivered: i64, clicked: i64) -> f64 {
    if delivered > 0 {
        (clicked as f64 / delivered as f64) * 100.0
    } else {
        0.0
    }
}

pub async fn get_campaign_metrics(
    state: &Arc<AppState>,
    campaign_id: Uuid,
) -> Result<CampaignMetrics, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let campaign: CrmCampaign = marketing_campaigns::table
        .filter(marketing_campaigns::id.eq(campaign_id))
        .first(&mut conn)
        .map_err(|_| "Campaign not found")?;

    let recipients: Vec<(Option<String>, Option<serde_json::Value>)> = marketing_recipients::table
        .filter(marketing_recipients::campaign_id.eq(campaign_id))
        .select((marketing_recipients::status, marketing_recipients::response))
        .load(&mut conn)
        .map_err(|e| format!("Query error: {}", e))?;

    let parse_response = |r: &Option<serde_json::Value>| -> Option<serde_json::Value> { r.clone() };

    let total = recipients.len() as i64;
    let sent = recipients.iter().filter(|(s, _)| s.as_deref() == Some("sent")).count() as i64;
    let delivered = recipients
        .iter()
        .filter(|(s, r)| {
            let is_delivered = s.as_deref() == Some("delivered") || s.as_deref() == Some("sent");
            let has_delivery_status = parse_response(r)
                .and_then(|v| v.get("status").cloned())
                .and_then(|s| s.as_str().map(|st| st == "delivered" || st == "read"))
                .unwrap_or(false);
            is_delivered || has_delivery_status
        })
        .count() as i64;
    let failed = recipients.iter().filter(|(s, _)| s.as_deref() == Some("failed")).count() as i64;
    let replied = recipients
        .iter()
        .filter(|(_, r)| {
            parse_response(r)
                .and_then(|v| v.get("type").cloned())
                .and_then(|t| t.as_str().map(|t| t == "reply"))
                .unwrap_or(false)
        })
        .count() as i64;

    let email_opens = if campaign.campaign_type == "email" || campaign.campaign_type == "multi" {
        email_tracking::table
            .filter(email_tracking::campaign_id.eq(campaign_id))
            .filter(email_tracking::opened.eq(true))
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap_or(0)
    } else {
        0
    };

    let email_clicks = if campaign.campaign_type == "email" || campaign.campaign_type == "multi" {
        email_tracking::table
            .filter(email_tracking::campaign_id.eq(campaign_id))
            .filter(email_tracking::clicked.eq(true))
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap_or(0)
    } else {
        0
    };

    let open_rate = calculate_open_rate(delivered, email_opens);
    let click_rate = calculate_click_rate(delivered, email_clicks);
    let conversion_rate = if delivered > 0 {
        (replied as f64 / delivered as f64) * 100.0
    } else {
        0.0
    };

    let budget: f64 = campaign.budget.as_ref().and_then(|b| b.to_string().parse::<f64>().ok()).unwrap_or(0.0);
    let cost_per_result = if sent > 0 { budget / sent as f64 } else { 0.0 };

    Ok(CampaignMetrics {
        campaign_id,
        channel: campaign.campaign_type,
        total_recipients: total,
        sent,
        delivered,
        failed,
        opened: email_opens,
        clicked: email_clicks,
        replied,
        open_rate,
        click_rate,
        conversion_rate,
        cost_per_result,
        total_spend: budget,
    })
}

pub async fn get_campaign_metrics_by_channel(
    state: &Arc<AppState>,
    campaign_id: Uuid,
) -> Result<Vec<ChannelBreakdown>, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let channels = vec!["email", "whatsapp", "instagram", "facebook", "telegram", "sms"];
    let mut breakdown = Vec::new();

    for channel in channels {
        let recipients: Vec<Option<String>> = marketing_recipients::table
            .filter(marketing_recipients::campaign_id.eq(campaign_id))
            .filter(marketing_recipients::channel.eq(channel))
            .select(marketing_recipients::status)
            .load(&mut conn)
            .unwrap_or_default();

        if recipients.is_empty() {
            continue;
        }

        let total = recipients.len() as i64;
        let sent = recipients.iter().filter(|s| s.as_deref() == Some("sent")).count() as i64;
        let delivered = recipients.iter().filter(|s| s.as_deref() == Some("delivered") || s.as_deref() == Some("read")).count() as i64;
        let opened = if channel == "email" {
            email_tracking::table
                .filter(email_tracking::campaign_id.eq(campaign_id))
                .filter(email_tracking::opened.eq(true))
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap_or(0)
        } else {
            0
        };
        let clicked = if channel == "email" {
            email_tracking::table
                .filter(email_tracking::campaign_id.eq(campaign_id))
                .filter(email_tracking::clicked.eq(true))
                .count()
                .get_result::<i64>(&mut conn)
                .unwrap_or(0)
        } else {
            0
        };

        breakdown.push(ChannelBreakdown {
            channel: channel.to_string(),
            recipients: total,
            sent,
            delivered,
            opened,
            clicked,
            open_rate: calculate_open_rate(delivered, opened),
            click_rate: calculate_click_rate(delivered, clicked),
        });
    }

    Ok(breakdown)
}
