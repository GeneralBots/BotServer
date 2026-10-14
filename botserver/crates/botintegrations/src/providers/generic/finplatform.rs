//! Finance-platform adapters: Snowflake (#967), Robinhood (#989),
//! Expensify (#1061), LinkedIn Ads (#1045), Google Ads (#1062) and
//! Lightspeed X-Series (#1030). Read-mostly surfaces with conservative
//! action sets; write actions are flagged High risk.

use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ParamKind, ParamSpec, ProviderSpec, Risk};

// ── Snowflake ────────────────────────────────────────────────────

const SNOWFLAKE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "snowflake.query.exec",
        method: "POST",
        path: "/api/v2/statements",
        summary: "Executed a SQL statement.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "snowflake.warehouses.list",
        method: "GET",
        path: "/api/v2/warehouses",
        summary: "Listed virtual warehouses.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
];

const SNOWFLAKE_KEYS: &[&str] = &[
    "snowflake.query.exec",
    "snowflake.warehouses.list",
];

pub const SNOWFLAKE_SPEC: ProviderSpec = ProviderSpec {
    slug: "snowflake",
    origin: Origin::Static("https://acct.example.snowflakecomputing.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: SNOWFLAKE_ACTIONS,
    action_keys: SNOWFLAKE_KEYS,
};

// ── Robinhood (official crypto API) ──────────────────────────────

const ROBINHOOD_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "robinhood.holdings.list",
        method: "GET",
        path: "/crypto/holdings/",
        summary: "Listed crypto holdings.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "robinhood.orders.list",
        method: "GET",
        path: "/crypto/orders/",
        summary: "Listed crypto orders.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
];

const ROBINHOOD_KEYS: &[&str] = &[
    "robinhood.holdings.list",
    "robinhood.orders.list",
];

pub const ROBINHOOD_SPEC: ProviderSpec = ProviderSpec {
    slug: "robinhood",
    origin: Origin::Static("https://api.robinhood.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: ROBINHOOD_ACTIONS,
    action_keys: ROBINHOOD_KEYS,
};

// ── Expensify (Integration Server) ───────────────────────────────

const EXPENSIFY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "expensify.policies.list",
        method: "POST",
        path: "/ExpensifyIntegrations",
        summary: "Listed export policies.",
        path_params: &[],
        query: &[],
        body_param: Some("requestJobDescription"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("requestJobDescription", true)],
    },
    ActionSpec {
        key: "expensify.reports.export",
        method: "POST",
        path: "/ExpensifyIntegrations",
        summary: "Exported reports as a file download.",
        path_params: &[],
        query: &[],
        body_param: Some("requestJobDescription"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("requestJobDescription", true)],
    },
];

const EXPENSIFY_KEYS: &[&str] = &[
    "expensify.policies.list",
    "expensify.reports.export",
];

pub const EXPENSIFY_SPEC: ProviderSpec = ProviderSpec {
    slug: "expensify",
    origin: Origin::Static("https://integrations.expensify.com/Integration-Server"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: EXPENSIFY_ACTIONS,
    action_keys: EXPENSIFY_KEYS,
};

// ── LinkedIn Ads ─────────────────────────────────────────────────

const LINKEDIN_ADS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "linkedin_ads.adaccounts.list",
        method: "GET",
        path: "/rest/adAccounts",
        summary: "Listed ad accounts.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "linkedin_ads.analytics.fetch",
        method: "GET",
        path: "/rest/adAnalytics",
        summary: "Fetched ad analytics.",
        path_params: &[],
        query: &[
            ("q", "q"),
            ("dateRange.start.day", "start_day"),
            ("dateRange.end.day", "end_day"),
        ],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("q"), s("start_day"), s("end_day")],
    },
    ActionSpec {
        key: "linkedin_ads.creatives.list",
        method: "GET",
        path: "/rest/adCreatives",
        summary: "Listed creatives of an ad account.",
        path_params: &[],
        query: &[("adAccountId", "ad_account_id")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("ad_account_id")],
    },
];

