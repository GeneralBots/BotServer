use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::campaigns::CrmCampaign;
use crate::metrics::{AggregateMetrics, ChannelBreakdown, TimeSeriesMetric};
use crate::schema::{email_tracking, marketing_campaigns, marketing_recipients};
use crate::state::AppState;

/// `(sent_at, delivered_at, failed_at)` timestamps of one recipient delivery.
type RecipientTimestamps = (
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
    Option<DateTime<Utc>>,
);

/// `(opened_at, clicked_at)` timestamps of one tracked email event.
type EmailEventTimestamps = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

pub async fn get_time_series_metrics(
    state: &Arc<AppState>,
    campaign_id: Uuid,
    interval_hours: i32,
) -> Result<Vec<TimeSeriesMetric>, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let recipients: Vec<RecipientTimestamps> = marketing_recipients::table
            .filter(marketing_recipients::campaign_id.eq(campaign_id))
            .select((
                marketing_recipients::sent_at,
                marketing_recipients::delivered_at,
                marketing_recipients::failed_at,
            ))
            .load(&mut conn)
            .map_err(|e| format!("Query error: {}", e))?;

    let mut sent_by_hour: HashMap<i64, i64> = HashMap::new();
    let mut delivered_by_hour: HashMap<i64, i64> = HashMap::new();
    let mut opened_by_hour: HashMap<i64, i64> = HashMap::new();
    let mut clicked_by_hour: HashMap<i64, i64> = HashMap::new();

    for (sent, delivered, failed) in recipients {
        if sent.is_some() || failed.is_some() {
            let ts = sent
                .or(failed)
                .map(|t| t.timestamp() / (interval_hours as i64 * 3600))
                .unwrap_or(0);
            *sent_by_hour.entry(ts).or_insert(0) += 1;
        }
        if let Some(d) = delivered {
            let ts = d.timestamp() / (interval_hours as i64 * 3600);
            *delivered_by_hour.entry(ts).or_insert(0) += 1;
        }
    }

    let email_events: Vec<EmailEventTimestamps> = email_tracking::table
            .filter(email_tracking::campaign_id.eq(campaign_id))
            .select((email_tracking::opened_at, email_tracking::clicked_at))
            .load(&mut conn)
            .unwrap_or_default();

    for (opened, clicked) in email_events {
        if let Some(ts) = opened {
            let key = ts.timestamp() / (interval_hours as i64 * 3600);
            *opened_by_hour.entry(key).or_insert(0) += 1;
        }
        if let Some(ts) = clicked {
            let key = ts.timestamp() / (interval_hours as i64 * 3600);
            *clicked_by_hour.entry(key).or_insert(0) += 1;
        }
    }

    let mut metrics: Vec<TimeSeriesMetric> = sent_by_hour
        .keys()
        .map(|&ts| TimeSeriesMetric {
            timestamp: DateTime::from_timestamp(ts * interval_hours as i64 * 3600, 0)
                .unwrap_or_else(Utc::now),
            sent: *sent_by_hour.get(&ts).unwrap_or(&0),
            delivered: *delivered_by_hour.get(&ts).unwrap_or(&0),
            opened: *opened_by_hour.get(&ts).unwrap_or(&0),
            clicked: *clicked_by_hour.get(&ts).unwrap_or(&0),
        })
        .collect();

    metrics.sort_by_key(|m| m.timestamp);
    Ok(metrics)
}

