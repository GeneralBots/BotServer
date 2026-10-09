pub(crate) use crate::{BillingError, LimitValue, UsageMetric};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use uuid::Uuid;

pub struct QuotaManager {
    usage_cache: Arc<RwLock<HashMap<Uuid, OrganizationQuotas>>>,
    alert_thresholds: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct OrganizationQuotas {
    pub organization_id: Uuid,
    pub plan_id: String,
    pub limits: QuotaLimits,
    pub usage: QuotaUsage,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct QuotaLimits {
    pub messages_per_day: LimitValue,
    pub storage_bytes: LimitValue,
    pub api_calls_per_day: LimitValue,
    pub bots: LimitValue,
    pub users: LimitValue,
    pub kb_documents: LimitValue,
    pub apps: LimitValue,
    pub emails_per_day: LimitValue,
    pub builds_per_day: LimitValue,
    pub uploads_per_hour: LimitValue,
}

#[derive(Debug, Clone, Default)]
pub struct QuotaUsage {
    pub messages_today: u64,
    pub storage_bytes: u64,
    pub api_calls_today: u64,
    pub bots: u64,
    pub users: u64,
    pub kb_documents: u64,
    pub apps: u64,
    pub emails_today: u64,
    pub builds_today: u64,
    pub uploads_this_hour: u64,
    pub last_reset: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub metric: UsageMetric,
    pub current: u64,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub percentage_used: f64,
    pub alerts: Vec<QuotaAlert>,
}

#[derive(Debug, Clone)]
pub struct QuotaAlert {
    pub metric: UsageMetric,
    pub threshold: f64,
    pub current_percentage: f64,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum QuotaAction {
    Allow,
    Warn { message: String, percentage: f64 },
    Block { message: String },
}

#[derive(Debug, Clone)]
pub struct UsageSummary {
    pub organization_id: Uuid,
    pub plan_id: String,
    pub items: Vec<UsageSummaryItem>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct UsageSummaryItem {
    pub metric: UsageMetric,
    pub current: u64,
    pub limit: Option<u64>,
    pub remaining: Option<u64>,
    pub percentage_used: f64,
    pub is_unlimited: bool,
}

pub struct QuotaMiddleware {
    quota_manager: Arc<QuotaManager>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LimitValue;

    fn create_test_quotas(org_id: Uuid, plan_id: &str) -> OrganizationQuotas {
        let now = chrono::Utc::now();
        OrganizationQuotas {
            organization_id: org_id,
            plan_id: plan_id.to_string(),
            limits: QuotaLimits {
                messages_per_day: LimitValue::Limited(100),
                storage_bytes: LimitValue::Limited(1024 * 1024 * 100),
                api_calls_per_day: LimitValue::Limited(1000),
                bots: LimitValue::Limited(5),
                users: LimitValue::Limited(10),
                kb_documents: LimitValue::Limited(50),
                apps: LimitValue::Limited(10),
                emails_per_day: LimitValue::Limited(50),
                builds_per_day: LimitValue::Limited(25),
                uploads_per_hour: LimitValue::Limited(25),
            },
            usage: QuotaUsage::default(),
            period_start: now,
            period_end: now + chrono::Duration::days(30),
        }
    }

    fn create_unlimited_quotas(org_id: Uuid) -> OrganizationQuotas {
        let now = chrono::Utc::now();
        OrganizationQuotas {
            organization_id: org_id,
            plan_id: "enterprise".to_string(),
            limits: QuotaLimits {
                messages_per_day: LimitValue::Unlimited,
                storage_bytes: LimitValue::Unlimited,
                api_calls_per_day: LimitValue::Unlimited,
                bots: LimitValue::Unlimited,
                users: LimitValue::Unlimited,
                kb_documents: LimitValue::Unlimited,
                apps: LimitValue::Unlimited,
                emails_per_day: LimitValue::Unlimited,
                builds_per_day: LimitValue::Unlimited,
                uploads_per_hour: LimitValue::Unlimited,
            },
            usage: QuotaUsage::default(),
            period_start: now,
            period_end: now + chrono::Duration::days(30),
        }
    }

    #[tokio::test]
    async fn test_set_and_get_quotas() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        let quotas = create_test_quotas(org_id, "business");

        manager.set_quotas(quotas.clone()).await;

        let retrieved = manager.get_quotas(org_id).await;
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.organization_id, org_id);
        assert_eq!(retrieved.plan_id, "business");
    }

    #[tokio::test]
    async fn test_check_quota_within_limit() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        let result = manager.check_quota(org_id, UsageMetric::Messages, 50).await;
        assert!(result.is_ok());
        let check = result.unwrap();
        assert!(check.allowed);
        assert_eq!(check.current, 50);
        assert_eq!(check.limit, Some(100));
        assert_eq!(check.remaining, Some(50));
        assert_eq!(check.percentage_used, 50.0);
    }

    #[tokio::test]
    async fn test_check_quota_exceeds_limit() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        let result = manager.check_quota(org_id, UsageMetric::Messages, 101).await;
        assert!(result.is_ok());
        let check = result.unwrap();
        assert!(!check.allowed);
        assert_eq!(check.remaining, Some(0));
    }

