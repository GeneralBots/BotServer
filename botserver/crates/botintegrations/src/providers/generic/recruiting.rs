use super::helpers::{resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const GREENHOUSE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "greenhouse.candidates.list",
        method: "GET",
        path: "/v1/candidates",
        summary: "Listed Greenhouse candidates.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "greenhouse.candidates.get",
        method: "GET",
        path: "/v1/candidates/{resource_id}",
        summary: "Read a Greenhouse candidate.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const GREENHOUSE_KEYS: &[&str] = &[
    "greenhouse.candidates.list",
    "greenhouse.candidates.get",
];

pub const GREENHOUSE_SPEC: ProviderSpec = ProviderSpec {
    slug: "greenhouse",
    origin: Origin::Static("https://harvest.greenhouse.io"),
    auth: AuthStyle::BasicJoin {
        first_field: "api_key",
        separator: ':',
        second_field: None,
    },
    actions: GREENHOUSE_ACTIONS,
    action_keys: GREENHOUSE_KEYS,
};

const CRUNCHBASE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "crunchbase.contacts.search",
        method: "GET",
        path: "/data/entities/organizations",
        summary: "Searched Crunchbase organizations.",
        path_params: &[],
        query: &[("q", "query"), ("field_ids", "fields")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("fields")],
    },
    ActionSpec {
        key: "crunchbase.deals.list",
        method: "GET",
        path: "/data/entities/funding_rounds",
        summary: "Listed Crunchbase funding rounds.",
        path_params: &[],
        query: &[("field_ids", "fields")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("fields")],
    },
];

const CRUNCHBASE_KEYS: &[&str] = &[
    "crunchbase.contacts.search",
    "crunchbase.deals.list",
];

pub const CRUNCHBASE_SPEC: ProviderSpec = ProviderSpec {
    slug: "crunchbase",
    origin: Origin::Static("https://api.crunchbase.com/api/v4"),
    auth: AuthStyle::QueryPairs {
        pairs: &[("user_key", "user_key")],
    },
    actions: CRUNCHBASE_ACTIONS,
    action_keys: CRUNCHBASE_KEYS,
};
