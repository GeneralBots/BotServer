use super::super::actions::{
    BANKING_ACTIONS, CRYPTO_ACTIONS, FINANCE_OPERATIONS_ACTIONS, PAYMENT_ACTIONS, STRIPE_ACTIONS,
};
use super::super::auth::{API_KEY, OAUTH2, TOKEN, UNKNOWN, UNSUPPORTED};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    // Stripe is a live adapter (#950 slice 2); the seed flips
    // llm_available and points at the concrete STRIPE_ACTIONS profile while
    // `action_is_implemented` keeps the per-action truth in sync with
    // botintegrations::providers::registry.
    ProviderSeed {
        llm_available: true,
        ..provider(
            "stripe",
            "Stripe",
            ProviderSpec {
                category: Category::Finance, strategy: Strategy::Improve, status: Status::Built,
                priority: Priority::Must, module: Some("botbilling"),
                official_docs: Some("https://docs.stripe.com/api"), auth: &API_KEY,
                actions: STRIPE_ACTIONS,
            },
        )
    },
    provider(
        "paypal",
        "PayPal",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.paypal.com/api/rest/"), auth: &OAUTH2,
            actions: PAYMENT_ACTIONS,
        },
    ),
    provider(
        "coinbase",
        "Coinbase",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.cdp.coinbase.com/api-reference/v2/rest-api/introduction"),
            auth: &API_KEY, actions: CRYPTO_ACTIONS,
        },
    ),
    provider(
        "wise",
        "Wise",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.wise.com/api-docs/api-reference"), auth: &TOKEN,
            actions: BANKING_ACTIONS,
        },
    ),
    provider(
        "ynab",
        "You Need a Budget",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: Some("https://api.ynab.com/"),
            auth: &OAUTH2, actions: FINANCE_OPERATIONS_ACTIONS,
        },
    ),
    provider(
        "moonpay",
        "MoonPay",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://dev.moonpay.com/reference/overview"), auth: &API_KEY,
            actions: CRYPTO_ACTIONS,
        },
    ),
    provider(
        "qik",
        "Qik",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Verify, status: Status::Unsupported,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNSUPPORTED,
            actions: PAYMENT_ACTIONS,
        },
    ),
    provider(
        "robinhood",
        "Robinhood",
        ProviderSpec {
            category: Category::Finance, strategy: Strategy::Verify, status: Status::Unsupported,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: CRYPTO_ACTIONS,
        },
    ),
];
