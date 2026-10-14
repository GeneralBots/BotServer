use super::helpers::{json_req, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const BIGCOMMERCE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "bigcommerce.products.list",
        method: "GET",
        path: "/catalog/products",
        summary: "Listed BigCommerce products.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "bigcommerce.products.search",
        method: "GET",
        path: "/catalog/products",
        summary: "Searched BigCommerce products.",
        path_params: &[],
        query: &[("keyword", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "bigcommerce.orders.list",
        method: "GET",
        path: "/orders",
        summary: "Listed BigCommerce orders.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
];

const BIGCOMMERCE_KEYS: &[&str] = &[
    "bigcommerce.products.list",
    "bigcommerce.products.search",
    "bigcommerce.orders.list",
];

pub const BIGCOMMERCE_SPEC: ProviderSpec = ProviderSpec {
    slug: "bigcommerce",
    origin: Origin::FromField {
        field: "store_hash",
        pattern: "https://api.bigcommerce.com/stores/{value}/v3",
    },
    auth: AuthStyle::ApiKeyHeader {
        header: "X-Auth-Token",
        field: "token",
    },
    actions: BIGCOMMERCE_ACTIONS,
    action_keys: BIGCOMMERCE_KEYS,
};
const DHL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "dhl.shipments.track",
        method: "GET",
        path: "/track/shipments",
        summary: "Tracked DHL shipments.",
        path_params: &[],
        query: &[("trackingNumber", "tracking_number")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("tracking_number")],
    },
];

const DHL_KEYS: &[&str] = &["dhl.shipments.track"];

pub const DHL_SPEC: ProviderSpec = ProviderSpec {
    slug: "dhl",
    origin: Origin::Static("https://api-eu.dhl.com"),
    auth: AuthStyle::ApiKeyHeader {
        header: "DHL-API-Key",
        field: "api_key",
    },
    actions: DHL_ACTIONS,
    action_keys: DHL_KEYS,
};
const SQUARESPACE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "squarespace.products.list",
        method: "GET",
        path: "/1.0/commerce/products",
        summary: "Listed Squarespace products.",
        path_params: &[],
        query: &[("cursor", "cursor")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("cursor")],
    },
    ActionSpec {
        key: "squarespace.orders.list",
        method: "GET",
        path: "/1.0/commerce/orders",
        summary: "Listed Squarespace orders.",
        path_params: &[],
        query: &[("cursor", "cursor")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("cursor")],
    },
    ActionSpec {
        key: "squarespace.orders.get",
        method: "GET",
        path: "/1.0/commerce/orders/{resource_id}",
        summary: "Read a Squarespace order.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const SQUARESPACE_KEYS: &[&str] = &[
    "squarespace.products.list",
    "squarespace.orders.list",
    "squarespace.orders.get",
];

pub const SQUARESPACE_SPEC: ProviderSpec = ProviderSpec {
    slug: "squarespace",
    origin: Origin::Static("https://api.squarespace.com"),
    auth: AuthStyle::BasicJoin {
        first_field: "token",
        separator: ':',
        second_field: None,
    },
    actions: SQUARESPACE_ACTIONS,
    action_keys: SQUARESPACE_KEYS,
};
const WHOP_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "whop.products.list",
        method: "GET",
        path: "/products",
        summary: "Listed Whop products.",
        path_params: &[],
        query: &[("per", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "whop.products.search",
        method: "GET",
        path: "/products",
        summary: "Searched Whop products.",
        path_params: &[],
        query: &[("search", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "whop.orders.list",
        method: "GET",
        path: "/orders",
        summary: "Listed Whop orders.",
        path_params: &[],
        query: &[("per", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "whop.products.create",
        method: "POST",
        path: "/products",
        summary: "Created a Whop product.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json_req("data")],
    },
    ActionSpec {
        key: "whop.products.update",
        method: "PATCH",
        path: "/products/{resource_id}",
        summary: "Updated a Whop product.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("resource_id"), json_req("data")],
    },
    ActionSpec {
        key: "whop.products.delete",
        method: "DELETE",
        path: "/products/{resource_id}",
        summary: "Deleted a Whop product.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const WHOP_KEYS: &[&str] = &[
    "whop.products.list",
    "whop.products.search",
    "whop.orders.list",
    "whop.products.create",
    "whop.products.update",
    "whop.products.delete",
];

pub const WHOP_SPEC: ProviderSpec = ProviderSpec {
    slug: "whop",
    origin: Origin::Static("https://api.whop.com/api/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: WHOP_ACTIONS,
    action_keys: WHOP_KEYS,
};
const WOOCOMMERCE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "woocommerce.products.list",
        method: "GET",
        path: "/products",
        summary: "Listed WooCommerce products.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "woocommerce.products.search",
        method: "GET",
        path: "/products",
        summary: "Searched WooCommerce products.",
        path_params: &[],
        query: &[("search", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "woocommerce.orders.list",
        method: "GET",
        path: "/orders",
        summary: "Listed WooCommerce orders.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
];

const WOOCOMMERCE_KEYS: &[&str] = &[
    "woocommerce.products.list",
    "woocommerce.products.search",
    "woocommerce.orders.list",
];

pub const WOOCOMMERCE_SPEC: ProviderSpec = ProviderSpec {
    slug: "woocommerce",
    origin: Origin::FromField {
        field: "url",
        pattern: "{value}/wp-json/wc/v3",
    },
    auth: AuthStyle::BasicJoin {
        first_field: "consumer_key",
        separator: ':',
        second_field: Some("consumer_secret"),
    },
    actions: WOOCOMMERCE_ACTIONS,
    action_keys: WOOCOMMERCE_KEYS,
};
