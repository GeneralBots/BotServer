use super::super::actions::{
    AI_PLATFORM_ACTIONS, ANALYTICS_ACTIONS, AWS_ACTIONS, CLOUD_ACTIONS, CONTENT_ACTIONS,
    DATA_PLATFORM_ACTIONS, DEPLOYMENT_ACTIONS, EMAIL_DELIVERY_ACTIONS, OBSERVABILITY_ACTIONS,
    REPOSITORY_ACTIONS, SEARCH_ACTIONS,
};
use super::super::auth::{API_KEY, AWS, GOOGLE_OAUTH2, OAUTH2, TOKEN, UNKNOWN};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    // GitHub is a live adapter (#950 slice 2); the seed flips
    // llm_available while `action_is_implemented` keeps the per-action truth
    // in sync with botintegrations::providers::registry. Status stays
    // Partial because only the repository action subset is implemented.
    ProviderSeed {
        llm_available: true,
        ..provider(
            "github",
            "GitHub",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Improve, status: Status::Partial,
                priority: Priority::Must, module: Some("botgit"),
                official_docs: Some("https://docs.github.com/en/rest"), auth: &TOKEN,
                actions: REPOSITORY_ACTIONS,
            },
        )
    },
    provider(
        "cursor",
        "Cursor",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::NewApp, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &API_KEY,
            actions: AI_PLATFORM_ACTIONS,
        },
    ),
    // AWS is the first provider with a live adapter (#950 slice 1); the
    // seed flips llm_available while `action_is_implemented` keeps the
    // per-action truth in sync with botintegrations::providers::registry.
    ProviderSeed {
        llm_available: true,
        ..provider(
            "aws",
            "AWS",
            ProviderSpec {
                category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
                priority: Priority::Must, module: Some("botintegrations"),
                official_docs: Some(
                "https://docs.aws.amazon.com/IAM/latest/UserGuide/id_credentials_access-keys.html",
            ),
                auth: &AWS, actions: AWS_ACTIONS,
            },
        )
    },
    provider(
        "google_cloud",
        "Google Cloud",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: None,
            official_docs: Some("https://cloud.google.com/apis/docs/overview"),
            auth: &GOOGLE_OAUTH2, actions: CLOUD_ACTIONS,
        },
    ),
    provider(
        "vercel",
        "Vercel",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://vercel.com/docs/rest-api"), auth: &TOKEN,
            actions: DEPLOYMENT_ACTIONS,
        },
    ),
    provider(
        "supabase",
        "Supabase",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://supabase.com/docs/reference/api/introduction"),
            auth: &API_KEY, actions: DATA_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "netlify",
        "Netlify",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.netlify.com/api-and-cli-guides/api-guides/get-started-with-api/"),
            auth: &OAUTH2, actions: DEPLOYMENT_ACTIONS,
        },
    ),
    provider(
        "cloudflare",
        "Cloudflare",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.cloudflare.com/api/"), auth: &TOKEN,
            actions: CLOUD_ACTIONS,
        },
    ),
    provider(
        "snowflake",
        "Snowflake",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.snowflake.com/en/developer-guide/sql-api/index"),
            auth: &UNKNOWN, actions: DATA_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "databricks",
        "Databricks",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.databricks.com/api/workspace/introduction"),
            auth: &TOKEN, actions: DATA_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "posthog",
        "PostHog",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://posthog.com/docs/api"), auth: &API_KEY,
            actions: ANALYTICS_ACTIONS,
        },
    ),
    provider(
        "amplitude",
        "Amplitude",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://amplitude.com/docs/apis"), auth: &API_KEY,
            actions: ANALYTICS_ACTIONS,
        },
    ),
    provider(
        "sentry",
        "Sentry",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botmonitoring"),
            official_docs: Some("https://docs.sentry.io/api/"), auth: &TOKEN,
            actions: OBSERVABILITY_ACTIONS,
        },
    ),
    provider(
        "splunk",
        "Splunk",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: Some("botmonitoring"),
            official_docs: Some("https://dev.splunk.com/enterprise/docs/devtools/httpeventcollector/"),
            auth: &TOKEN, actions: OBSERVABILITY_ACTIONS,
        },
    ),
    provider(
        "hugging_face",
        "Hugging Face",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://huggingface.co/docs/api-inference/index"), auth: &TOKEN,
            actions: AI_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "resend",
        "Resend",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: Some("botemail"),
            official_docs: Some("https://resend.com/docs/api-reference/introduction"),
            auth: &API_KEY, actions: EMAIL_DELIVERY_ACTIONS,
        },
    ),
    provider(
        "postmark",
        "Postmark",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: Some("botemail"),
            official_docs: Some("https://postmarkapp.com/developer/api/overview"), auth: &TOKEN,
            actions: EMAIL_DELIVERY_ACTIONS,
        },
    ),
    provider(
        "algolia",
        "Algolia",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://www.algolia.com/doc/rest-api/search/"), auth: &API_KEY,
            actions: SEARCH_ACTIONS,
        },
    ),
    provider(
        "upstash_redis",
        "Upstash Redis",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None,
            official_docs: Some("https://upstash.com/docs/redis/features/restapi"), auth: &TOKEN,
            actions: DATA_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "hex",
        "Hex",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &TOKEN,
            actions: DATA_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "mintlify",
        "Mintlify",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &API_KEY,
            actions: CONTENT_ACTIONS,
        },
    ),
    provider(
        "devin",
        "Devin",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &API_KEY,
            actions: AI_PLATFORM_ACTIONS,
        },
    ),
    provider(
        "pydantic_logfire",
        "Pydantic Logfire",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Nice, module: Some("botmonitoring"),
            official_docs: Some("https://logfire.pydantic.dev/docs/how-to-guides/query-api/"),
            auth: &TOKEN, actions: OBSERVABILITY_ACTIONS,
        },
    ),
    provider(
        "grafana",
        "Grafana",
        ProviderSpec {
            category: Category::Developer, strategy: Strategy::Integrate, status: Status::Built,
            priority: Priority::Must, module: Some("botmonitoring"),
            official_docs: Some("https://grafana.com/docs/grafana/latest/developers/http_api/"),
            auth: &TOKEN, actions: OBSERVABILITY_ACTIONS,
        },
    ),
];
