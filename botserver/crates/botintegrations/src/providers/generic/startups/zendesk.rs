use super::*;

const ZENDESK_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "zendesk.tickets.list",
        method: "GET",
        path: "/tickets.json",
        summary: "Listed support tickets.",
        path_params: &[],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "zendesk.tickets.search",
        method: "GET",
        path: "/search.json",
        summary: "Searched support tickets.",
        path_params: &[],
        query: &[("query", "query"), ("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("limit")],
    },
    ActionSpec {
        key: "zendesk.tickets.get",
        method: "GET",
        path: "/tickets/{resource_id}.json",
        summary: "Read support ticket.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "zendesk.tickets.create",
        method: "POST",
        path: "/tickets.json",
        summary: "Created support ticket.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: Some("ticket"),
        risk: Risk::Medium,
        params: &[json_req("data")],
    },
    ActionSpec {
        key: "zendesk.tickets.update",
        method: "PUT",
        path: "/tickets/{resource_id}.json",
        summary: "Updated support ticket.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: Some("ticket"),
        risk: Risk::Medium,
        params: &[s_req("resource_id"), json_req("data")],
    },
    ActionSpec {
        key: "zendesk.tickets.delete",
        method: "DELETE",
        path: "/tickets/{resource_id}.json",
        summary: "Deleted support ticket.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const ZENDESK_KEYS: &[&str] = &[
    "zendesk.tickets.list",
    "zendesk.tickets.search",
    "zendesk.tickets.get",
    "zendesk.tickets.create",
    "zendesk.tickets.update",
    "zendesk.tickets.delete",
];

pub const ZENDESK_SPEC: ProviderSpec = ProviderSpec {
    slug: "zendesk",
    origin: Origin::ZendeskSubdomain,
    auth: AuthStyle::BasicTemplate {
        user_template: "{email}/token",
        password_field: "token",
    },
    actions: ZENDESK_ACTIONS,
    action_keys: ZENDESK_KEYS,
};

// ---------------------------------------------------------------------------
// Trello REST API - key/token query authentication.
// ---------------------------------------------------------------------------

// HubSpot CRM API v3 - Bearer token (private app or OAuth).

const HUBSPOT_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "hubspot.contacts.list",
        method: "GET",
        path: "/crm/v3/objects/contacts",
        summary: "Listed HubSpot contacts.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "hubspot.contacts.search",
        method: "POST",
        path: "/crm/v3/objects/contacts/search",
        summary: "Searched HubSpot contacts.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "hubspot.deals.list",
        method: "GET",
        path: "/crm/v3/objects/deals",
        summary: "Listed HubSpot deals.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "hubspot.contacts.create",
        method: "POST",
        path: "/crm/v3/objects/contacts",
        summary: "Created a HubSpot contact.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "hubspot.contacts.update",
        method: "PATCH",
        path: "/crm/v3/objects/contacts/{resource_id}",
        summary: "Updated a HubSpot contact.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("resource_id"), json("data", true)],
    },
    ActionSpec {
        key: "hubspot.contacts.delete",
        method: "DELETE",
        path: "/crm/v3/objects/contacts/{resource_id}",
        summary: "Deleted a HubSpot contact.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const HUBSPOT_KEYS: &[&str] = &[
    "hubspot.contacts.list",
    "hubspot.contacts.search",
    "hubspot.deals.list",
    "hubspot.contacts.create",
    "hubspot.contacts.update",
    "hubspot.contacts.delete",
];

pub const HUBSPOT_SPEC: ProviderSpec = ProviderSpec {
    slug: "hubspot",
    origin: Origin::Static("https://api.hubapi.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: HUBSPOT_ACTIONS,
    action_keys: HUBSPOT_KEYS,
};

const CLICKFUNNELS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "clickfunnels.campaigns.list",
        method: "GET",
        path: "/workspaces/{workspace_id}/funnels",
        summary: "Listed ClickFunnels funnels.",
        path_params: &["workspace_id"],
        query: &[("per_page", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("workspace_id"), s("limit")],
    },
    ActionSpec {
        key: "clickfunnels.contacts.list",
        method: "GET",
        path: "/workspaces/{workspace_id}/contacts",
        summary: "Listed ClickFunnels contacts.",
        path_params: &["workspace_id"],
        query: &[("per_page", "limit"), ("filter[email]", "email")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("workspace_id"), s("limit"), s("email")],
    },
];

const CLICKFUNNELS_KEYS: &[&str] = &[
    "clickfunnels.campaigns.list",
    "clickfunnels.contacts.list",
];

pub const CLICKFUNNELS_SPEC: ProviderSpec = ProviderSpec {
    slug: "clickfunnels",
    origin: Origin::Static("https://api.myclickfunnels.com/api/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CLICKFUNNELS_ACTIONS,
    action_keys: CLICKFUNNELS_KEYS,
};
