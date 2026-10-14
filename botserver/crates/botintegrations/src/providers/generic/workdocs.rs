use super::helpers::{json, resource_id, s, s_req};
use super::{ActionSpec, AuthStyle, Origin, ProviderSpec, Risk};
const SMARTSHEET_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "smartsheet.spreadsheets.list",
        method: "GET",
        path: "/2.0/sheets",
        summary: "Listed Smartsheet sheets.",
        path_params: &[],
        query: &[("pageSize", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "smartsheet.values.get",
        method: "GET",
        path: "/2.0/sheets/{resource_id}",
        summary: "Read a Smartsheet sheet.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "smartsheet.rows.append",
        method: "POST",
        path: "/2.0/sheets/{resource_id}/rows",
        summary: "Appended rows to a sheet.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("sheet_id"), json("data", true)],
    },
];

const SMARTSHEET_KEYS: &[&str] = &[
    "smartsheet.spreadsheets.list",
    "smartsheet.values.get",
    "smartsheet.rows.append",
];

pub const SMARTSHEET_SPEC: ProviderSpec = ProviderSpec {
    slug: "smartsheet",
    origin: Origin::Static("https://api.smartsheet.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: SMARTSHEET_ACTIONS,
    action_keys: SMARTSHEET_KEYS,
};

const CLICKUP_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "clickup.work_items.list",
        method: "GET",
        path: "/list/{list_id}/task",
        summary: "Listed ClickUp tasks in a list.",
        path_params: &["list_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("list_id")],
    },
    ActionSpec {
        key: "clickup.work_items.create",
        method: "POST",
        path: "/list/{list_id}/task",
        summary: "Created a ClickUp task.",
        path_params: &["list_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("list_id"), json("data", true)],
    },
    ActionSpec {
        key: "clickup.work_items.update",
        method: "PUT",
        path: "/task/{resource_id}",
        summary: "Updated a ClickUp task.",
        path_params: &["resource_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("task_id"), json("data", true)],
    },
    ActionSpec {
        key: "clickup.work_items.delete",
        method: "DELETE",
        path: "/task/{resource_id}",
        summary: "Deleted a ClickUp task.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const CLICKUP_KEYS: &[&str] = &[
    "clickup.work_items.list",
    "clickup.work_items.create",
    "clickup.work_items.update",
    "clickup.work_items.delete",
];

pub const CLICKUP_SPEC: ProviderSpec = ProviderSpec {
    slug: "clickup",
    origin: Origin::Static("https://api.clickup.com/api/v2"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: CLICKUP_ACTIONS,
    action_keys: CLICKUP_KEYS,
};

const TYPEFORM_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "typeform.forms.list",
        method: "GET",
        path: "/forms",
        summary: "Listed Typeform forms.",
        path_params: &[],
        query: &[("page_size", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("limit")],
    },
    ActionSpec {
        key: "typeform.responses.list",
        method: "GET",
        path: "/forms/{form_id}/responses",
        summary: "Listed form responses.",
        path_params: &["form_id"],
        query: &[("page_size", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("form_id"), s("limit")],
    },
    ActionSpec {
        key: "typeform.forms.create",
        method: "POST",
        path: "/forms",
        summary: "Created a Typeform form.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[json("data", true)],
    },
];

const TYPEFORM_KEYS: &[&str] = &[
    "typeform.forms.list",
    "typeform.responses.list",
    "typeform.forms.create",
];

pub const TYPEFORM_SPEC: ProviderSpec = ProviderSpec {
    slug: "typeform",
    origin: Origin::Static("https://api.typeform.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: TYPEFORM_ACTIONS,
    action_keys: TYPEFORM_KEYS,
};

const DROPBOX_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "dropbox.files.list",
        method: "POST",
        path: "/2/files/list_folder",
        summary: "Listed Dropbox folder contents.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", false)],
    },
    ActionSpec {
        key: "dropbox.files.search",
        method: "POST",
        path: "/2/files/search_v2",
        summary: "Searched Dropbox files.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "dropbox.files.get",
        method: "POST",
        path: "/2/files/get_metadata",
        summary: "Read Dropbox file metadata.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Low,
        params: &[json("data", true)],
    },
    ActionSpec {
        key: "dropbox.files.delete",
        method: "POST",
        path: "/2/files/delete_v2",
        summary: "Deleted a Dropbox file or folder.",
        path_params: &[],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::High,
        params: &[json("data", true)],
    },
];

const DROPBOX_KEYS: &[&str] = &[
    "dropbox.files.list",
    "dropbox.files.search",
    "dropbox.files.get",
    "dropbox.files.delete",
];

pub const DROPBOX_SPEC: ProviderSpec = ProviderSpec {
    slug: "dropbox",
    origin: Origin::Static("https://api.dropboxapi.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: DROPBOX_ACTIONS,
    action_keys: DROPBOX_KEYS,
};

const BOX_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "box.files.search",
        method: "GET",
        path: "/2.0/search",
        summary: "Searched Box content.",
        path_params: &[],
        query: &[("query", "query"), ("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("query"), s("limit")],
    },
    ActionSpec {
        key: "box.files.list",
        method: "GET",
        path: "/2.0/folders/{folder_id}/items",
        summary: "Listed Box folder items.",
        path_params: &["folder_id"],
        query: &[("limit", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s("folder_id"), s("limit")],
    },
    ActionSpec {
        key: "box.files.get",
        method: "GET",
        path: "/2.0/files/{resource_id}",
        summary: "Read a Box file.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: resource_id(),
    },
    ActionSpec {
        key: "box.files.delete",
        method: "DELETE",
        path: "/2.0/files/{resource_id}",
        summary: "Deleted a Box file.",
        path_params: &["resource_id"],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::High,
        params: resource_id(),
    },
];

const BOX_KEYS: &[&str] = &[
    "box.files.search",
    "box.files.list",
    "box.files.get",
    "box.files.delete",
];

pub const BOX_SPEC: ProviderSpec = ProviderSpec {
    slug: "box",
    origin: Origin::Static("https://api.box.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: BOX_ACTIONS,
    action_keys: BOX_KEYS,
};

const AIRTABLE_ACTIONS: &[ActionSpec] = &[
    ActionSpec {
        key: "airtable.spreadsheets.list",
        method: "GET",
        path: "/v0/meta/bases",
        summary: "Listed Airtable bases.",
        path_params: &[],
        query: &[],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[],
    },
    ActionSpec {
        key: "airtable.records.list",
        method: "GET",
        path: "/v0/{base_id}/{table_name}",
        summary: "Listed Airtable records.",
        path_params: &["base_id", "table_name"],
        query: &[("pageSize", "limit")],
        body_param: None,
        body_wrapper: None,
        risk: Risk::Low,
        params: &[s_req("base_id"), s_req("table_name"), s("limit")],
    },
    ActionSpec {
        key: "airtable.values.update",
        method: "PATCH",
        path: "/v0/{base_id}/{table_name}/{resource_id}",
        summary: "Updated an Airtable record.",
        path_params: &["base_id", "table_name", "record_id"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("base_id"), s_req("table_name"), s_req("record_id"), json("data", true)],
    },
    ActionSpec {
        key: "airtable.rows.append",
        method: "POST",
        path: "/v0/{base_id}/{table_name}",
        summary: "Created an Airtable record.",
        path_params: &["base_id", "table_name"],
        query: &[],
        body_param: Some("data"),
        body_wrapper: None,
        risk: Risk::Medium,
        params: &[s_req("base_id"), s_req("table_name"), json("data", true)],
    },
];

const AIRTABLE_KEYS: &[&str] = &[
    "airtable.spreadsheets.list",
    "airtable.records.list",
    "airtable.values.update",
    "airtable.rows.append",
];

pub const AIRTABLE_SPEC: ProviderSpec = ProviderSpec {
    slug: "airtable",
    origin: Origin::Static("https://api.airtable.com"),
    auth: AuthStyle::Bearer {
        token_field: "token",
    },
    actions: AIRTABLE_ACTIONS,
    action_keys: AIRTABLE_KEYS,
};
