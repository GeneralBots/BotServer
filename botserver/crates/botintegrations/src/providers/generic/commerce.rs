use super::helpers::{resource_id, s};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const PRINTFUL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "printful.products.list",
        method: "GET",
        path: "/sync/products",
        summary: "Listed Printful sync products.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "printful.orders.list",
        method: "GET",
        path: "/orders",
        summary: "Listed Printful orders.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "printful.orders.get",
        method: "GET",
        path: "/orders/{resource_id}",
        summary: "Read a Printful order.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const PRINTFUL_KEYS: &[&str] = &[
    "printful.products.list",
    "printful.orders.list",
    "printful.orders.get",
];

pub const PRINTFUL_SPEC: ProviderSpec = ProviderSpec {
    slug: "printful",
    origin: Origin::Static("https://api.printful.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: PRINTFUL_ACTIONS,
    action_keys: PRINTFUL_KEYS,
};
