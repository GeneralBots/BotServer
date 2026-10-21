pub(crate) use bigdecimal::BigDecimal;
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use uuid::Uuid;
pub(crate) use crate::campaigns::CrmCampaign;
pub(crate) use crate::schema::advisor_recommendations;
pub(crate) use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecommendationSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckType {
    SpfDkimDmarc,
    BounceRate,
    OpenRate,
    SpamComplaints,
    NewIp,
    ListAge,
    UnsubscribeRate,
    DeliveryRate,
    EngagementTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub check_name: String,
    pub severity: RecommendationSeverity,
    pub message: String,
    pub details: Option<String>,
    pub action_items: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = advisor_recommendations)]
pub struct AdvisorRecommendation {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub campaign_id: Option<Uuid>,
    pub recommendation: String,
    pub reason: Option<String>,
    pub check_name: String,
    pub severity: String,
    pub message: String,
    pub details: Option<String>,
    pub dismissed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyzeCampaignRequest {
    pub campaign_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct AdvisorResponse {
    pub campaign_id: Uuid,
    pub recommendations: Vec<Recommendation>,
    pub summary: AdvisorSummary,
}

#[derive(Debug, Serialize)]
pub struct AdvisorSummary {
    pub total: usize,
    pub critical: usize,
    pub warnings: usize,
    pub info: usize,
}

#[derive(Debug, Clone)]
struct DbMetricsRow {
    sent_count: i64,
    delivered_count: i64,
    bounce_count: i64,
    open_count: i64,
    click_count: i64,
    complaint_count: i64,
    unsubscribe_count: i64,
    reply_count: i64,
}

pub struct AdvisorEngine;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
