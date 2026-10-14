//! Developer-platform adapters: Mintlify (#960), Pydantic Logfire (#964),
//! Grain (#1012), Descript (#1008) and Google Workspace Admin (#1011).
//! Each spec follows the shared ActionSpec/ProviderSpec shape consumed by
//! `GenericAdapter`; credentials come from Vault-backed connections.

use super::helpers::{json, resource_id, s};
use super::{ActionSpec, AuthStyle, Origin, ParamKind, ParamSpec, ProviderSpec, Risk};

// ── Mintlify ─────────────────────────────────────────────────────

const MINTLIFY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "mintlify.projects.list",
        method: "GET",
        path: "/v1/projects",
        summary: "Listed Mintlify docs projects.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "mintlify.pages.list",
        method: "GET",
        path: "/v1/projects/{resource_id}/pages",
        summary: "Listed pages of a docs project.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "mintlify.docs.search",
        method: "POST",
        path: "/v1/search",
        summary: "Searched documentation content.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "mintlify.pages.update",
        method: "PUT",
        path: "/v1/projects/{resource_id}/pages/{page_id}",
        summary: "Updated a documentation page.",
        path_params: &["resource_id", "page_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[
            ParamSpec {
                name: "resource_id",
                kind: ParamKind::Str,
                required: true,
            },
            ParamSpec {
                name: "page_id",
                kind: ParamKind::Str,
                required: true,
            },
            json("data", true),
        ],
    },
];

const MINTLIFY_KEYS: &[&str] = &[
    "mintlify.projects.list",
    "mintlify.pages.list",
    "mintlify.docs.search",
    "mintlify.pages.update",
];

pub const MINTLIFY_SPEC: ProviderSpec = ProviderSpec {
    slug: "mintlify",
    origin: Origin::Static("https://api.mintlify.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: MINTLIFY_ACTIONS,
    action_keys: MINTLIFY_KEYS,
};

// ── Pydantic Logfire ─────────────────────────────────────────────

const LOGFIRE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "logfire.organizations.list",
        method: "GET",
        path: "/v1/organizations",
        summary: "Listed Logfire organizations.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "logfire.projects.list",
        method: "GET",
        path: "/v1/projects",
        summary: "Listed Logfire projects.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "logfire.query.exec",
        method: "POST",
        path: "/v1/query/exec",
        summary: "Executed a read-only SQL query over telemetry.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
];

const LOGFIRE_KEYS: &[&str] = &[
    "logfire.organizations.list",
    "logfire.projects.list",
    "logfire.query.exec",
];

pub const LOGFIRE_SPEC: ProviderSpec = ProviderSpec {
    slug: "pydantic_logfire",
    origin: Origin::Static("https://logfire-api.pydantic.dev"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: LOGFIRE_ACTIONS,
    action_keys: LOGFIRE_KEYS,
};

// ── Grain ────────────────────────────────────────────────────────

const GRAIN_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "grain.recordings.list",
        method: "GET",
        path: "/recordings",
        summary: "Listed meeting recordings.",
        path_params: &[],
        query: &[("limit", "limit"), ("cursor", "cursor")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("cursor")],
    },
    ActionSpec {
        key: "grain.recordings.get",
        method: "GET",
        path: "/recordings/{resource_id}",
        summary: "Fetched a meeting recording with transcript.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "grain.highlights.list",
        method: "GET",
        path: "/highlights",
        summary: "Listed highlights across recordings.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
];

const GRAIN_KEYS: &[&str] = &[
    "grain.recordings.list",
    "grain.recordings.get",
    "grain.highlights.list",
];

pub const GRAIN_SPEC: ProviderSpec = ProviderSpec {
    slug: "grain",
    origin: Origin::Static("https://api.grain.com/_api/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GRAIN_ACTIONS,
    action_keys: GRAIN_KEYS,
};

// ── Descript ─────────────────────────────────────────────────────

const DESCRIPT_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "descript.projects.list",
        method: "GET",
        path: "/projects",
        summary: "Listed Descript projects.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "descript.projects.get",
        method: "GET",
        path: "/projects/{resource_id}",
        summary: "Fetched a Descript project.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "descript.transcripts.get",
        method: "GET",
        path: "/transcripts/{resource_id}",
        summary: "Fetched a transcript.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const DESCRIPT_KEYS: &[&str] = &[
    "descript.projects.list",
    "descript.projects.get",
    "descript.transcripts.get",
];

pub const DESCRIPT_SPEC: ProviderSpec = ProviderSpec {
    slug: "descript",
    origin: Origin::Static("https://api.descript.com/v3"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: DESCRIPT_ACTIONS,
    action_keys: DESCRIPT_KEYS,
};

// ── Google Workspace Admin (Directory API) ───────────────────────

const GWSADMIN_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "gwsadmin.users.list",
        method: "GET",
        path: "/users",
        summary: "Listed workspace users for a domain.",
        path_params: &[],
        query: &[
            ("domain", "domain"),
            ("query", "query"),
            ("maxResults", "limit"),
        ],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("domain"), s("query"), s("limit")],
    },
    ActionSpec {
        key: "gwsadmin.users.get",
        method: "GET",
        path: "/users/{resource_id}",
        summary: "Fetched a workspace user.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "gwsadmin.users.create",
        method: "POST",
        path: "/users",
        summary: "Created a workspace user.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "gwsadmin.users.suspend",
        method: "POST",
        path: "/users/{resource_id}/suspend",
        summary: "Suspended a workspace user.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
    ActionSpec {
        key: "gwsadmin.groups.list",
        method: "GET",
        path: "/groups",
        summary: "Listed workspace groups.",
        path_params: &[],
        query: &[("domain", "domain"), ("maxResults", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("domain"), s("limit")],
    },
    ActionSpec {
        key: "gwsadmin.members.list",
        method: "GET",
        path: "/groups/{resource_id}/members",
        summary: "Listed members of a group.",
        path_params: &["resource_id"],
        query: &[("maxResults", "limit")],
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

const GWSADMIN_KEYS: &[&str] = &[
    "gwsadmin.users.list",
    "gwsadmin.users.get",
    "gwsadmin.users.create",
    "gwsadmin.users.suspend",
    "gwsadmin.groups.list",
    "gwsadmin.members.list",
];

pub const GWSADMIN_SPEC: ProviderSpec = ProviderSpec {
    slug: "google_workspace_admin",
    origin: Origin::Static("https://admin.googleapis.com/directory/v1"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GWSADMIN_ACTIONS,
    action_keys: GWSADMIN_KEYS,
};

// ── Google Cloud (Resource Manager) ─────────────────────────────

const GCLOUD_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "google_cloud.projects.list",
        method: "GET",
        path: "/v1/projects",
        summary: "Listed accessible GCP projects.",
        path_params: &[],
        query: &[("pageSize", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "google_cloud.projects.get",
        method: "GET",
        path: "/v1/projects/{resource_id}",
        summary: "Fetched a GCP project.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const GCLOUD_KEYS: &[&str] = &[
    "google_cloud.projects.list",
    "google_cloud.projects.get",
];

pub const GCLOUD_SPEC: ProviderSpec = ProviderSpec {
    slug: "google_cloud",
    origin: Origin::Static("https://cloudresourcemanager.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GCLOUD_ACTIONS,
    action_keys: GCLOUD_KEYS,
};
