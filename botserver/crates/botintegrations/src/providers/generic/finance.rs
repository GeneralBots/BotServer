use super::helpers::{json, json_req, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const COINBASE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "coinbase.accounts.list",
        method: "GET",
        path: "/v2/accounts",
        summary: "Listed Coinbase accounts.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "coinbase.transactions.list",
        method: "GET",
        path: "/v2/accounts/{account_id}/transactions",
        summary: "Listed account transactions.",
        path_params: &["account_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id"), s("limit")],
    },
    ActionSpec {
        key: "coinbase.quotes.get",
        method: "GET",
        path: "/v2/prices/{currency_pair}/spot",
        summary: "Read spot price quote.",
        path_params: &["currency_pair"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("currency_pair")],
    },
];

const COINBASE_KEYS: &[&str] = &[
    "coinbase.accounts.list",
    "coinbase.transactions.list",
    "coinbase.quotes.get",
];

pub const COINBASE_SPEC: ProviderSpec = ProviderSpec {
    slug: "coinbase",
    origin: Origin::Static("https://api.coinbase.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: COINBASE_ACTIONS,
    action_keys: COINBASE_KEYS,
};
const MERCURY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "mercury.accounts.list",
        method: "GET",
        path: "/api/v1/accounts",
        summary: "Listed Mercury accounts.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "mercury.transactions.list",
        method: "GET",
        path: "/api/v1/account/{account_id}/transactions",
        summary: "Listed Mercury transactions.",
        path_params: &["account_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id"), s("limit")],
    },
    ActionSpec {
        key: "mercury.transactions.search",
        method: "GET",
        path: "/api/v1/account/{account_id}/transactions",
        summary: "Searched Mercury transactions.",
        path_params: &["account_id"],
        query: &[("search", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id"), s_req("query")],
    },
    ActionSpec {
        key: "mercury.balances.get",
        method: "GET",
        path: "/api/v1/account/{account_id}",
        summary: "Read Mercury account balance.",
        path_params: &["account_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id")],
    },
    ActionSpec {
        key: "mercury.transfers.create",
        method: "POST",
        path: "/api/v1/transfers",
        summary: "Created a Mercury transfer.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json_req("data")],
    },
];

const MERCURY_KEYS: &[&str] = &[
    "mercury.accounts.list",
    "mercury.transactions.list",
    "mercury.transactions.search",
    "mercury.balances.get",
    "mercury.transfers.create",
];

pub const MERCURY_SPEC: ProviderSpec = ProviderSpec {
    slug: "mercury",
    origin: Origin::Static("https://api.mercury.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: MERCURY_ACTIONS,
    action_keys: MERCURY_KEYS,
};

// ---------------------------------------------------------------------------
// Whop REST API v2 - Bearer token.
// ---------------------------------------------------------------------------
const MOONPAY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "moonpay.accounts.list",
        method: "GET",
        path: "/v1/accounts/me",
        summary: "Read the MoonPay account.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "moonpay.transactions.list",
        method: "GET",
        path: "/v1/transactions",
        summary: "Listed MoonPay transactions.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "moonpay.quotes.get",
        method: "GET",
        path: "/v3/currencies/{currency_code}/quote",
        summary: "Read a buy quote.",
        path_params: &["currency_code"],
        query: &[("baseCurrencyAmount", "amount")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("currency_code"), s("amount")],
    },
];

const MOONPAY_KEYS: &[&str] = &[
    "moonpay.accounts.list",
    "moonpay.transactions.list",
    "moonpay.quotes.get",
];

pub const MOONPAY_SPEC: ProviderSpec = ProviderSpec {
    slug: "moonpay",
    origin: Origin::Static("https://api.moonpay.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: MOONPAY_ACTIONS,
    action_keys: MOONPAY_KEYS,
};
const PADDLE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "paddle.payments.list",
        method: "GET",
        path: "/billing/transactions",
        summary: "Listed Paddle transactions.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "paddle.payments.get",
        method: "GET",
        path: "/billing/transactions/{resource_id}",
        summary: "Read a Paddle transaction.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "paddle.customers.search",
        method: "GET",
        path: "/billing/customers",
        summary: "Searched Paddle customers.",
        path_params: &[],
        query: &[("email", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
];

const PADDLE_KEYS: &[&str] = &[
    "paddle.payments.list",
    "paddle.payments.get",
    "paddle.customers.search",
];

pub const PADDLE_SPEC: ProviderSpec = ProviderSpec {
    slug: "paddle",
    origin: Origin::Static("https://api.paddle.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: PADDLE_ACTIONS,
    action_keys: PADDLE_KEYS,
};
const WISE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "wise.accounts.list",
        method: "GET",
        path: "/v1/profiles/{profile_id}",
        summary: "Read Wise profile and accounts.",
        path_params: &["profile_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("profile_id")],
    },
    ActionSpec {
        key: "wise.balances.get",
        method: "GET",
        path: "/v4/profiles/{profile_id}/balances",
        summary: "Listed Wise balances.",
        path_params: &["profile_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("profile_id")],
    },
    ActionSpec {
        key: "wise.transfers.create",
        method: "POST",
        path: "/v1/transfers",
        summary: "Created a Wise transfer.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
];

const WISE_KEYS: &[&str] = &[
    "wise.accounts.list",
    "wise.balances.get",
    "wise.transfers.create",
];

pub const WISE_SPEC: ProviderSpec = ProviderSpec {
    slug: "wise",
    origin: Origin::Static("https://api.wise.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: WISE_ACTIONS,
    action_keys: WISE_KEYS,
};

const QUICKBOOKS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "quickbooks.invoices.search",
        method: "POST",
        path: "/v3/company/{realm_id}/query",
        summary: "Queried QuickBooks invoices with SQL.",
        path_params: &["realm_id"],
        query: &[("query", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("realm_id"), s_req("query")],
    },
    ActionSpec {
        key: "quickbooks.reports.get",
        method: "GET",
        path: "/v3/company/{realm_id}/companyinfo/{realm_id}",
        summary: "Read QuickBooks company information.",
        path_params: &["realm_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("realm_id")],
    },
];

const QUICKBOOKS_KEYS: &[&str] = &[
    "quickbooks.invoices.search",
    "quickbooks.reports.get",
];

pub const QUICKBOOKS_SPEC: ProviderSpec = ProviderSpec {
    slug: "quickbooks",
    origin: Origin::Static("https://sandbox-quickbooks.api.intuit.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: QUICKBOOKS_ACTIONS,
    action_keys: QUICKBOOKS_KEYS,
};
