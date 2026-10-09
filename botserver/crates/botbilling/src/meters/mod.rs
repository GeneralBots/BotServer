pub(crate) use crate::{BillingError, UsageMetric};
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

pub struct UsageMeteringService {
    storage: Arc<RwLock<MeteringStorage>>,
    aggregation_interval_secs: u64,
}

#[derive(Default)]
struct MeteringStorage {
    events: HashMap<Uuid, Vec<UsageEvent>>,
    aggregated: HashMap<Uuid, HashMap<UsageMetric, AggregatedUsage>>,
    daily_snapshots: HashMap<Uuid, Vec<DailySnapshot>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub metric: UsageMetric,
    pub value: i64,
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AggregatedUsage {
    pub metric: UsageMetric,
    pub total: u64,
    pub count: u64,
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub average: f64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySnapshot {
    pub organization_id: Uuid,
    pub date: chrono::NaiveDate,
    pub metrics: HashMap<UsageMetric, u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeteringReport {
    pub organization_id: Uuid,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub metrics: Vec<MetricReport>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricReport {
    pub metric: UsageMetric,
    pub current_value: u64,
    pub period_total: u64,
    pub period_average: f64,
    pub peak_value: u64,
    pub trend: UsageTrend,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageTrend {
    Increasing,
    Decreasing,
    Stable,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_event() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        let result = service
            .record_event(org_id, UsageMetric::Messages, 10, None)
            .await;
        assert!(result.is_ok());

        let event = result.unwrap();
        assert_eq!(event.organization_id, org_id);
        assert_eq!(event.metric, UsageMetric::Messages);
        assert_eq!(event.value, 10);
    }

    #[tokio::test]
    async fn test_get_current_usage() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        service.record_increment(org_id, UsageMetric::Messages, 10).await.unwrap();
        service.record_increment(org_id, UsageMetric::Messages, 20).await.unwrap();

        let usage = service.get_current_usage(org_id, UsageMetric::Messages).await;
        assert!(usage.is_ok());
        assert_eq!(usage.unwrap(), 30);
    }

    #[tokio::test]
    async fn test_get_aggregated_usage() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        service.record_increment(org_id, UsageMetric::ApiCalls, 5).await.unwrap();
        service.record_increment(org_id, UsageMetric::ApiCalls, 15).await.unwrap();

        let agg = service.get_aggregated_usage(org_id, UsageMetric::ApiCalls).await.unwrap().unwrap();
        assert_eq!(agg.total, 20);
        assert_eq!(agg.count, 2);
        assert_eq!(agg.average, 10.0);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();
        let now = Utc::now();

        service.record_increment(org_id, UsageMetric::Messages, 50).await.unwrap();

        let report = service
            .generate_report(org_id, now - chrono::Duration::days(1), now)
            .await
            .unwrap();

        assert_eq!(report.metrics.len(), 7);
    }

    #[tokio::test]
    async fn test_create_daily_snapshot() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        service.record_increment(org_id, UsageMetric::Messages, 25).await.unwrap();

        let snapshot = service.create_daily_snapshot(org_id).await.unwrap();
        assert_eq!(snapshot.metrics.get(&UsageMetric::Messages), Some(&25));
    }

    #[tokio::test]
    async fn test_reset_daily_metrics() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        service.record_increment(org_id, UsageMetric::Messages, 100).await.unwrap();
        service.record_increment(org_id, UsageMetric::ApiCalls, 50).await.unwrap();
        service.record_increment(org_id, UsageMetric::Bots, 3).await.unwrap();

        service.reset_daily_metrics(org_id).await.unwrap();

        let messages = service.get_current_usage(org_id, UsageMetric::Messages).await.unwrap();
        let api_calls = service.get_current_usage(org_id, UsageMetric::ApiCalls).await.unwrap();
        let bots = service.get_current_usage(org_id, UsageMetric::Bots).await.unwrap();

        assert_eq!(messages, 0);
        assert_eq!(api_calls, 0);
        assert_eq!(bots, 3);
    }

    #[tokio::test]
    async fn test_decrement_reduces_total() {
        let service = UsageMeteringService::new();
        let org_id = Uuid::new_v4();

        service.record_increment(org_id, UsageMetric::StorageBytes, 1000).await.unwrap();
        service.record_decrement(org_id, UsageMetric::StorageBytes, 300).await.unwrap();

        let usage = service.get_current_usage(org_id, UsageMetric::StorageBytes).await.unwrap();
        assert_eq!(usage, 700);
    }

    #[test]
    fn test_usage_trend_variants() {
        let trends = vec![
            UsageTrend::Increasing,
            UsageTrend::Decreasing,
            UsageTrend::Stable,
            UsageTrend::Unknown,
        ];

        for trend in trends {
            let serialized = serde_json::to_string(&trend).unwrap();
            let deserialized: UsageTrend = serde_json::from_str(&serialized).unwrap();
            assert_eq!(trend, deserialized);
        }
    }
}

mod part1;
mod part2;
mod part3;

pub use part3::*;
