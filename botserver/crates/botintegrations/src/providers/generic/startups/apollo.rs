use super::*;

const APOLLO_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "apollo.contacts.list",
        method: "GET",
        path: "/v1/contacts",
        summary: "Listed Apollo contacts.",
        path_params: &[],
        query: &[("page", "page")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("page")],
    },
    ActionSpec {
        key: "apollo.contacts.search",
        method: "GET",
        path: "/v1/contacts/search",
        summary: "Searched Apollo contacts.",
        path_params: &[],
        query: &[("q_keywords", "query"), ("page", "page")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("page")],
    },
    ActionSpec {
        key: "apollo.deals.list",
        method: "POST",
        path: "/v1/opportunities/search",
        summary: "Listed Apollo opportunities.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[ParamSpec {
            name: "data",
            kind: ParamKind::Json,
            required: false,
        }],
    },
    ActionSpec {
        key: "apollo.contacts.create",
        method: "POST",
        path: "/v1/contacts",
        summary: "Created an Apollo contact.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[ParamSpec {
            name: "data",
            kind: ParamKind::Json,
            required: true,
        }],
    },
    ActionSpec {
        key: "apollo.contacts.update",
        method: "PUT",
        path: "/v1/contacts/{resource_id}",
        summary: "Updated an Apollo contact.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("resource_id"),
            ParamSpec {
                name: "data",
                kind: ParamKind::Json,
                required: true,
            }],
    },
];


const APOLLO_KEYS: &[&str] = &[
    "apollo.contacts.list",
    "apollo.contacts.search",
    "apollo.deals.list",
    "apollo.contacts.create",
    "apollo.contacts.update",
];

pub const APOLLO_SPEC: ProviderSpec = ProviderSpec {
    slug: "apollo",
    origin: Origin::Static("https://api.apollo.io"),
    auth: AuthStyle::ApiKeyHeader {
        header: "X-Api-Key",
        field: "api_key",
    },
    actions: APOLLO_ACTIONS,
    action_keys: APOLLO_KEYS,
};
