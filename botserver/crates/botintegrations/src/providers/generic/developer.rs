use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ParamKind, ParamSpec, ProviderSpec, Risk};
const AHREFS_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "ahrefs.keywords.search",
    method: "GET",
    path: "/v3/site-explorer/organic-keywords",
    summary: "Fetched organic keywords for a target.",
    path_params: &[],
    query: &[("target", "target"), ("country", "country"), ("limit", "limit")],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[s_req("target"), s("country"), s("limit")],
}];

const AHREFS_KEYS: &[&str] = &["ahrefs.keywords.search"];

pub const AHREFS_SPEC: ProviderSpec = ProviderSpec {
    slug: "ahrefs",
    origin: Origin::Static("https://api.ahrefs.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: AHREFS_ACTIONS,
    action_keys: AHREFS_KEYS,
};
const ALGOLIA_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "algolia.indexes.list",
        method: "GET",
        path: "/1/indexes",
        summary: "Listed Algolia indexes.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "algolia.records.search",
        method: "POST",
        path: "/1/indexes/{index_id}/query",
        summary: "Searched an Algolia index.",
        path_params: &["index_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("index_id"), json("data", true)],
    },
    ActionSpec {
        key: "algolia.records.upsert",
        method: "PUT",
        path: "/1/indexes/{index_id}/objects/{object_id}",
        summary: "Upserted an Algolia object.",
        path_params: &["index_id", "object_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("index_id"), s_req("object_id"), json("data", true)],
    },
    ActionSpec {
        key: "algolia.records.delete",
        method: "DELETE",
        path: "/1/indexes/{index_id}/objects/{resource_id}",
        summary: "Deleted an Algolia object.",
        path_params: &["index_id", "resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: &[
            s_req("index_id"),
            ParamSpec {
                name: "resource_id",
                kind: ParamKind::Str,
                required: true,
            },
        ],
    },
];

const ALGOLIA_KEYS: &[&str] = &[
    "algolia.indexes.list",
    "algolia.records.search",
    "algolia.records.upsert",
    "algolia.records.delete",
];

pub const ALGOLIA_SPEC: ProviderSpec = ProviderSpec {
    slug: "algolia",
    origin: Origin::FromField {
        field: "application_id",
        pattern: "https://{value}-dsn.algolia.net",
    },
    auth: AuthStyle::ApiKeyHeaders {
        pairs: &[
            ("X-Algolia-Application-Id", "application_id"),
            ("X-Algolia-API-Key", "api_key"),
        ],
    },
    actions: ALGOLIA_ACTIONS,
    action_keys: ALGOLIA_KEYS,
};
const CLOUDFLARE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "cloudflare.resources.list",
        method: "GET",
        path: "/client/v4/zones",
        summary: "Listed Cloudflare zones.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "cloudflare.resources.search",
        method: "GET",
        path: "/client/v4/zones",
        summary: "Searched Cloudflare zones.",
        path_params: &[],
        query: &[("name", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "cloudflare.resources.get",
        method: "GET",
        path: "/client/v4/zones/{resource_id}",
        summary: "Read a Cloudflare zone.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const CLOUDFLARE_KEYS: &[&str] = &[
    "cloudflare.resources.list",
    "cloudflare.resources.search",
    "cloudflare.resources.get",
];

pub const CLOUDFLARE_SPEC: ProviderSpec = ProviderSpec {
    slug: "cloudflare",
    origin: Origin::Static("https://api.cloudflare.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CLOUDFLARE_ACTIONS,
    action_keys: CLOUDFLARE_KEYS,
};

// ---------------------------------------------------------------------------
// Hugging Face Hub API - Bearer token. Verified subset: model listing and
// search (inference runs require per-endpoint routing and stay planned).
// ---------------------------------------------------------------------------
const CURSOR_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "cursor.teams.members",
    method: "GET",
    path: "/v0/teams/members",
    summary: "Listed Cursor team members.",
    path_params: &[],
    query: &[],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[],
}];

const CURSOR_KEYS: &[&str] = &["cursor.teams.members"];

pub const CURSOR_SPEC: ProviderSpec = ProviderSpec {
    slug: "cursor",
    origin: Origin::Static("https://api.cursor.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: CURSOR_ACTIONS,
    action_keys: CURSOR_KEYS,
};
const HF_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "hugging_face.models.list",
        method: "GET",
        path: "/api/models",
        summary: "Listed Hugging Face models.",
        path_params: &[],
        query: &[("limit", "limit"), ("sort", "sort")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("sort")],
    },
    ActionSpec {
        key: "hugging_face.models.search",
        method: "GET",
        path: "/api/models",
        summary: "Searched Hugging Face models.",
        path_params: &[],
        query: &[("search", "query"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("limit")],
    },
];

const HF_KEYS: &[&str] = &["hugging_face.models.list", "hugging_face.models.search"];

pub const HUGGING_FACE_SPEC: ProviderSpec = ProviderSpec {
    slug: "hugging_face",
    origin: Origin::Static("https://huggingface.co"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: HF_ACTIONS,
    action_keys: HF_KEYS,
};

// ---------------------------------------------------------------------------
// Resend REST API - Bearer api key.
// ---------------------------------------------------------------------------
const RESEND_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "resend.messages.list",
        method: "GET",
        path: "/emails",
        summary: "Listed sent emails.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "resend.messages.get",
        method: "GET",
        path: "/emails/{resource_id}",
        summary: "Read a sent email.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "resend.domains.list",
        method: "GET",
        path: "/domains",
        summary: "Listed sending domains.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "resend.messages.send",
        method: "POST",
        path: "/emails",
        summary: "Sent an email via Resend.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];

const RESEND_KEYS: &[&str] = &[
    "resend.messages.list",
    "resend.messages.get",
    "resend.domains.list",
    "resend.messages.send",
];

pub const RESEND_SPEC: ProviderSpec = ProviderSpec {
    slug: "resend",
    origin: Origin::Static("https://api.resend.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: RESEND_ACTIONS,
    action_keys: RESEND_KEYS,
};

// ---------------------------------------------------------------------------
// Sentry Web API - Bearer token.
// ---------------------------------------------------------------------------
const SUPABASE_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "supabase.projects.list",
    method: "GET",
    path: "/v1/projects",
    summary: "Listed Supabase projects.",
    path_params: &[],
    query: &[],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[],
}];

const SUPABASE_KEYS: &[&str] = &["supabase.projects.list"];

pub const SUPABASE_SPEC: ProviderSpec = ProviderSpec {
    slug: "supabase",
    origin: Origin::Static("https://api.supabase.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: SUPABASE_ACTIONS,
    action_keys: SUPABASE_KEYS,
};
const VERCEL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "vercel.projects.list",
        method: "GET",
        path: "/v2/projects",
        summary: "Listed Vercel projects.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "vercel.deployments.list",
        method: "GET",
        path: "/v6/deployments",
        summary: "Listed Vercel deployments.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "vercel.deployments.get",
        method: "GET",
        path: "/v13/deployments/{resource_id}",
        summary: "Read a Vercel deployment.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "vercel.deployments.cancel",
        method: "PATCH",
        path: "/v12/deployments/{resource_id}/cancel",
        summary: "Cancelled a Vercel deployment.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const VERCEL_KEYS: &[&str] = &[
    "vercel.projects.list",
    "vercel.deployments.list",
    "vercel.deployments.get",
    "vercel.deployments.cancel",
];

pub const VERCEL_SPEC: ProviderSpec = ProviderSpec {
    slug: "vercel",
    origin: Origin::Static("https://api.vercel.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: VERCEL_ACTIONS,
    action_keys: VERCEL_KEYS,
};
