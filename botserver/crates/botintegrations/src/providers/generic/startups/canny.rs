use super::*;

// ---------------------------------------------------------------------------
// Lemlist API v2 - Basic :{apiKey} (empty team id) or teamId:apiKey.
// Verified subset: campaign listing and per-campaign statistics.
// ---------------------------------------------------------------------------
const CANNY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "canny.tickets.list",
        method: "POST",
        path: "/posts/list",
        summary: "Listed Canny posts.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("status")],
    },
    ActionSpec {
        key: "canny.tickets.search",
        method: "POST",
        path: "/posts/list",
        summary: "Searched Canny posts.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "canny.tickets.get",
        method: "POST",
        path: "/posts/get",
        summary: "Read a Canny post.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("id")],
    },
    ActionSpec {
        key: "canny.tickets.create",
        method: "POST",
        path: "/posts/create",
        summary: "Created a Canny post.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("boardID"), s_req("title"), s_req("details")],
    },
    ActionSpec {
        key: "canny.tickets.update",
        method: "POST",
        path: "/posts/update",
        summary: "Updated a Canny post.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("id"), s("title"), s("details")],
    },
    ActionSpec {
        key: "canny.tickets.delete",
        method: "POST",
        path: "/posts/delete",
        summary: "Deleted a Canny post.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const CANNY_KEYS: &[&str] = &[
    "canny.tickets.list",
    "canny.tickets.search",
    "canny.tickets.get",
    "canny.tickets.create",
    "canny.tickets.update",
    "canny.tickets.delete",
];

pub const CANNY_SPEC: ProviderSpec = ProviderSpec {
    slug: "canny",
    origin: Origin::Static("https://canny.io/api/v1"),
    auth: AuthStyle::BodyField { field: "api_key" },
    actions: CANNY_ACTIONS,
    action_keys: CANNY_KEYS,
};
