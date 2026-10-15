use super::super::actions::{
    BOOKMARK_ACTIONS, FILE_ACTIONS, HEALTH_ACTIONS, MEDIA_ACTIONS, NEWS_ACTIONS, SMART_HOME_ACTIONS,
};
use super::super::auth::{API_KEY, GOOGLE_OAUTH2, OAUTH2, TOKEN, UNSUPPORTED};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    provider(
        "google_photos",
        "Google Photos",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/photos"), auth: &GOOGLE_OAUTH2,
            actions: FILE_ACTIONS,
        },
    ),
    provider(
        "readwise",
        "Readwise",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://readwise.io/api_deets"), auth: &TOKEN,
            actions: BOOKMARK_ACTIONS,
        },
    ),
    provider(
        "fitbit",
        "Fitbit",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://dev.fitbit.com/build/reference/web-api/"), auth: &OAUTH2,
            actions: HEALTH_ACTIONS,
        },
    ),
    provider(
        "philips_hue",
        "Philips Hue",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.meethue.com/develop/hue-api-v2/"),
            auth: &API_KEY, actions: SMART_HOME_ACTIONS,
        },
    ),
    provider(
        "newscatcher",
        "NewsCatcher",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://www.newscatcherapi.com/docs/v3/api-reference/overview"),
            auth: &API_KEY, actions: NEWS_ACTIONS,
        },
    ),
    provider(
        "eight_sleep",
        "Eight Sleep",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Verify, status: Status::Unsupported,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNSUPPORTED,
            actions: HEALTH_ACTIONS,
        },
    ),
    provider(
        "last_fm",
        "Last.fm",
        ProviderSpec {
            category: Category::Lifestyle, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: Some("https://www.last.fm/api"),
            auth: &API_KEY, actions: MEDIA_ACTIONS,
        },
    ),
];
