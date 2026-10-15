use super::super::actions::{
    ANALYTICS_ACTIONS, CALENDAR_ACTIONS, CONTENT_ACTIONS, MARKETING_ACTIONS, MESSAGING_ACTIONS,
    SOCIAL_ACTIONS,
};
use super::super::auth::{
    API_KEY, GOOGLE_OAUTH2, INSTAGRAM_OAUTH2, OAUTH2, TELEGRAM, TOKEN, UNKNOWN, UNSUPPORTED,
    WHATSAPP,
};
use super::super::types::{Category, Priority, ProviderSeed, Status, Strategy};
use super::{ProviderSpec, provider};

pub(super) const PROVIDERS: &[ProviderSeed] = &[
    // #1424/#1427 — Telegram and WhatsApp are live channel adapters
    // (`bottelegram`/`botwhatsapp`): the seeds give them the same vault-backed
    // credential setter as every other provider (Sources UI + chat), so the
    // channel secrets (bot token, webhook secret, access token, verify
    // token, phone number id) are settable without config.csv.
    provider(
        "telegram",
        "Telegram",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Must, module: Some("bottelegram"),
            official_docs: Some("https://core.telegram.org/bots/api"), auth: &TELEGRAM,
            actions: MESSAGING_ACTIONS,
        },
    ),
    provider(
        "whatsapp",
        "WhatsApp Business",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Must, module: Some("botwhatsapp"),
            official_docs: Some("https://developers.facebook.com/docs/whatsapp/cloud-api"),
            auth: &WHATSAPP, actions: MESSAGING_ACTIONS,
        },
    ),
    // Instagram is a live adapter (`botintegrations::providers::instagram`),
    // so the seed flips llm_available while `action_is_implemented` keeps the
    // per-action truth in sync with the registered adapter. Status stays
    // Partial because the Graph API exposes no media search and no media
    // delete for Instagram, so only list/get/publish execute today.
    ProviderSeed {
        llm_available: true,
        ..provider(
            "instagram",
            "Instagram",
            ProviderSpec {
                category: Category::SocialMessaging, strategy: Strategy::Integrate,
                status: Status::Partial, priority: Priority::Must, module: Some("botinstagram"),
                official_docs: Some("https://developers.facebook.com/docs/instagram-platform"),
                auth: &INSTAGRAM_OAUTH2, actions: SOCIAL_ACTIONS,
            },
        )
    },
    provider(
        "facebook_pages",
        "Facebook Pages",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Must, module: Some("botsocial"),
            official_docs: Some("https://developers.facebook.com/docs/pages-api/"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "pinterest",
        "Pinterest",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.pinterest.com/docs/api/v5/"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "reddit",
        "Reddit",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://www.reddit.com/dev/api/"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "twitch",
        "Twitch",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://dev.twitch.tv/docs/api/"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "bluesky",
        "Bluesky",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.bsky.app/docs/api/at-protocol-xrpc-api"),
            auth: &TOKEN, actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "mastodon",
        "Mastodon",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://docs.joinmastodon.org/api/"), auth: &OAUTH2,
            actions: SOCIAL_ACTIONS,
        },
    ),
    provider(
        "signal",
        "Signal",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Verify,
            status: Status::Unsupported, priority: Priority::Nice, module: None,
            official_docs: None, auth: &UNSUPPORTED, actions: MESSAGING_ACTIONS,
        },
    ),
    provider(
        "youtube_analytics",
        "YouTube Analytics",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Unsupported, priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/youtube/analytics"),
            auth: &GOOGLE_OAUTH2, actions: ANALYTICS_ACTIONS,
        },
    ),
    provider(
        "linkedin_ads",
        "LinkedIn Ads",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Unsupported, priority: Priority::Nice, module: None,
            official_docs: Some("https://learn.microsoft.com/linkedin/marketing/"), auth: &OAUTH2,
            actions: MARKETING_ACTIONS,
        },
    ),
    provider(
        "luma",
        "Luma",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None, official_docs: None,
            auth: &API_KEY, actions: CALENDAR_ACTIONS,
        },
    ),
    provider(
        "blogger",
        "Blogger",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.google.com/blogger/docs/3.0/getting_started"),
            auth: &GOOGLE_OAUTH2, actions: CONTENT_ACTIONS,
        },
    ),
    provider(
        "beehiiv",
        "Beehiiv",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Integrate,
            status: Status::Built, priority: Priority::Nice, module: None,
            official_docs: Some("https://developers.beehiiv.com/api-reference"), auth: &API_KEY,
            actions: CONTENT_ACTIONS,
        },
    ),
    provider(
        "moltbook",
        "Moltbook",
        ProviderSpec {
            category: Category::SocialMessaging, strategy: Strategy::Verify, status: Status::Built,
            priority: Priority::Nice, module: None, official_docs: None, auth: &UNKNOWN,
            actions: CONTENT_ACTIONS,
        },
    ),
];
