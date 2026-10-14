use super::*;

pub use botlib::db_pool::DbPool;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingAlertNotification {
    pub alert_id: Uuid,
    pub organization_id: Uuid,
    pub severity: String,
    pub alert_type: String,
    pub title: String,
    pub message: String,
    pub metric: String,
    pub percentage: f64,
    pub triggered_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductConfig {
    pub branding: BrandingConfig,
    pub plans: HashMap<String, PlanConfig>,
    pub features: Vec<String>,
    pub stripe: Option<StripeConfig>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub name: String,
    pub logo: Option<String>,
    pub primary_color: String,
    pub secondary_color: Option<String>,
    pub favicon: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanConfig {
    pub name: String,
    pub description: Option<String>,
    pub price: PlanPrice,
    pub limits: PlanLimits,
    pub features: Vec<String>,
    pub stripe_price_id: Option<String>,
    pub trial_days: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlanPrice {
    Free,
    Fixed { amount: u64, currency: String, period: BillingPeriod },
    Custom,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BillingPeriod {
    Monthly,
    Yearly,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLimits {
    pub messages_per_day: LimitValue,
    pub storage_mb: LimitValue,
    pub bots: LimitValue,
    pub users: LimitValue,
    pub api_calls_per_day: LimitValue,
    pub signups_per_day: Option<LimitValue>,
    pub kb_documents: LimitValue,
    pub apps: LimitValue,
    pub emails_per_day: LimitValue,
    pub builds_per_day: LimitValue,
    pub uploads_per_hour: LimitValue,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LimitValue {
    Limited(u64),
    Unlimited,
}
impl LimitValue {
    pub fn is_unlimited(&self) -> bool {
        matches!(self, Self::Unlimited)
    }

    pub fn value(&self) -> Option<u64> {
        match self {
            Self::Limited(v) => Some(*v),
            Self::Unlimited => None,
        }
    }

    pub fn check(&self, current: u64) -> bool {
        match self {
            Self::Limited(limit) => current < *limit,
            Self::Unlimited => true,
        }
    }
}
impl Default for LimitValue {
    fn default() -> Self {
        Self::Limited(0)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeConfig {
    pub publishable_key: String,
    pub webhook_secret: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: chrono::DateTime<chrono::Utc>,
    pub current_period_end: chrono::DateTime<chrono::Utc>,
    pub stripe_subscription_id: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Trialing,
    PastDue,
    Canceled,
    Unpaid,
    Incomplete,
    IncompleteExpired,
    Paused,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub organization_id: Uuid,
    pub metric: UsageMetric,
    pub value: u64,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum UsageMetric {
    #[default]
    Messages,
    StorageBytes,
    ApiCalls,
    Bots,
    Users,
    KbDocuments,
    Apps,
    EmailsPerDay,
    BuildsPerDay,
    UploadsPerHour,
}
pub struct BillingService {
    config: Arc<RwLock<ProductConfig>>,
    saas_enabled: bool,
}
impl BillingService {
    pub fn new(config: ProductConfig, saas_enabled: bool) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            saas_enabled,
        }
    }

    pub async fn load_from_file(path: &Path) -> Result<ProductConfig, BillingError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| BillingError::ConfigLoad(e.to_string()))?;

        let config: ProductConfig = serde_json::from_str(&content)
            .map_err(|e| BillingError::ConfigParse(e.to_string()))?;

        Ok(config)
    }

    pub fn is_saas_enabled(&self) -> bool {
        self.saas_enabled
    }

    pub async fn get_plan(&self, plan_id: &str) -> Option<PlanConfig> {
        let config = self.config.read().await;
        config.plans.get(plan_id).cloned()
    }

    pub async fn get_all_plans(&self) -> HashMap<String, PlanConfig> {
        let config = self.config.read().await;
        config.plans.clone()
    }

    pub async fn check_limit(
        &self,
        plan_id: &str,
        metric: UsageMetric,
        current_usage: u64,
    ) -> Result<bool, BillingError> {
        let plan = self.get_plan(plan_id).await
            .ok_or_else(|| BillingError::PlanNotFound(plan_id.to_string()))?;

        let limit = match metric {
            UsageMetric::Messages => plan.limits.messages_per_day,
            UsageMetric::StorageBytes => LimitValue::Limited(plan.limits.storage_mb.value().unwrap_or(0) * 1024 * 1024),
            UsageMetric::ApiCalls => plan.limits.api_calls_per_day,
            UsageMetric::Bots => plan.limits.bots,
            UsageMetric::Users => plan.limits.users,
            UsageMetric::KbDocuments => plan.limits.kb_documents,
            UsageMetric::Apps => plan.limits.apps,
            UsageMetric::EmailsPerDay => plan.limits.emails_per_day,
            UsageMetric::BuildsPerDay => plan.limits.builds_per_day,
            UsageMetric::UploadsPerHour => plan.limits.uploads_per_hour,
        };

        Ok(limit.check(current_usage))
    }

    pub async fn get_branding(&self) -> BrandingConfig {
        let config = self.config.read().await;
        config.branding.clone()
    }
}
#[derive(Debug, Clone)]
pub enum BillingError {
    ConfigLoad(String),
    ConfigParse(String),
    PlanNotFound(String),
    QuotaExceeded(UsageMetric),
    StripeError(String),
    SubscriptionNotFound,
    InvalidOperation(String),
}
impl std::fmt::Display for BillingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigLoad(e) => write!(f, "Failed to load billing config: {e}"),
            Self::ConfigParse(e) => write!(f, "Failed to parse billing config: {e}"),
            Self::PlanNotFound(id) => write!(f, "Plan not found: {id}"),
            Self::QuotaExceeded(metric) => write!(f, "Quota exceeded for {metric:?}"),
            Self::StripeError(e) => write!(f, "Stripe error: {e}"),
            Self::SubscriptionNotFound => write!(f, "Subscription not found"),
            Self::InvalidOperation(e) => write!(f, "Invalid operation: {e}"),
        }
    }
}
impl std::error::Error for BillingError {}
