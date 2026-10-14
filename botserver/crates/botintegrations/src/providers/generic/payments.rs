use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const SQUARE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "square.payments.list",
        method: "POST",
        path: "/v2/payments/list",
        summary: "Listed Square payments.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", false)],
    },
    ActionSpec {
        key: "square.payments.get",
        method: "GET",
        path: "/v2/payments/{resource_id}",
        summary: "Read a Square payment.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "square.customers.search",
        method: "POST",
        path: "/v2/customers/search",
        summary: "Searched Square customers.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "square.payments.create",
        method: "POST",
        path: "/v2/payments",
        summary: "Created a Square payment.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "square.payments.refund",
        method: "POST",
        path: "/v2/refunds",
        summary: "Refunded a Square payment.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
];

const SQUARE_KEYS: &[&str] = &[
    "square.payments.list",
    "square.payments.get",
    "square.customers.search",
    "square.payments.create",
    "square.payments.refund",
];

pub const SQUARE_SPEC: ProviderSpec = ProviderSpec {
    slug: "square",
    origin: Origin::Static("https://connect.squareup.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: SQUARE_ACTIONS,
    action_keys: SQUARE_KEYS,
};

const PAYPAL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "paypal.payments.list",
        method: "GET",
        path: "/v1/reporting/transactions",
        summary: "Listed PayPal transactions.",
        path_params: &[],
        query: &[("start_date", "start_date"), ("end_date", "end_date")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("start_date"), s_req("end_date")],
    },
    ActionSpec {
        key: "paypal.payments.get",
        method: "GET",
        path: "/v2/checkout/orders/{resource_id}",
        summary: "Read a PayPal order.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "paypal.payments.refund",
        method: "POST",
        path: "/v2/payments/captures/{resource_id}/refund",
        summary: "Refunded a PayPal capture.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("capture_id"), json("data", false)],
    },
];

const PAYPAL_KEYS: &[&str] = &[
    "paypal.payments.list",
    "paypal.payments.get",
    "paypal.payments.refund",
];

pub const PAYPAL_SPEC: ProviderSpec = ProviderSpec {
    slug: "paypal",
    origin: Origin::Static("https://api-m.paypal.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: PAYPAL_ACTIONS,
    action_keys: PAYPAL_KEYS,
};

const XERO_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "xero_accounting.invoices.list",
        method: "GET",
        path: "/2.0/Invoices",
        summary: "Listed Xero invoices.",
        path_params: &[],
        query: &[("where", "where")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("where")],
    },
    ActionSpec {
        key: "xero_accounting.invoices.create",
        method: "POST",
        path: "/2.0/Invoices",
        summary: "Created a Xero invoice.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "xero_accounting.invoices.update",
        method: "POST",
        path: "/2.0/Invoices/{resource_id}",
        summary: "Updated a Xero invoice.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("invoice_id"), json("data", true)],
    },
];

const XERO_KEYS: &[&str] = &[
    "xero_accounting.invoices.list",
    "xero_accounting.invoices.create",
    "xero_accounting.invoices.update",
];

pub const XERO_ACCOUNTING_SPEC: ProviderSpec = ProviderSpec {
    slug: "xero_accounting",
    origin: Origin::Static("https://api.xero.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: XERO_ACTIONS,
    action_keys: XERO_KEYS,
};

const YNAB_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "ynab.transactions.list",
        method: "GET",
        path: "/budgets/{budget_id}/transactions",
        summary: "Listed YNAB transactions.",
        path_params: &["budget_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("budget_id")],
    },
    ActionSpec {
        key: "ynab.expenses.create",
        method: "POST",
        path: "/budgets/{budget_id}/transactions",
        summary: "Created a YNAB transaction.",
        path_params: &["budget_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("budget_id"), json("data", true)],
    },
    ActionSpec {
        key: "ynab.transactions.search",
        method: "GET",
        path: "/budgets/{budget_id}/transactions/{resource_id}",
        summary: "Read a YNAB transaction.",
        path_params: &["budget_id", "transaction_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("budget_id"), s_req("transaction_id")],
    },
];

const YNAB_KEYS: &[&str] = &[
    "ynab.transactions.list",
    "ynab.expenses.create",
    "ynab.transactions.search",
];

pub const YNAB_SPEC: ProviderSpec = ProviderSpec {
    slug: "ynab",
    origin: Origin::Static("https://api.youneedabudget.com/v1"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: YNAB_ACTIONS,
    action_keys: YNAB_KEYS,
};

const RAMP_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "ramp.transactions.list",
        method: "GET",
        path: "/developer/v1/transactions",
        summary: "Listed Ramp transactions.",
        path_params: &[],
        query: &[("page_size", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "ramp.transactions.get",
        method: "GET",
        path: "/developer/v1/transactions/{resource_id}",
        summary: "Read a Ramp transaction.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const RAMP_KEYS: &[&str] = &["ramp.transactions.list", "ramp.transactions.get"];

pub const RAMP_SPEC: ProviderSpec = ProviderSpec {
    slug: "ramp",
    origin: Origin::Static("https://gateway.rampapp.gg/api"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: RAMP_ACTIONS,
    action_keys: RAMP_KEYS,
};
