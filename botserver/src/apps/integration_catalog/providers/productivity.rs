use super::super::actions::{
    ADMIN_ACTIONS, AUTOMATION_ACTIONS, BOOKMARK_ACTIONS, CALENDAR_ACTIONS, DESIGN_ACTIONS,
    FILE_ACTIONS, FORM_ACTIONS, KNOWLEDGE_ACTIONS, MAIL_ACTIONS, MEETING_ACTIONS,
    MESSAGING_ACTIONS, PROJECT_ACTIONS, SIGNATURE_ACTIONS, SPREADSHEET_ACTIONS, TASK_ACTIONS,
};
use super::super::auth::{API_KEY, GOOGLE_OAUTH2, IMAP, MICROSOFT_OAUTH2, OAUTH2, TRELLO_KEY_TOKEN, UNKNOWN};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    provider(
        "slack",
        "Slack",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botintegrations"),
            official_docs: Some("https://api.slack.com/docs"), auth: &OAUTH2,
            actions: MESSAGING_ACTIONS,
        },
    ),
    provider(
        "zoom",
        "Zoom",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botmeet"),
            official_docs: Some("https://developers.zoom.us/docs/api/"), auth: &OAUTH2,
            actions: MEETING_ACTIONS,
        },
    ),
    provider(
        "calendly",
        "Calendly",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: None,
            official_docs: Some("https://developer.calendly.com/api-docs"), auth: &OAUTH2,
            actions: CALENDAR_ACTIONS,
        },
    ),
    provider(
        "canva",
        "Canva",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::NewApp, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://www.canva.dev/docs/connect/"), auth: &OAUTH2,
            actions: DESIGN_ACTIONS,
        },
    ),
    provider(
        "clickup",
        "ClickUp",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.clickup.com/reference/getstarted"),
            auth: &OAUTH2, actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "monday",
        "monday",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.monday.com/api-reference/docs"), auth: &OAUTH2,
            actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "todoist",
        "Todoist",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.todoist.com/rest/v2/"), auth: &OAUTH2,
            actions: TASK_ACTIONS,
        },
    ),
    provider(
        "google_forms",
        "Google Forms",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/forms/api/guides"),
            auth: &GOOGLE_OAUTH2, actions: FORM_ACTIONS,
        },
    ),
    provider(
        "box",
        "Box",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.box.com/reference/"), auth: &OAUTH2,
            actions: FILE_ACTIONS,
        },
    ),
    provider(
        "docusign",
        "DocuSign",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.docusign.com/docs/esign-rest-api/"),
            auth: &OAUTH2, actions: SIGNATURE_ACTIONS,
        },
    ),
    provider(
        "arena",
        "Arena",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "n8n",
        "n8n",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botintegrations"),
            official_docs: Some("https://docs.n8n.io/api/"), auth: &API_KEY,
            actions: AUTOMATION_ACTIONS,
        },
    ),
    provider(
        "google_workspace_admin",
        "Google Workspace Admin",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/admin-sdk"), auth: &GOOGLE_OAUTH2,
            actions: ADMIN_ACTIONS,
        },
    ),
    provider(
        "trello",
        "Trello",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.atlassian.com/cloud/trello/rest/"),
            auth: &TRELLO_KEY_TOKEN, actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "confluence",
        "Confluence",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.atlassian.com/cloud/confluence/rest/"),
            auth: &OAUTH2, actions: KNOWLEDGE_ACTIONS,
        },
    ),
    provider(
        "typeform",
        "Typeform",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.typeform.com/"), auth: &OAUTH2,
            actions: FORM_ACTIONS,
        },
    ),
    provider(
        "jotform",
        "Jotform",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://api.jotform.com/docs/"), auth: &API_KEY,
            actions: FORM_ACTIONS,
        },
    ),
    provider(
        "cal_com",
        "Cal.com",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://cal.com/docs/api-reference/v2/introduction"),
            auth: &API_KEY, actions: CALENDAR_ACTIONS,
        },
    ),
    provider(
        "smartsheet",
        "Smartsheet",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.smartsheet.com/api/smartsheet/openapi"),
            auth: &OAUTH2, actions: SPREADSHEET_ACTIONS,
        },
    ),
    provider(
        "basecamp",
        "Basecamp",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://github.com/basecamp/bc3-api"), auth: &OAUTH2,
            actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "motion",
        "Motion",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.usemotion.com/docs/motion-rest-api/44e01dba38158-getting-started"),
            auth: &API_KEY, actions: TASK_ACTIONS,
        },
    ),
    provider(
        "grain",
        "Grain",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: MEETING_ACTIONS,
        },
    ),
    provider(
        "sharepoint",
        "SharePoint",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botm365"),
            official_docs: Some("https://learn.microsoft.com/graph/api/resources/sharepoint"),
            auth: &MICROSOFT_OAUTH2, actions: FILE_ACTIONS,
        },
    ),
    provider(
        "descript",
        "Descript",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: DESIGN_ACTIONS,
        },
    ),
    provider(
        "raindrop",
        "Raindrop",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.raindrop.io/"), auth: &OAUTH2,
            actions: BOOKMARK_ACTIONS,
        },
    ),
    provider(
        "imap",
        "IMAP",
        ProviderSpec {
            category: Category::Productivity, strategy: Strategy::Improve,
            status: Status::Unsupported, priority: Priority::Must, module: Some("botemail"),
            official_docs: Some("https://www.rfc-editor.org/rfc/rfc9051"), auth: &IMAP,
            actions: MAIL_ACTIONS,
        },
    ),
];