    #[tokio::test]
    async fn test_check_quota_unlimited() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_unlimited_quotas(org_id)).await;

        let result = manager.check_quota(org_id, UsageMetric::Messages, 1_000_000).await;
        assert!(result.is_ok());
        let check = result.unwrap();
        assert!(check.allowed);
        assert_eq!(check.limit, None);
        assert_eq!(check.percentage_used, 0.0);
    }

    #[tokio::test]
    async fn test_increment_usage() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        let result = manager.increment_usage(org_id, UsageMetric::Messages, 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);

        let quotas = manager.get_quotas(org_id).await.unwrap();
        assert_eq!(quotas.usage.messages_today, 10);
    }

    #[tokio::test]
    async fn test_decrement_usage() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        manager.increment_usage(org_id, UsageMetric::Bots, 3).await.unwrap();
        let result = manager.decrement_usage(org_id, UsageMetric::Bots, 1).await;
        assert!(result.is_ok());

        let quotas = manager.get_quotas(org_id).await.unwrap();
        assert_eq!(quotas.usage.bots, 2);
    }

    #[tokio::test]
    async fn test_reset_daily_quotas() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        manager.increment_usage(org_id, UsageMetric::Messages, 50).await.unwrap();
        manager.increment_usage(org_id, UsageMetric::ApiCalls, 100).await.unwrap();
        manager.increment_usage(org_id, UsageMetric::Bots, 2).await.unwrap();

        let result = manager.reset_daily_quotas(org_id).await;
        assert!(result.is_ok());

        let quotas = manager.get_quotas(org_id).await.unwrap();
        assert_eq!(quotas.usage.messages_today, 0);
        assert_eq!(quotas.usage.api_calls_today, 0);
        assert_eq!(quotas.usage.bots, 2);
        assert!(quotas.usage.last_reset.is_some());
    }

    #[tokio::test]
    async fn test_check_action_allow() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        let action = manager.check_action(org_id, UsageMetric::Messages).await;
        assert!(matches!(action, QuotaAction::Allow));
    }

    #[tokio::test]
    async fn test_check_action_warn() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        manager.set_usage(org_id, UsageMetric::Messages, 91).await.unwrap();

        let action = manager.check_action(org_id, UsageMetric::Messages).await;
        assert!(matches!(action, QuotaAction::Warn { .. }));
    }

    #[tokio::test]
    async fn test_check_action_block() {
        let manager = QuotaManager::new();
        let org_id = Uuid::new_v4();
        manager.set_quotas(create_test_quotas(org_id, "business")).await;

        manager.set_usage(org_id, UsageMetric::Messages, 100).await.unwrap();

        let action = manager.check_action(org_id, UsageMetric::Messages).await;
        assert!(matches!(action, QuotaAction::Block { .. }));
    }

    #[test]
    fn test_quota_usage_default() {
        let usage = QuotaUsage::default();
        assert_eq!(usage.messages_today, 0);
        assert_eq!(usage.storage_bytes, 0);
        assert_eq!(usage.api_calls_today, 0);
        assert!(usage.last_reset.is_none());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part4::*;
