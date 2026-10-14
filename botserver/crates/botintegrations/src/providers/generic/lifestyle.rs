use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ParamKind, ParamSpec, ProviderSpec, Risk};
const LAST_FM_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "last_fm.library.list",
        method: "GET",
        path: "/2.0/?method=user.gettopartists",
        summary: "Listed top artists for a listener.",
        path_params: &[],
        query: &[("user", "user"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("user"), s("limit")],
    },
    ActionSpec {
        key: "last_fm.library.search",
        method: "GET",
        path: "/2.0/?method=artist.search",
        summary: "Searched Last.fm artists.",
        path_params: &[],
        query: &[("artist", "query"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("limit")],
    },
];

const LAST_FM_KEYS: &[&str] = &["last_fm.library.list", "last_fm.library.search"];

pub const LAST_FM_SPEC: ProviderSpec = ProviderSpec {
    slug: "last_fm",
    origin: Origin::Static("https://ws.audioscrobbler.com"),
    auth: AuthStyle::QueryPairs {
        pairs: &[("api_key", "api_key"), ("format", "format_fixed")],
    },
    actions: LAST_FM_ACTIONS,
    action_keys: LAST_FM_KEYS,
};
const LUMA_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "luma.events.list",
        method: "GET",
        path: "/public/v1/event/list",
        summary: "Listed Luma events.",
        path_params: &[],
        query: &[("pagination_limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "luma.events.search",
        method: "GET",
        path: "/public/v1/event/search",
        summary: "Searched Luma events.",
        path_params: &[],
        query: &[("query", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "luma.events.get",
        method: "GET",
        path: "/public/v1/event",
        summary: "Read a Luma event.",
        path_params: &[],
        query: &[("api_id", "event_id")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("event_id")],
    },
    ActionSpec {
        key: "luma.events.create",
        method: "POST",
        path: "/public/v1/event/create",
        summary: "Created a Luma event.",
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
];

const LUMA_KEYS: &[&str] = &[
    "luma.events.list",
    "luma.events.search",
    "luma.events.get",
    "luma.events.create",
];

pub const LUMA_SPEC: ProviderSpec = ProviderSpec {
    slug: "luma",
    origin: Origin::Static("https://api.lu.ma"),
    auth: AuthStyle::ApiKeyHeader {
        header: "x-luma-api-key",
        field: "api_key",
    },
    actions: LUMA_ACTIONS,
    action_keys: LUMA_KEYS,
};
const HUE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "philips_hue.devices.list",
        method: "GET",
        path: "/clip/v2/resource/device",
        summary: "Listed Hue devices.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "philips_hue.scenes.list",
        method: "GET",
        path: "/clip/v2/resource/scene",
        summary: "Listed Hue scenes.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "philips_hue.devices.update",
        method: "PUT",
        path: "/clip/v2/resource/light/{resource_id}",
        summary: "Updated a Hue light.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("resource_id"), json("data", true)],
    },
    ActionSpec {
        key: "philips_hue.scenes.activate",
        method: "PUT",
        path: "/clip/v2/resource/scene/{resource_id}",
        summary: "Activated a Hue scene.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("resource_id"), json("data", true)],
    },
];

const HUE_KEYS: &[&str] = &[
    "philips_hue.devices.list",
    "philips_hue.scenes.list",
    "philips_hue.devices.update",
    "philips_hue.scenes.activate",
];

pub const PHILIPS_HUE_SPEC: ProviderSpec = ProviderSpec {
    slug: "philips_hue",
    origin: Origin::FromField {
        field: "bridge_url",
        pattern: "{value}",
    },
    auth: AuthStyle::ApiKeyHeader {
        header: "hue-application-key",
        field: "api_key",
    },
    actions: HUE_ACTIONS,
    action_keys: HUE_KEYS,
};
const READWISE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "readwise.bookmarks.list",
        method: "GET",
        path: "/api/v3/list/",
        summary: "Listed saved documents.",
        path_params: &[],
        query: &[("pageCursor", "cursor"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("cursor"), s("limit")],
    },
    ActionSpec {
        key: "readwise.bookmarks.search",
        method: "GET",
        path: "/api/v3/list/",
        summary: "Searched saved documents.",
        path_params: &[],
        query: &[("query", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "readwise.bookmarks.get",
        method: "GET",
        path: "/api/v3/list/{resource_id}",
        summary: "Read a saved document.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];

const READWISE_KEYS: &[&str] = &[
    "readwise.bookmarks.list",
    "readwise.bookmarks.search",
    "readwise.bookmarks.get",
];

pub const READWISE_SPEC: ProviderSpec = ProviderSpec {
    slug: "readwise",
    origin: Origin::Static("https://readwise.io"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: READWISE_ACTIONS,
    action_keys: READWISE_KEYS,
};

// ---------------------------------------------------------------------------
// Cloudflare API v4 - Bearer token; resources are zones.
// ---------------------------------------------------------------------------

const RAINDROP_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "raindrop.bookmarks.list",
        method: "GET",
        path: "/rest/v1/raindrops/0",
        summary: "Listed saved bookmarks.",
        path_params: &[],
        query: &[("perpage", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "raindrop.bookmarks.search",
        method: "GET",
        path: "/rest/v1/raindrops/0",
        summary: "Searched saved bookmarks.",
        path_params: &[],
        query: &[("search", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "raindrop.bookmarks.get",
        method: "GET",
        path: "/rest/v1/raindrop/{resource_id}",
        summary: "Read a bookmark.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "raindrop.bookmarks.create",
        method: "POST",
        path: "/rest/v1/raindrop",
        summary: "Saved a bookmark.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "raindrop.bookmarks.delete",
        method: "DELETE",
        path: "/rest/v1/raindrop/{resource_id}",
        summary: "Deleted a bookmark.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const RAINDROP_KEYS: &[&str] = &[
    "raindrop.bookmarks.list",
    "raindrop.bookmarks.search",
    "raindrop.bookmarks.get",
    "raindrop.bookmarks.create",
    "raindrop.bookmarks.delete",
];

pub const RAINDROP_SPEC: ProviderSpec = ProviderSpec {
    slug: "raindrop",
    origin: Origin::Static("https://api.raindrop.io"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: RAINDROP_ACTIONS,
    action_keys: RAINDROP_KEYS,
};

const FITBIT_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "fitbit.activities.list",
        method: "GET",
        path: "/1/user/-/activities/list.json",
        summary: "Listed Fitbit activities.",
        path_params: &[],
        query: &[("afterDate", "after_date"), ("sort", "sort"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("after_date"), s("sort"), s("limit")],
    },
    ActionSpec {
        key: "fitbit.metrics.query",
        method: "GET",
        path: "/1/user/-/activities/heart-rate/date/today/1d.json",
        summary: "Read today's heart-rate series.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "fitbit.activities.delete",
        method: "DELETE",
        path: "/1/user/-/activities/{resource_id}.json",
        summary: "Deleted a logged activity.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const FITBIT_KEYS: &[&str] = &[
    "fitbit.activities.list",
    "fitbit.metrics.query",
    "fitbit.activities.delete",
];

pub const FITBIT_SPEC: ProviderSpec = ProviderSpec {
    slug: "fitbit",
    origin: Origin::Static("https://api.fitbit.com"),
    auth: AuthStyle::BearerHeaders {
        token_field: "token",
        headers: &[],
    },
    actions: FITBIT_ACTIONS,
    action_keys: FITBIT_KEYS,
};