const LINKEDIN_ADS_KEYS: &[&str] = &[
    "linkedin_ads.adaccounts.list",
    "linkedin_ads.analytics.fetch",
    "linkedin_ads.creatives.list",
];

pub const LINKEDIN_ADS_SPEC: ProviderSpec = ProviderSpec {
    slug: "linkedin_ads",
    origin: Origin::Static("https://api.linkedin.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: LINKEDIN_ADS_ACTIONS,
    action_keys: LINKEDIN_ADS_KEYS,
};

// ── Google Ads ───────────────────────────────────────────────────

const GOOGLE_ADS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "google_ads.customers.list_accessible",
        method: "GET",
        path: "/v16/customers:listAccessibleCustomers",
        summary: "Listed accessible Google Ads customers.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "google_ads.query.search_stream",
        method: "POST",
        path: "/v16/customers/{resource_id}/googleAds:searchStream",
        summary: "Ran a GAQL reporting query.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[
            ParamSpec {
                name: "resource_id",
                kind: ParamKind::Str,
                required: true,
            },
            json("data", true),
        ],
    },
];

const GOOGLE_ADS_KEYS: &[&str] = &[
    "google_ads.customers.list_accessible",
    "google_ads.query.search_stream",
];

pub const GOOGLE_ADS_SPEC: ProviderSpec = ProviderSpec {
    slug: "google_ads",
    origin: Origin::Static("https://googleads.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GOOGLE_ADS_ACTIONS,
    action_keys: GOOGLE_ADS_KEYS,
};

// ── Lightspeed X-Series ──────────────────────────────────────────

const LIGHTSPEED_X_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "lightspeed_x.items.list",
        method: "GET",
        path: "/API/V3/Account/{resource_id}/Item.json",
        summary: "Listed catalog items.",
        path_params: &["resource_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[
            ParamSpec {
                name: "resource_id",
                kind: ParamKind::Str,
                required: true,
            },
            s("limit"),
        ],
    },
    ActionSpec {
        key: "lightspeed_x.sales.list",
        method: "GET",
        path: "/API/V3/Account/{resource_id}/Sale.json",
        summary: "Listed sales.",
        path_params: &["resource_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[
            ParamSpec {
                name: "resource_id",
                kind: ParamKind::Str,
                required: true,
            },
            s("limit"),
        ],
    },
];

const LIGHTSPEED_X_KEYS: &[&str] = &[
    "lightspeed_x.items.list",
    "lightspeed_x.sales.list",
];

pub const LIGHTSPEED_X_SPEC: ProviderSpec = ProviderSpec {
    slug: "lightspeed_x",
    origin: Origin::Static("https://api.lightspeedapp.com"),
    auth: AuthStyle::BasicTemplate {
        user_template: "{api_key}",
        password_field: "api_secret",
    },
    actions: LIGHTSPEED_X_ACTIONS,
    action_keys: LIGHTSPEED_X_KEYS,
};

// ── Rippling (HR) ────────────────────────────────────────────────

const RIPPLING_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "rippling.employees.list",
        method: "GET",
        path: "/api/v1/employees",
        summary: "Listed employees.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "rippling.employees.get",
        method: "GET",
        path: "/api/v1/employees/{resource_id}",
        summary: "Fetched an employee profile.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "rippling.timeoffs.list",
        method: "GET",
        path: "/api/third-party/time-off-requests",
        summary: "Listed time-off requests.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
];

const RIPPLING_KEYS: &[&str] = &[
    "rippling.employees.list",
    "rippling.employees.get",
    "rippling.timeoffs.list",
];

pub const RIPPLING_SPEC: ProviderSpec = ProviderSpec {
    slug: "rippling",
    origin: Origin::Static("https://api.rippling.com"),
    auth: AuthStyle::BasicTemplate {
        user_template: "{api_token}",
        password_field: "password",
    },
    actions: RIPPLING_ACTIONS,
    action_keys: RIPPLING_KEYS,
};
