use super::super::actions::{
    ACCOUNTING_ACTIONS, COMMERCE_ACTIONS, CONTENT_ACTIONS, CRM_ACTIONS, PAYMENT_ACTIONS,
    SHIPPING_ACTIONS,
};
use super::super::auth::{API_KEY, BASIC, OAUTH2, TOKEN};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    provider(
        "squarespace",
        "Squarespace",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.squarespace.com/commerce-apis/overview"),
            auth: &API_KEY, actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "webflow",
        "Webflow",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.webflow.com/data/reference/rest-introduction"),
            auth: &OAUTH2, actions: CONTENT_ACTIONS,
        },
    ),
    provider(
        "highlevel",
        "HighLevel",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://marketplace.gohighlevel.com/docs/"), auth: &OAUTH2,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "bigcommerce",
        "BigCommerce",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.bigcommerce.com/docs/rest-management"),
            auth: &TOKEN, actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "square",
        "Square",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.squareup.com/reference/square"), auth: &OAUTH2,
            actions: PAYMENT_ACTIONS,
        },
    ),
    provider(
        "quickbooks",
        "QuickBooks",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botbanking"),
            official_docs: Some("https://developer.intuit.com/app/developer/qbo/docs/get-started"),
            auth: &OAUTH2, actions: ACCOUNTING_ACTIONS,
        },
    ),
    provider(
        "paddle",
        "Paddle",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.paddle.com/api-reference/overview"),
            auth: &API_KEY, actions: PAYMENT_ACTIONS,
        },
    ),
    provider(
        "printful",
        "Printful",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.printful.com/docs/"), auth: &OAUTH2,
            actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "whop",
        "Whop",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://dev.whop.com/api-reference"), auth: &API_KEY,
            actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "woocommerce",
        "WooCommerce",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://woocommerce.github.io/woocommerce-rest-api-docs/"),
            auth: &BASIC, actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "xero_accounting",
        "Xero Accounting",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.xero.com/documentation/api/accounting/overview"),
            auth: &OAUTH2, actions: ACCOUNTING_ACTIONS,
        },
    ),
    provider(
        "lightspeed_x",
        "Lightspeed X",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &OAUTH2,
            actions: COMMERCE_ACTIONS,
        },
    ),
    provider(
        "fedex",
        "FedEx",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.fedex.com/api/en-us/catalog.html"),
            auth: &OAUTH2, actions: SHIPPING_ACTIONS,
        },
    ),
    provider(
        "ups",
        "UPS",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.ups.com/api/reference"), auth: &OAUTH2,
            actions: SHIPPING_ACTIONS,
        },
    ),
    provider(
        "dhl",
        "DHL",
        ProviderSpec {
            category: Category::SmallBusiness, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.dhl.com/api-reference"), auth: &API_KEY,
            actions: SHIPPING_ACTIONS,
        },
    ),
];
