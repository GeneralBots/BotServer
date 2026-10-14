use super::*;

pub const TODOIST_SPEC: ProviderSpec = ProviderSpec {
    slug: "todoist",
    origin: Origin::Static("https://api.todoist.com/rest/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: TODOIST_ACTIONS,
    action_keys: TODOIST_KEYS,
};
const NOTION_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "notion.pages.search",
        method: "POST",
        path: "/search",
        summary: "Searched Notion pages and databases.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", false)],
    },
    ActionSpec {
        key: "notion.pages.get",
        method: "GET",
        path: "/pages/{page_id}",
        summary: "Read a Notion page.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "notion.pages.create",
        method: "POST",
        path: "/pages",
        summary: "Created a Notion page.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "notion.pages.update",
        method: "PATCH",
        path: "/pages/{resource_id}",
        summary: "Updated Notion page properties.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("page_id"), json("data", true)],
    },
    ActionSpec {
        key: "notion.pages.archive",
        method: "PATCH",
        path: "/pages/{resource_id}",
        summary: "Archived a Notion page.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[s_req("page_id"), json("data", true)],
    },
];
const NOTION_KEYS: &[&str] = &[
    "notion.pages.search",
    "notion.pages.get",
    "notion.pages.create",
    "notion.pages.update",
    "notion.pages.archive",
];
pub const NOTION_SPEC: ProviderSpec = ProviderSpec {
    slug: "notion",
    origin: Origin::Static("https://api.notion.com/v1"),
    auth: AuthStyle::BearerHeaders {
        token_field: "token",
        headers: &[("Notion-Version", "2022-06-28")],
    },
    actions: NOTION_ACTIONS,
    action_keys: NOTION_KEYS,
};
const CALENDLY_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "calendly.events.list",
        method: "GET",
        path: "/scheduled_events",
        summary: "Listed Calendly scheduled events.",
        path_params: &[],
        query: &[("count", "limit"), ("user", "user_uri")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("user_uri")],
    },
    ActionSpec {
        key: "calendly.users.me",
        method: "GET",
        path: "/users/me",
        summary: "Read the Calendly current user.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
];
const CALENDLY_KEYS: &[&str] = &["calendly.events.list", "calendly.users.me"];
pub const CALENDLY_SPEC: ProviderSpec = ProviderSpec {
    slug: "calendly",
    origin: Origin::Static("https://api.calendly.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CALENDLY_ACTIONS,
    action_keys: CALENDLY_KEYS,
};
const GOOGLE_DRIVE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "drive.files.list",
        method: "GET",
        path: "/drive/v3/files",
        summary: "Listed Google Drive files.",
        path_params: &[],
        query: &[("pageSize", "limit"), ("q", "query"), ("pageToken", "cursor")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("query"), s("cursor")],
    },
    ActionSpec {
        key: "drive.files.get",
        method: "GET",
        path: "/drive/v3/files/{resource_id}",
        summary: "Read Drive file metadata.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
];
const GOOGLE_DRIVE_KEYS: &[&str] = &["drive.files.list", "drive.files.get"];
pub const GOOGLE_DRIVE_SPEC: ProviderSpec = ProviderSpec {
    slug: "drive",
    origin: Origin::Static("https://www.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GOOGLE_DRIVE_ACTIONS,
    action_keys: GOOGLE_DRIVE_KEYS,
};
const GCAL_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "calendar.events.list",
        method: "GET",
        path: "/calendar/v3/calendars/primary/events",
        summary: "Listed calendar events.",
        path_params: &[],
        query: &[("maxResults", "limit"), ("timeMin", "time_min"), ("timeMax", "time_max")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit"), s("time_min"), s("time_max")],
    },
    ActionSpec {
        key: "calendar.events.create",
        method: "POST",
        path: "/calendar/v3/calendars/primary/events",
        summary: "Created a calendar event.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];
const GCAL_KEYS: &[&str] = &["calendar.events.list", "calendar.events.create"];
pub const GOOGLE_CALENDAR_SPEC: ProviderSpec = ProviderSpec {
    slug: "calendar",
    origin: Origin::Static("https://www.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GCAL_ACTIONS,
    action_keys: GCAL_KEYS,
};
const GPHOTOS_ACTIONS: &[ActionSpec] = &[ActionSpec {
    key: "google_photos.mediaitems.list",
    method: "GET",
    path: "/v1/mediaItems",
    summary: "Listed Google Photos media items.",
    path_params: &[],
    query: &[("pageSize", "limit")],
    body_param: None,
    body_wrapper: None,
    risk: Risk::Low,
    params: &[s("limit")],
}];
const GPHOTOS_KEYS: &[&str] = &["google_photos.mediaitems.list"];
pub const GOOGLE_PHOTOS_SPEC: ProviderSpec = ProviderSpec {
    slug: "google_photos",
    origin: Origin::Static("https://photoslibrary.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GPHOTOS_ACTIONS,
    action_keys: GPHOTOS_KEYS,
};
const GFORMS_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "google_forms.forms.get",
        method: "GET",
        path: "/v1/forms/{resource_id}",
        summary: "Read a Google Form.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "google_forms.responses.list",
        method: "GET",
        path: "/v1/forms/{form_id}/responses",
        summary: "Listed form responses.",
        path_params: &["form_id"],
        query: &[("pageSize", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("form_id"), s("limit")],
    },
];
const GFORMS_KEYS: &[&str] = &["google_forms.forms.get", "google_forms.responses.list"];
pub const GOOGLE_FORMS_SPEC: ProviderSpec = ProviderSpec {
    slug: "google_forms",
    origin: Origin::Static("https://forms.googleapis.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: GFORMS_ACTIONS,
    action_keys: GFORMS_KEYS,
};
const OUTLOOK_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "outlook.messages.list",
        method: "GET",
        path: "/v1.0/me/messages",
        summary: "Listed mailbox messages.",
        path_params: &[],
        query: &[("$top", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "outlook.messages.send",
        method: "POST",
        path: "/v1.0/me/sendMail",
        summary: "Sent a mailbox message.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
];
const OUTLOOK_KEYS: &[&str] = &["outlook.messages.list", "outlook.messages.send"];
pub const OUTLOOK_SPEC: ProviderSpec = ProviderSpec {
    slug: "outlook",
    origin: Origin::Static("https://graph.microsoft.com"),
    auth: AuthStyle::BearerHeaders {
        token_field: "token",
        headers: &[("ConsistencyLevel", "eventual")],
    },
    actions: OUTLOOK_ACTIONS,
    action_keys: OUTLOOK_KEYS,
};
