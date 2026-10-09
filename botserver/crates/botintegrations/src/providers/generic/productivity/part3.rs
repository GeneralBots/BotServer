use super::*;

const OUTLOOK_CAL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "outlook_calendar.events.list",
        method: "GET",
        path: "/v1.0/me/events",
        summary: "Listed Outlook calendar events.",
        path_params: &[],
        query: &[("$top", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "outlook_calendar.events.create",
        method: "POST",
        path: "/v1.0/me/events",
        summary: "Created an Outlook event.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];
const OUTLOOK_CAL_KEYS: &[&str] = &[
    "outlook_calendar.events.list",
    "outlook_calendar.events.create",
];
pub const OUTLOOK_CALENDAR_SPEC: ProviderSpec = ProviderSpec {
    slug: "outlook_calendar",
    origin: Origin::Static("https://graph.microsoft.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: OUTLOOK_CAL_ACTIONS,
    action_keys: OUTLOOK_CAL_KEYS,
};
const ONEDRIVE_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "onedrive.files.list",
    method: "GET",
    path: "/v1.0/me/drive/root/children",
    summary: "Listed OneDrive root children.",
    path_params: &[],
    query: &[("$top", "limit")],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[s("limit")],
}];
const ONEDRIVE_KEYS: &[&str] = &["onedrive.files.list"];
pub const ONEDRIVE_SPEC: ProviderSpec = ProviderSpec {
    slug: "onedrive",
    origin: Origin::Static("https://graph.microsoft.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: ONEDRIVE_ACTIONS,
    action_keys: ONEDRIVE_KEYS,
};
const SHAREPOINT_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "sharepoint.sites.search",
    method: "GET",
    path: "/v1.0/sites",
    summary: "Searched SharePoint sites.",
    path_params: &[],
    query: &[("search", "query")],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[s_req("query")],
}];
const SHAREPOINT_KEYS: &[&str] = &["sharepoint.sites.search"];
pub const SHAREPOINT_SPEC: ProviderSpec = ProviderSpec {
    slug: "sharepoint",
    origin: Origin::Static("https://graph.microsoft.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: SHAREPOINT_ACTIONS,
    action_keys: SHAREPOINT_KEYS,
};
const CONFLUENCE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "confluence.pages.search",
        method: "GET",
        path: "/api/v2/pages",
        summary: "Listed Confluence pages.",
        path_params: &[],
        query: &[("space-id", "space_id"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("space_id"), s("limit")],
    },
    ActionSpec {
        key: "confluence.pages.get",
        method: "GET",
        path: "/api/v2/pages/{resource_id}",
        summary: "Read a Confluence page.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &resource_id(),
    },
];
const CONFLUENCE_KEYS: &[&str] = &["confluence.pages.search", "confluence.pages.get"];
pub const CONFLUENCE_SPEC: ProviderSpec = ProviderSpec {
    slug: "confluence",
    origin: Origin::FromField {
        field: "site_url",
        pattern: "{value}",
    },
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CONFLUENCE_ACTIONS,
    action_keys: CONFLUENCE_KEYS,
};
const GSHEETS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "sheets.values.get",
        method: "GET",
        path: "/v4/spreadsheets/{spreadsheet_id}/values/{range}",
        summary: "Read a range from a Google Sheet.",
        path_params: &["spreadsheet_id", "range"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("spreadsheet_id"), s_req("range")],
    },
    ActionSpec {
        key: "sheets.rows.append",
        method: "POST",
        path: "/v4/spreadsheets/{spreadsheet_id}/values/{range}:append",
        summary: "Appended rows to a Google Sheet.",
        path_params: &["spreadsheet_id", "range"],
        query: &[("valueInputOption", "input_option")],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("spreadsheet_id"), s_req("range"), json("data", true), s("input_option")],
    },
    ActionSpec {
        key: "sheets.spreadsheets.create",
        method: "POST",
        path: "/v4/spreadsheets",
        summary: "Created a Google Sheet.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];
const GSHEETS_KEYS: &[&str] = &[
    "sheets.values.get",
    "sheets.rows.append",
    "sheets.spreadsheets.create",
];
pub const GOOGLE_SHEETS_SPEC: ProviderSpec = ProviderSpec {
    slug: "sheets",
    origin: Origin::Static("https://sheets.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GSHEETS_ACTIONS,
    action_keys: GSHEETS_KEYS,
};
const GTASKS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "tasks.list",
        method: "GET",
        path: "/lists/{list_id}/tasks",
        summary: "Listed Google Tasks.",
        path_params: &["list_id"],
        query: &[("maxResults", "limit"), ("showCompleted", "show_completed")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("list_id"), s("limit"), s("show_completed")],
    },
    ActionSpec {
        key: "tasks.create",
        method: "POST",
        path: "/lists/{list_id}/tasks",
        summary: "Created a Google Task.",
        path_params: &["list_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("list_id"), json("data", true)],
    },
    ActionSpec {
        key: "tasks.update",
        method: "PATCH",
        path: "/lists/{list_id}/tasks/{resource_id}",
        summary: "Updated a Google Task.",
        path_params: &["list_id", "task_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("list_id"), s_req("task_id"), json("data", true)],
    },
    ActionSpec {
        key: "tasks.complete",
        method: "PATCH",
        path: "/lists/{list_id}/tasks/{resource_id}",
        summary: "Completed a Google Task.",
        path_params: &["list_id", "task_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("list_id"), s_req("task_id"), json("data", true)],
    },
    ActionSpec {
        key: "tasks.delete",
        method: "DELETE",
        path: "/lists/{list_id}/tasks/{resource_id}",
        summary: "Deleted a Google Task.",
        path_params: &["list_id", "task_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("list_id"), s_req("task_id")],
    },
];
const GTASKS_KEYS: &[&str] = &[
    "tasks.list",
    "tasks.create",
    "tasks.update",
    "tasks.complete",
    "tasks.delete",
];
pub const GOOGLE_TASKS_SPEC: ProviderSpec = ProviderSpec {
    slug: "tasks",
    origin: Origin::Static("https://tasks.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GTASKS_ACTIONS,
    action_keys: GTASKS_KEYS,
};