async fn get_channel_breakdown(
    conn: &mut diesel::PgConnection,
    campaign_ids: &[Uuid],
) -> Result<Vec<ChannelBreakdown>, String> {
    use crate::metrics::{calculate_click_rate, calculate_open_rate};

    let channels = vec!["email", "whatsapp", "instagram", "facebook", "telegram", "sms"];
    let mut breakdown = Vec::new();

    for channel in channels {
        let recipients: Vec<Option<String>> = marketing_recipients::table
            .filter(marketing_recipients::campaign_id.eq_any(campaign_ids))
            .filter(marketing_recipients::channel.eq(Some(channel)))
            .select(marketing_recipients::status)
            .load(conn)
            .unwrap_or_default();

        if recipients.is_empty() {
            continue;
        }

        let total = recipients.len() as i64;
        let sent = recipients.iter().filter(|s| s.as_deref() == Some("sent")).count() as i64;
        let delivered = recipients.iter().filter(|s| s.as_deref() == Some("delivered") || s.as_deref() == Some("read")).count() as i64;

        let opened = if channel == "email" {
            email_tracking::table
                .filter(email_tracking::campaign_id.eq_any(campaign_ids))
                .filter(email_tracking::opened.eq(true))
                .count()
                .get_result::<i64>(conn)
                .unwrap_or(0)
        } else {
            0
        };

        let clicked = if channel == "email" {
            email_tracking::table
                .filter(email_tracking::campaign_id.eq_any(campaign_ids))
                .filter(email_tracking::clicked.eq(true))
                .count()
                .get_result::<i64>(conn)
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

pub async fn get_aggregate_metrics(
    state: &Arc<AppState>,
    branch_id: Uuid,
) -> Result<AggregateMetrics, String> {
    let mut conn = state.conn.get().map_err(|e| format!("DB error: {}", e))?;

    let total_campaigns: i64 = marketing_campaigns::table
        .filter(marketing_campaigns::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_campaigns: i64 = marketing_campaigns::table
        .filter(marketing_campaigns::branch_id.eq(branch_id))
        .filter(marketing_campaigns::status.eq(Some("running")))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let campaigns: Vec<CrmCampaign> = marketing_campaigns::table
        .filter(marketing_campaigns::branch_id.eq(branch_id))
        .select(marketing_campaigns::all_columns)
        .load(&mut conn)
        .unwrap_or_default();

    let campaign_ids: Vec<Uuid> = campaigns.iter().map(|c| c.id).collect();

    let recipients: Vec<(Option<String>, Option<String>)> = marketing_recipients::table
        .filter(marketing_recipients::campaign_id.eq_any(campaign_ids.clone()))
        .select((marketing_recipients::channel, marketing_recipients::status))
        .load(&mut conn)
        .unwrap_or_default();

    let total_recipients = recipients.len() as i64;
    let total_sent = recipients.iter().filter(|(_, s)| s.as_deref() == Some("sent")).count() as i64;
    let total_delivered = recipients.iter().filter(|(_, s)| s.as_deref() == Some("delivered") || s.as_deref() == Some("read")).count() as i64;

    let total_opened: i64 = email_tracking::table
        .filter(email_tracking::campaign_id.eq_any(campaign_ids.clone()))
        .filter(email_tracking::opened.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_clicked: i64 = email_tracking::table
        .filter(email_tracking::campaign_id.eq_any(campaign_ids.clone()))
        .filter(email_tracking::clicked.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let avg_open_rate = if total_delivered > 0 {
        (total_opened as f64 / total_delivered as f64) * 100.0
    } else {
        0.0
    };

    let avg_click_rate = if total_delivered > 0 {
        (total_clicked as f64 / total_delivered as f64) * 100.0
    } else {
        0.0
    };

    let channel_breakdown = get_channel_breakdown(&mut conn, &campaign_ids).await?;

    Ok(AggregateMetrics {
        total_campaigns,
        active_campaigns,
        total_recipients,
        total_sent,
        total_delivered,
        total_opened,
        total_clicked,
        avg_open_rate,
        avg_click_rate,
        channel_breakdown,
    })
}

fn get_default_context(state: &Arc<AppState>) -> Uuid {
    let (_, branch_id, _) = state.get_scope();
    branch_id
}

pub async fn get_campaign_metrics_api(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
) -> Result<Json<crate::metrics::CampaignMetrics>, (StatusCode, String)> {
    match crate::metrics::get_campaign_metrics(&state, campaign_id).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn get_campaign_channel_breakdown_api(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
) -> Result<Json<Vec<ChannelBreakdown>>, (StatusCode, String)> {
    match crate::metrics::get_campaign_metrics_by_channel(&state, campaign_id).await {
        Ok(breakdown) => Ok(Json(breakdown)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn get_campaign_timeseries_api(
    State(state): State<Arc<AppState>>,
    Path((campaign_id, interval)): Path<(Uuid, i32)>,
) -> Result<Json<Vec<TimeSeriesMetric>>, (StatusCode, String)> {
    match get_time_series_metrics(&state, campaign_id, interval).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}

pub async fn get_aggregate_metrics_api(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AggregateMetrics>, (StatusCode, String)> {
    let branch_id = get_default_context(&state);

    match get_aggregate_metrics(&state, branch_id).await {
        Ok(metrics) => Ok(Json(metrics)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e)),
    }
}
