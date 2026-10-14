use super::helpers::{json, json_req, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const BEEHIIV_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "beehiiv.content.list",
        method: "GET",
        path: "/public/v1/publications/{publication_id}/posts",
        summary: "Listed beehiiv posts.",
        path_params: &["publication_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("publication_id"), s("limit")],
    },
    ActionSpec {
        key: "beehiiv.content.search",
        method: "GET",
        path: "/public/v1/publications/{publication_id}/posts",
        summary: "Searched beehiiv posts.",
        path_params: &["publication_id"],
        query: &[("query", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("publication_id"), s_req("query")],
    },
    ActionSpec {
        key: "beehiiv.subscriptions.list",
        method: "GET",
        path: "/public/v1/publications/{publication_id}/subscriptions",
        summary: "Listed beehiiv subscriptions.",
        path_params: &["publication_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("publication_id"), s("limit")],
    },
];

const BEEHIIV_KEYS: &[&str] = &[
    "beehiiv.content.list",
    "beehiiv.content.search",
    "beehiiv.subscriptions.list",
];

pub const BEEHIIV_SPEC: ProviderSpec = ProviderSpec {
    slug: "beehiiv",
    origin: Origin::Static("https://api.beehiiv.com"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: BEEHIIV_ACTIONS,
    action_keys: BEEHIIV_KEYS,
};
const LEMLIST_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "lemlist.campaigns.list",
        method: "GET",
        path: "/campaigns",
        summary: "Listed Lemlist campaigns.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "lemlist.campaigns.reports.get",
        method: "GET",
        path: "/campaigns/{campaign_id}/stats",
        summary: "Read Lemlist campaign statistics.",
        path_params: &["campaign_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("campaign_id")],
    },
];

const LEMLIST_KEYS: &[&str] = &[
    "lemlist.campaigns.list",
    "lemlist.campaigns.reports.get",
];

pub const LEMLIST_SPEC: ProviderSpec = ProviderSpec {
    slug: "lemlist",
    origin: Origin::Static("https://api.lemlist.com/api"),
    auth: AuthStyle::BasicTemplate {
        user_template: "",
        password_field: "api_key",
    },
    actions: LEMLIST_ACTIONS,
    action_keys: LEMLIST_KEYS,
};

// ---------------------------------------------------------------------------
// Luma public API v1 - x-luma-api-key header. Verified subset of the catalog
// (public API has no delete operation).
// ---------------------------------------------------------------------------
const LOOPS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "loops_so.contacts.search",
        method: "GET",
        path: "/contacts/search",
        summary: "Searched Loops contacts by email.",
        path_params: &[],
        query: &[("email", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "loops_so.contacts.create",
        method: "POST",
        path: "/contacts/create",
        summary: "Created a Loops contact.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "loops_so.campaigns.send",
        method: "POST",
        path: "/transactional/send",
        summary: "Sent a Loops transactional email.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];

const LOOPS_KEYS: &[&str] = &[
    "loops_so.contacts.search",
    "loops_so.contacts.create",
    "loops_so.campaigns.send",
];

pub const LOOPS_SO_SPEC: ProviderSpec = ProviderSpec {
    slug: "loops_so",
    origin: Origin::Static("https://app.loops.so/api/v1"),
    auth: AuthStyle::Bearer {
        token_field: "api_key",
    },
    actions: LOOPS_ACTIONS,
    action_keys: LOOPS_KEYS,
};
const MAILCHIMP_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "mailchimp.campaigns.list",
        method: "GET",
        path: "/campaigns",
        summary: "Listed Mailchimp campaigns.",
        path_params: &[],
        query: &[("count", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "mailchimp.contacts.search",
        method: "GET",
        path: "/search-members",
        summary: "Searched Mailchimp contacts.",
        path_params: &[],
        query: &[("query", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "mailchimp.campaigns.reports.get",
        method: "GET",
        path: "/reports/{campaign_id}",
        summary: "Read campaign report.",
        path_params: &["campaign_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("campaign_id")],
    },
    ActionSpec {
        key: "mailchimp.campaigns.create",
        method: "POST",
        path: "/campaigns",
        summary: "Created a Mailchimp campaign.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json_req("data")],
    },
    ActionSpec {
        key: "mailchimp.campaigns.send",
        method: "POST",
        path: "/campaigns/{campaign_id}/actions/send",
        summary: "Sent a Mailchimp campaign.",
        path_params: &["campaign_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("campaign_id")],
    },
    ActionSpec {
        key: "mailchimp.campaigns.delete",
        method: "DELETE",
        path: "/campaigns/{campaign_id}",
        summary: "Deleted a Mailchimp campaign.",
        path_params: &["campaign_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("campaign_id")],
    },
];

const MAILCHIMP_KEYS: &[&str] = &[
    "mailchimp.campaigns.list",
    "mailchimp.contacts.search",
    "mailchimp.campaigns.reports.get",
    "mailchimp.campaigns.create",
    "mailchimp.campaigns.send",
    "mailchimp.campaigns.delete",
];

pub const MAILCHIMP_SPEC: ProviderSpec = ProviderSpec {
    slug: "mailchimp",
    origin: Origin::MailchimpDataCenter,
    auth: AuthStyle::BasicTemplate {
        user_template: "anystring",
        password_field: "api_key",
    },
    actions: MAILCHIMP_ACTIONS,
    action_keys: MAILCHIMP_KEYS,
};

// ---------------------------------------------------------------------------
// Mercury Platform API - Bearer token.
// ---------------------------------------------------------------------------
const PHANTOMBUSTER_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "phantombuster.workflows.list",
        method: "GET",
        path: "/workflows/fetch-all",
        summary: "Listed PhantomBuster workflows.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "phantombuster.runs.list",
        method: "GET",
        path: "/workflows/fetch-containers",
        summary: "Listed PhantomBuster container runs.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
];

const PHANTOMBUSTER_KEYS: &[&str] = &[
    "phantombuster.workflows.list",
    "phantombuster.runs.list",
];

pub const PHANTOMBUSTER_SPEC: ProviderSpec = ProviderSpec {
    slug: "phantombuster",
    origin: Origin::Static("https://api.phantombuster.com/api/v2"),
    auth: AuthStyle::ApiKeyHeader {
        header: "X-Phantombuster-Key",
        field: "api_key",
    },
    actions: PHANTOMBUSTER_ACTIONS,
    action_keys: PHANTOMBUSTER_KEYS,
};

const ZOOM_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "zoom.meetings.list",
        method: "GET",
        path: "/users/me/meetings",
        summary: "Listed Zoom meetings.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "zoom.meetings.get",
        method: "GET",
        path: "/meetings/{meeting_id}",
        summary: "Read a Zoom meeting.",
        path_params: &["meeting_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("meeting_id")],
    },
    ActionSpec {
        key: "zoom.recordings.list",
        method: "GET",
        path: "/meetings/{meeting_id}/recordings",
        summary: "Listed meeting recordings.",
        path_params: &["meeting_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("meeting_id")],
    },
    ActionSpec {
        key: "zoom.meetings.create",
        method: "POST",
        path: "/users/me/meetings",
        summary: "Created a Zoom meeting.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "zoom.meetings.cancel",
        method: "DELETE",
        path: "/meetings/{meeting_id}",
        summary: "Cancelled a Zoom meeting.",
        path_params: &["meeting_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("meeting_id")],
    },
];

const ZOOM_KEYS: &[&str] = &[
    "zoom.meetings.list",
    "zoom.meetings.get",
    "zoom.recordings.list",
    "zoom.meetings.create",
    "zoom.meetings.cancel",
];

pub const ZOOM_SPEC: ProviderSpec = ProviderSpec {
    slug: "zoom",
    origin: Origin::Static("https://api.zoom.us/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: ZOOM_ACTIONS,
    action_keys: ZOOM_KEYS,
};

const INTERCOM_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "intercom.tickets.list",
        method: "GET",
        path: "/tickets",
        summary: "Listed Intercom tickets.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "intercom.tickets.search",
        method: "POST",
        path: "/tickets/search",
        summary: "Searched Intercom tickets.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "intercom.tickets.get",
        method: "GET",
        path: "/tickets/{resource_id}",
        summary: "Read an Intercom ticket.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "intercom.tickets.create",
        method: "POST",
        path: "/tickets",
        summary: "Created an Intercom ticket.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];

const INTERCOM_KEYS: &[&str] = &[
    "intercom.tickets.list",
    "intercom.tickets.search",
    "intercom.tickets.get",
    "intercom.tickets.create",
];

pub const INTERCOM_SPEC: ProviderSpec = ProviderSpec {
    slug: "intercom",
    origin: Origin::Static("https://api.intercom.io"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: INTERCOM_ACTIONS,
    action_keys: INTERCOM_KEYS,
};
