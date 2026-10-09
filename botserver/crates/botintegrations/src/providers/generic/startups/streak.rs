use super::*;

// ---------------------------------------------------------------------------
// Apollo.io REST API v1 - X-Api-Key header. Verified subset of the catalog:
// contacts and opportunities CRUD via /v1/contacts and /v1/opportunities.
// ---------------------------------------------------------------------------
const STREAK_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "streak.deals.list",
        method: "GET",
        path: "/v1/pipelines/{pipeline_key}/boxes",
        summary: "Listed pipeline boxes (deals).",
        path_params: &["pipeline_key"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("pipeline_key")],
    },
    ActionSpec {
        key: "streak.deals.get",
        method: "GET",
        path: "/v1/pipelines/{pipeline_key}/boxes/{resource_id}",
        summary: "Read a pipeline box.",
        path_params: &["pipeline_key", "resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("pipeline_key"), s_req("box_key")],
    },
];

const STREAK_KEYS: &[&str] = &["streak.deals.list", "streak.deals.get"];

pub const STREAK_SPEC: ProviderSpec = ProviderSpec {
    slug: "streak",
    origin: Origin::Static("https://www.streak.com/api"),
    auth: AuthStyle::BasicJoin {
        first_field: "api_key",
        separator: ':',
        second_field: None,
    },
    actions: STREAK_ACTIONS,
    action_keys: STREAK_KEYS,
};
