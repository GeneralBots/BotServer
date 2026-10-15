use super::super::actions::{
    AUTOMATION_ACTIONS, BANKING_ACTIONS, CRM_ACTIONS, FINANCE_OPERATIONS_ACTIONS, HR_ACTIONS,
    MARKETING_ACTIONS, SEO_ACTIONS, SUPPORT_ACTIONS,
};
use super::super::auth::{API_KEY, BASIC, GOOGLE_OAUTH2, OAUTH2, TOKEN, UNKNOWN, ZENDESK_TOKEN};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    provider(
        "apollo",
        "Apollo",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.apollo.io/reference/api-overview"), auth: &API_KEY,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "lemlist",
        "lemlist",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.lemlist.com/"), auth: &API_KEY,
            actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "loops_so",
        "Loops.so",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://loops.so/docs/api-reference/intro"), auth: &API_KEY,
            actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "streak",
        "Streak",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://streak.readme.io/reference/getting-started-with-the-streak-api"),
            auth: &API_KEY, actions: CRM_ACTIONS,
        },
    ),
    provider(
        "ahrefs",
        "Ahrefs",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.ahrefs.com/docs/api/reference/introduction"),
            auth: &TOKEN, actions: SEO_ACTIONS,
        },
    ),
    provider(
        "google_ads",
        "Google Ads",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/google-ads/api/docs/start"),
            auth: &GOOGLE_OAUTH2, actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "mercury",
        "Mercury",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.mercury.com/reference/welcome-to-mercury-api"),
            auth: &TOKEN, actions: BANKING_ACTIONS,
        },
    ),
    provider(
        "ramp",
        "Ramp",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.ramp.com/developer-api/v1/guides/getting-started"),
            auth: &OAUTH2, actions: FINANCE_OPERATIONS_ACTIONS,
        },
    ),
    provider(
        "expensify",
        "Expensify",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: FINANCE_OPERATIONS_ACTIONS,
        },
    ),
    provider(
        "rippling",
        "Rippling",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &OAUTH2,
            actions: HR_ACTIONS,
        },
    ),
    provider(
        "ashby",
        "Ashby",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.ashbyhq.com/docs/authentication"), auth: &BASIC,
            actions: HR_ACTIONS,
        },
    ),
    provider(
        "plain",
        "Plain",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://www.plain.com/docs/api-reference/introduction"),
            auth: &API_KEY, actions: SUPPORT_ACTIONS,
        },
    ),
    provider(
        "salesforce",
        "Salesforce",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Improve, status: Status::Built,
            priority: Priority::Must, module: Some("botintegrations"),
            official_docs: Some("https://developer.salesforce.com/docs/apis"), auth: &OAUTH2,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "hubspot",
        "HubSpot",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botintegrations"),
            official_docs: Some("https://developers.hubspot.com/docs/api/overview"), auth: &OAUTH2,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "mailchimp",
        "Mailchimp",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://mailchimp.com/developer/marketing/api/"), auth: &API_KEY,
            actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "zendesk",
        "Zendesk",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developer.zendesk.com/api-reference/"),
            auth: &ZENDESK_TOKEN, actions: SUPPORT_ACTIONS,
        },
    ),
    provider(
        "intercom",
        "Intercom",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.intercom.com/docs/references/rest-api/api.intercom.io/"),
            auth: &OAUTH2, actions: SUPPORT_ACTIONS,
        },
    ),
    provider(
        "attio",
        "Attio",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.attio.com/rest-api/overview"), auth: &OAUTH2,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "crunchbase",
        "Crunchbase",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://data.crunchbase.com/docs/using-the-api"), auth: &API_KEY,
            actions: CRM_ACTIONS,
        },
    ),
    provider(
        "greenhouse",
        "Greenhouse",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.greenhouse.io/harvest.html"), auth: &BASIC,
            actions: HR_ACTIONS,
        },
    ),
    provider(
        "clickfunnels",
        "ClickFunnels",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &OAUTH2,
            actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "canny",
        "Canny",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.canny.io/api-reference"), auth: &API_KEY,
            actions: SUPPORT_ACTIONS,
        },
    ),
    provider(
        "phantombuster",
        "PhantomBuster",
        ProviderSpec {
            category: Category::Startups, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://hub.phantombuster.com/reference"), auth: &API_KEY,
            actions: AUTOMATION_ACTIONS,
        },
    ),
];
