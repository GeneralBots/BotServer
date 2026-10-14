use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const SALESFORCE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "salesforce.contacts.list",
        method: "GET",
        path: "/services/data/v59.0/sobjects/Contact",
        summary: "Described HubSpot-style Salesforce contacts.",
        path_params: &[],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "salesforce.contacts.search",
        method: "GET",
        path: "/services/data/v59.0/query",
        summary: "Queried Salesforce records with SOQL.",
        path_params: &[],
        query: &[("q", "query")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query")],
    },
    ActionSpec {
        key: "salesforce.contacts.create",
        method: "POST",
        path: "/services/data/v59.0/sobjects/Contact",
        summary: "Created a Salesforce contact.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "salesforce.contacts.update",
        method: "PATCH",
        path: "/services/data/v59.0/sobjects/Contact/{resource_id}",
        summary: "Updated a Salesforce contact.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("contact_id"), json("data", true)],
    },
];

const SALESFORCE_KEYS: &[&str] = &[
    "salesforce.contacts.list",
    "salesforce.contacts.search",
    "salesforce.contacts.create",
    "salesforce.contacts.update",
];

pub const SALESFORCE_SPEC: ProviderSpec = ProviderSpec {
    slug: "salesforce",
    origin: Origin::FromField {
        field: "instance_url",
        pattern: "{value}",
    },
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: SALESFORCE_ACTIONS,
    action_keys: SALESFORCE_KEYS,
};

const HIGHLEVEL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "highlevel.contacts.list",
        method: "GET",
        path: "/contacts/",
        summary: "Listed HighLevel contacts.",
        path_params: &[],
        query: &[("locationId", "location_id"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("location_id"), s("limit")],
    },
    ActionSpec {
        key: "highlevel.contacts.search",
        method: "GET",
        path: "/contacts/{resource_id}",
        summary: "Read a HighLevel contact.",
        path_params: &["contact_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("contact_id")],
    },
    ActionSpec {
        key: "highlevel.contacts.create",
        method: "POST",
        path: "/contacts/",
        summary: "Created a HighLevel contact.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "highlevel.contacts.update",
        method: "PUT",
        path: "/contacts/{resource_id}",
        summary: "Updated a HighLevel contact.",
        path_params: &["contact_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("contact_id"), json("data", true)],
    },
];

const HIGHLEVEL_KEYS: &[&str] = &[
    "highlevel.contacts.list",
    "highlevel.contacts.search",
    "highlevel.contacts.create",
    "highlevel.contacts.update",
];

pub const HIGHLEVEL_SPEC: ProviderSpec = ProviderSpec {
    slug: "highlevel",
    origin: Origin::Static("https://services.leadconnectorhq.com/contacts"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: HIGHLEVEL_ACTIONS,
    action_keys: HIGHLEVEL_KEYS,
};

const DOCUSIGN_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "docusign.envelopes.list",
        method: "GET",
        path: "/restapi/v2.1/accounts/{account_id}/envelopes",
        summary: "Listed DocuSign envelopes.",
        path_params: &["account_id"],
        query: &[("from_date", "from_date")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id"), s("from_date")],
    },
    ActionSpec {
        key: "docusign.envelopes.get",
        method: "GET",
        path: "/restapi/v2.1/accounts/{account_id}/envelopes/{envelope_id}",
        summary: "Read a DocuSign envelope.",
        path_params: &["account_id", "envelope_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("account_id"), s_req("envelope_id")],
    },
    ActionSpec {
        key: "docusign.envelopes.create",
        method: "POST",
        path: "/restapi/v2.1/accounts/{account_id}/envelopes",
        summary: "Created and sent a DocuSign envelope.",
        path_params: &["account_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("account_id"), json("data", true)],
    },
];

const DOCUSIGN_KEYS: &[&str] = &[
    "docusign.envelopes.list",
    "docusign.envelopes.get",
    "docusign.envelopes.create",
];

pub const DOCUSIGN_SPEC: ProviderSpec = ProviderSpec {
    slug: "docusign",
    origin: Origin::FromField {
        field: "base_url",
        pattern: "{value}",
    },
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: DOCUSIGN_ACTIONS,
    action_keys: DOCUSIGN_KEYS,
};

const ATTIO_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "attio.contacts.list",
        method: "GET",
        path: "/v2/objects/people/records",
        summary: "Listed Attio people records.",
        path_params: &[],
        query: &[("pageSize", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "attio.contacts.search",
        method: "POST",
        path: "/v2/records/people/query",
        summary: "Queried Attio people records.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "attio.deals.list",
        method: "POST",
        path: "/v2/records/deals/query",
        summary: "Queried Attio deal records.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "attio.contacts.create",
        method: "POST",
        path: "/v2/records/people",
        summary: "Created an Attio person record.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];

const ATTIO_KEYS: &[&str] = &[
    "attio.contacts.list",
    "attio.contacts.search",
    "attio.deals.list",
    "attio.contacts.create",
];

pub const ATTIO_SPEC: ProviderSpec = ProviderSpec {
    slug: "attio",
    origin: Origin::Static("https://api.attio.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: ATTIO_ACTIONS,
    action_keys: ATTIO_KEYS,
};

const CANVA_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "canva.assets.list",
        method: "GET",
        path: "/v1/assets",
        summary: "Listed Canva assets.",
        path_params: &[],
        query: &[("continuation", "cursor")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("cursor")],
    },
    ActionSpec {
        key: "canva.assets.get",
        method: "GET",
        path: "/v1/assets/{resource_id}",
        summary: "Read a Canva asset.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "canva.assets.delete",
        method: "DELETE",
        path: "/v1/assets/{resource_id}",
        summary: "Deleted a Canva asset.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const CANVA_KEYS: &[&str] = &["canva.assets.list", "canva.assets.get", "canva.assets.delete"];

pub const CANVA_SPEC: ProviderSpec = ProviderSpec {
    slug: "canva",
    origin: Origin::Static("https://api.canva.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CANVA_ACTIONS,
    action_keys: CANVA_KEYS,
};
