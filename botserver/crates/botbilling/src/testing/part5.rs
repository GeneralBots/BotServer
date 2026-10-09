use super::*;

pub fn create_standard_test_scenarios() -> Vec<BillingTestScenario> {
    vec![
        BillingTestScenario {
            name: "New Customer Signup".to_string(),
            description: "Test new customer creating account and subscribing".to_string(),
            steps: vec![
                TestStep {
                    action: TestAction::CreateCustomer {
                        email: "test@example.com".to_string(),
                        name: Some("Test User".to_string()),
                    },
                    expected_result: ExpectedResult::Success,
                    delay_ms: None,
                },
                TestStep {
                    action: TestAction::CreateSubscription {
                        plan: "starter".to_string(),
                        trial_days: Some(14),
                    },
                    expected_result: ExpectedResult::SubscriptionStatus(SubscriptionStatus::Trialing),
                    delay_ms: None,
                },
            ],
        },
        BillingTestScenario {
            name: "Plan Upgrade".to_string(),
            description: "Test customer upgrading from starter to pro".to_string(),
            steps: vec![
                TestStep {
                    action: TestAction::CreateCustomer {
                        email: "upgrade@example.com".to_string(),
                        name: None,
                    },
                    expected_result: ExpectedResult::Success,
                    delay_ms: None,
                },
                TestStep {
                    action: TestAction::CreateSubscription {
                        plan: "starter".to_string(),
                        trial_days: None,
                    },
                    expected_result: ExpectedResult::SubscriptionStatus(SubscriptionStatus::Active),
                    delay_ms: None,
                },
                TestStep {
                    action: TestAction::UpgradeSubscription {
                        new_plan: "pro".to_string(),
                    },
                    expected_result: ExpectedResult::Success,
                    delay_ms: Some(100),
                },
            ],
        },
        BillingTestScenario {
            name: "Payment Failure Recovery".to_string(),
            description: "Test handling payment failure and recovery".to_string(),
            steps: vec![
                TestStep {
                    action: TestAction::CreateCustomer {
                        email: "failure@example.com".to_string(),
                        name: None,
                    },
                    expected_result: ExpectedResult::Success,
                    delay_ms: None,
                },
                TestStep {
                    action: TestAction::CreateSubscription {
                        plan: "pro".to_string(),
                        trial_days: None,
                    },
                    expected_result: ExpectedResult::SubscriptionStatus(SubscriptionStatus::Active),
                    delay_ms: None,
                },
                TestStep {
                    action: TestAction::SimulatePaymentFailure,
                    expected_result: ExpectedResult::SubscriptionStatus(SubscriptionStatus::PastDue),
                    delay_ms: Some(50),
                },
                TestStep {
                    action: TestAction::SimulatePaymentSuccess,
                    expected_result: ExpectedResult::SubscriptionStatus(SubscriptionStatus::Active),
                    delay_ms: Some(50),
                },
            ],
        },
    ]
}
