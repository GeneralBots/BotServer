use super::super::actions::{
    CALENDAR_ACTIONS, FILE_ACTIONS, KNOWLEDGE_ACTIONS, MAIL_ACTIONS, MEDIA_ACTIONS,
    PROJECT_ACTIONS, SOCIAL_ACTIONS, SPREADSHEET_ACTIONS, TASK_ACTIONS,
};
use super::super::auth::{GOOGLE_OAUTH2, MICROSOFT_OAUTH2, OAUTH2};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    provider(
        "calendar",
        "Calendar",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botcalendar"),
            official_docs: Some("https://developers.google.com/calendar/api/guides/overview"),
            auth: &GOOGLE_OAUTH2, actions: CALENDAR_ACTIONS,
        },
    ),
    provider(
        "drive",
        "Drive",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botdrive"),
            official_docs: Some("https://developers.google.com/drive/api/guides/about-sdk"),
            auth: &GOOGLE_OAUTH2, actions: FILE_ACTIONS,
        },
    ),
    provider(
        "sheets",
        "Sheets",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botsheet"),
            official_docs: Some("https://developers.google.com/sheets/api/guides/concepts"),
            auth: &GOOGLE_OAUTH2, actions: SPREADSHEET_ACTIONS,
        },
    ),
    provider(
        "tasks",
        "Tasks",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("bottasks"),
            official_docs: Some("https://developers.google.com/tasks/reference/rest"),
            auth: &GOOGLE_OAUTH2, actions: TASK_ACTIONS,
        },
    ),
    provider(
        "notion",
        "Notion",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botintegrations"),
            official_docs: Some("https://developers.notion.com/reference/intro"), auth: &OAUTH2,
            actions: KNOWLEDGE_ACTIONS,
        },
    ),
    provider(
        "linear",
        "Linear",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.linear.app/docs/graphql/working-with-the-graphql-api"),
            auth: &OAUTH2, actions: PROJECT_ACTIONS,
        },
    ),
    provider(
        "airtable",
        "Airtable",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://airtable.com/developers/web/api/introduction"),
            auth: &OAUTH2, actions: SPREADSHEET_ACTIONS,
        },
    ),
    provider(
        "dropbox",
        "Dropbox",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://www.dropbox.com/developers/documentation/http/documentation"),
            auth: &OAUTH2, actions: FILE_ACTIONS,
        },
    ),
    provider(
        "outlook",
        "Microsoft Outlook",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botm365"),
            official_docs: Some("https://learn.microsoft.com/graph/api/resources/mail-api-overview"),
            auth: &MICROSOFT_OAUTH2, actions: MAIL_ACTIONS,
        },
    ),
    provider(
        "outlook_calendar",
        "MS Outlook Calendar",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botm365"),
            official_docs: Some("https://learn.microsoft.com/graph/api/resources/calendar"),
            auth: &MICROSOFT_OAUTH2, actions: CALENDAR_ACTIONS,
        },
    ),
    provider(
        "onedrive",
        "OneDrive",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botm365"),
            official_docs: Some("https://learn.microsoft.com/graph/onedrive-concept-overview"),
            auth: &MICROSOFT_OAUTH2, actions: FILE_ACTIONS,
        },
    ),
    provider(
        "spotify",
        "Spotify",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.spotify.com/documentation/web-api"),
            auth: &OAUTH2, actions: MEDIA_ACTIONS,
        },
    ),
    provider(
        "x",
        "X (Twitter)",
        ProviderSpec {
            category: Category::Featured, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.x.com/en/docs/x-api"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
];
