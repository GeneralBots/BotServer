    use super::*;

    pub(super) fn test_product_config() -> ProductConfig {
        default_product_config()
    }

    #[test]
    pub(super) fn test_limit_value_limited() {
        let limit = LimitValue::Limited(100);
        assert!(!limit.is_unlimited());
        assert_eq!(limit.value(), Some(100));
        assert!(limit.check(50));
        assert!(limit.check(99));
        assert!(!limit.check(100));
        assert!(!limit.check(101));
    }

    #[test]
    pub(super) fn test_limit_value_unlimited() {
        let limit = LimitValue::Unlimited;
        assert!(limit.is_unlimited());
        assert_eq!(limit.value(), None);
        assert!(limit.check(0));
        assert!(limit.check(1_000_000));
        assert!(limit.check(u64::MAX));
    }

    #[test]
    pub(super) fn test_limit_value_default() {
        let limit = LimitValue::default();
        assert_eq!(limit.value(), Some(0));
        assert!(!limit.check(0));
        assert!(!limit.check(1));
    }

    #[test]
    pub(super) fn test_default_product_config_has_required_plans() {
        let config = test_product_config();
        assert!(config.plans.contains_key("free"));
        // personal/business/enterprise removed — legacy cleanup
        assert!(config.plans.contains_key("shared")); // placeholder
        assert!(config.plans.contains_key("private-cloud")); // placeholder
    }

    #[test]
    pub(super) fn test_free_plan_limits() {
        let config = test_product_config();
        let free = config.plans.get("free").unwrap();

        assert_eq!(free.name, "Free");
        assert!(matches!(free.price, PlanPrice::Free));

        assert_eq!(free.limits.messages_per_day.value(), Some(10));
        assert_eq!(free.limits.storage_mb.value(), Some(20));
        assert_eq!(free.limits.bots.value(), Some(1));
        assert_eq!(free.limits.users.value(), Some(1));
    }





    #[test]
    pub(super) fn test_shared_plan_has_trial() {
        let config = test_product_config();
        let shared = config.plans.get("shared").unwrap();
        assert_eq!(shared.trial_days, Some(14));
    }

    #[test]
    pub(super) fn test_free_plan_no_trial() {
        let config = test_product_config();
        let free = config.plans.get("free").unwrap();
        assert_eq!(free.trial_days, None);
    }

    #[test]
    pub(super) fn test_branding_config() {
        let config = test_product_config();
        assert_eq!(config.branding.name, "General Bots");
        assert_eq!(config.branding.primary_color, "#1976d2");
    }

    #[test]
    pub(super) fn test_features_list() {
        let config = test_product_config();
        assert!(config.features.contains(&"basic_chat".to_string()));
        assert!(config.features.contains(&"api_access".to_string()));
        assert!(config.features.contains(&"sso_saml".to_string()));
    }

    #[test]
    pub(super) fn test_private_cloud_has_features() {
        let config = test_product_config();
        let pc = config.plans.get("private-cloud").unwrap();

        assert!(pc.features.contains(&"basic_chat".to_string()));
        assert!(pc.features.contains(&"sso_saml".to_string()));
        assert!(pc.features.contains(&"audit_logs".to_string()));
        assert!(pc.features.contains(&"on_premise".to_string()));
    }

    #[tokio::test]
    async fn test_billing_service_get_plan() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let free = service.get_plan("free").await;
        assert!(free.is_some());
        assert_eq!(free.unwrap().name, "Free");

        let nonexistent = service.get_plan("nonexistent").await;
        assert!(nonexistent.is_none());
    }

    #[tokio::test]
    async fn test_billing_service_get_all_plans() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let plans = service.get_all_plans().await;
        assert_eq!(plans.len(), 3);
        assert!(plans.contains_key("free"));
        assert!(plans.contains_key("shared"));
        assert!(plans.contains_key("private-cloud"));
    }

    #[tokio::test]
    async fn test_billing_service_check_limit_within_quota() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let result = service
            .check_limit("free", UsageMetric::Messages, 3)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_billing_service_check_limit_exceeded() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let result = service
            .check_limit("free", UsageMetric::Messages, 10)
            .await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_billing_service_check_limit_unlimited() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let result = service
            .check_limit("private-cloud", UsageMetric::Messages, 1_000_000)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_billing_service_check_limit_plan_not_found() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let result = service
            .check_limit("nonexistent", UsageMetric::Messages, 1)
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), BillingError::PlanNotFound(_)));
    }

    #[tokio::test]
    async fn test_billing_service_get_branding() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let branding = service.get_branding().await;
        assert_eq!(branding.name, "General Bots");
    }

    #[test]
    pub(super) fn test_billing_service_saas_enabled() {
        let config = test_product_config();

        let service_enabled = BillingService::new(config.clone(), true);
        assert!(service_enabled.is_saas_enabled());

        let service_disabled = BillingService::new(config, false);
        assert!(!service_disabled.is_saas_enabled());
    }

    #[test]
    pub(super) fn test_subscription_status_variants() {
        let statuses = vec![
            SubscriptionStatus::Active,
            SubscriptionStatus::Trialing,
            SubscriptionStatus::PastDue,
            SubscriptionStatus::Canceled,
            SubscriptionStatus::Unpaid,
            SubscriptionStatus::Incomplete,
            SubscriptionStatus::IncompleteExpired,
            SubscriptionStatus::Paused,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            let deserialized: SubscriptionStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    pub(super) fn test_usage_metric_variants() {
        let metrics = vec![
            UsageMetric::Messages,
            UsageMetric::StorageBytes,
            UsageMetric::ApiCalls,
            UsageMetric::Bots,
            UsageMetric::Users,
            UsageMetric::KbDocuments,
            UsageMetric::Apps,
            UsageMetric::EmailsPerDay,
            UsageMetric::BuildsPerDay,
            UsageMetric::UploadsPerHour,
        ];

        for metric in metrics {
            let serialized = serde_json::to_string(&metric).unwrap();
            let deserialized: UsageMetric = serde_json::from_str(&serialized).unwrap();
            assert_eq!(metric, deserialized);
        }
    }

    #[test]
    pub(super) fn test_billing_period_equality() {
        assert_eq!(BillingPeriod::Monthly, BillingPeriod::Monthly);
        assert_eq!(BillingPeriod::Yearly, BillingPeriod::Yearly);
        assert_ne!(BillingPeriod::Monthly, BillingPeriod::Yearly);
    }

    #[test]
    pub(super) fn test_billing_error_display() {
        let errors = vec![
            (BillingError::ConfigLoad("test".to_string()), "Failed to load billing config: test"),
            (BillingError::ConfigParse("test".to_string()), "Failed to parse billing config: test"),
            (BillingError::PlanNotFound("test".to_string()), "Plan not found: test"),
            (BillingError::QuotaExceeded(UsageMetric::Messages), "Quota exceeded for Messages"),
            (BillingError::StripeError("test".to_string()), "Stripe error: test"),
            (BillingError::SubscriptionNotFound, "Subscription not found"),
            (BillingError::InvalidOperation("test".to_string()), "Invalid operation: test"),
        ];

        for (error, expected_msg) in errors {
            assert_eq!(error.to_string(), expected_msg);
        }
    }

    #[tokio::test]
    async fn test_billing_service_storage_limit_conversion() {
        let config = test_product_config();
        let service = BillingService::new(config, true);

        let result = service
            .check_limit("free", UsageMetric::StorageBytes, 10 * 1024 * 1024)
            .await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        let result_exceeded = service
            .check_limit("free", UsageMetric::StorageBytes, 21 * 1024 * 1024)
            .await;
        assert!(result_exceeded.is_ok());
        assert!(!result_exceeded.unwrap());
    }

    #[test]
    pub(super) fn test_plan_price_serialization() {
        let free = PlanPrice::Free;
        let free_json = serde_json::to_string(&free).unwrap();
        assert_eq!(free_json, "null", "untagged unit variants serialize as null");

        let fixed = PlanPrice::Fixed {
            amount: 1000,
            currency: "usd".to_string(),
            period: BillingPeriod::Monthly,
        };
        let fixed_json = serde_json::to_string(&fixed).unwrap();
        assert!(fixed_json.contains("1000"));
        assert!(fixed_json.contains("usd"));

        let custom = PlanPrice::Custom;
        let custom_json = serde_json::to_string(&custom).unwrap();
        assert_eq!(custom_json, "null", "untagged unit variants serialize as null");
    }

    #[test]
    pub(super) fn test_plan_limits_all_metrics() {
        let config = test_product_config();
        let shared = config.plans.get("shared").unwrap();

        assert!(shared.limits.messages_per_day.is_unlimited());
        assert!(shared.limits.storage_mb.value().is_some());
        assert!(shared.limits.bots.value().is_some());
        assert!(shared.limits.users.value().is_some());
        assert!(shared.limits.api_calls_per_day.is_unlimited());
        assert!(shared.limits.kb_documents.value().is_some());
        assert!(shared.limits.apps.value().is_some());
        assert!(shared.limits.emails_per_day.value().is_some());
        assert!(shared.limits.builds_per_day.value().is_some());
        assert!(shared.limits.uploads_per_hour.value().is_some());
    }

    #[test]
    pub(super) fn test_subscription_struct() {
        let now = chrono::Utc::now();
        let subscription = Subscription {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            plan_id: "business".to_string(),
            status: SubscriptionStatus::Active,
            current_period_start: now,
            current_period_end: now + chrono::Duration::days(30),
            stripe_subscription_id: Some("sub_123".to_string()),
            stripe_customer_id: Some("cus_123".to_string()),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(subscription.plan_id, "business");
        assert_eq!(subscription.status, SubscriptionStatus::Active);
        assert!(subscription.stripe_subscription_id.is_some());
    }

    #[test]
    pub(super) fn test_usage_record_struct() {
        let now = chrono::Utc::now();
        let record = UsageRecord {
            organization_id: Uuid::new_v4(),
            metric: UsageMetric::Messages,
            value: 100,
            period_start: now,
            period_end: now + chrono::Duration::days(1),
        };

        assert_eq!(record.metric, UsageMetric::Messages);
        assert_eq!(record.value, 100);
    }
