//! Provider adapter plane for real integration action execution (#950 slice 1).
//!
//! An adapter turns a Vault credential envelope plus validated parameters
//! into live provider API calls. The registry below is the single source of
//! truth for which provider/action pairs execute today; the integration
//! catalog reads it through [`implemented_action_names`] so the advertised
//! surface can never drift ahead of the implementations.
//!
//! Security contract:
//! - credentials load strictly from Vault immediately before invocation;
//! - every outcome passes through [`redact_credentials`] before returning;
//! - error strings are static sentinels safe to map onto HTTP responses;
//!   provider-side details stay in the server log only.

pub(crate) use std::future::Future;
pub(crate) use std::pin::Pin;
pub(crate) use std::sync::Arc;
pub(crate) use serde::Serialize;
pub(crate) use serde_json::Value;
pub(crate) use crate::repository;
pub(crate) use crate::scope::ConnectionScope;
pub(crate) use crate::state::IntegrationState;

pub mod aws;
pub mod ashby;
pub mod carriers;
pub mod bluesky;
pub mod generic;
pub mod linear;
pub mod monday;
pub mod plain;
pub mod github;
pub mod instagram;
pub mod rest_client;
pub mod stripe;

/// Unknown provider or action name; maps to HTTP 404.
pub const ERR_UNKNOWN_ACTION: &str = "unknown_action";
/// Known catalog action without a backing adapter implementation; HTTP 404.
pub const ERR_ACTION_NOT_AVAILABLE: &str = "action_not_available";
/// No active, non-revoked connection for scope and provider; HTTP 502.
pub const ERR_NO_ACTIVE_CONNECTION: &str = "connection_not_found";
/// Vault failure while loading credentials; maps to HTTP 503.
pub const ERR_VAULT_UNAVAILABLE: &str = "vault_unavailable";
/// Database failure while resolving the connection; maps to HTTP 502.
pub const ERR_STORAGE_UNAVAILABLE: &str = "storage_unavailable";
/// Prefix for parameter or credential-shape validation failures; HTTP 400.
pub const ERR_INVALID_REQUEST: &str = "invalid_request";

/// Outcome of one provider action invocation, safe for chat and API output.
#[derive(Debug, Clone, Serialize)]
pub struct ActionOutcome {
    pub summary: String,
    pub data: Value,
    pub truncated: bool,
}

/// One parameter of an LLM-safe action. Names and types only - parameter
/// values never appear here, so nothing credential-shaped can leak.
#[derive(Debug, Clone, Serialize)]
pub struct LlmSafeParam {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub required: bool,
}

/// Chat-safe action metadata for @integration mention blocks (#939 phase D).
///
/// Mirrors the catalog's `LlmAction` shape without any authentication
/// fields: no auth method, field or Vault path ever enters this struct,
/// because it is rendered verbatim into LLM system prompts.
#[derive(Debug, Clone, Serialize)]
pub struct LlmSafeAction {
    pub name: String,
    pub summary: String,
    pub params: Vec<LlmSafeParam>,
    pub risk: String,
    pub requires_approval: bool,
}

/// A provider adapter executing actions against a live external service.
pub trait ProviderAdapter: Send + Sync {
    fn provider(&self) -> &'static str;

    fn implemented_actions(&self) -> &'static [&'static str];

    /// Chat-surface action metadata used by the @integration mention prompt
    /// block. Every entry must be simultaneously implemented by this adapter
    /// and exposed on the chat surface (catalog `surfaces` contains Chat), so
    /// the "implemented && chat-executable" filter is enforced at declaration
    /// time rather than at render time. Adapters that have not opted into the
    /// chat surface return an empty vector via the default implementation.
    fn safe_action_catalog(&self) -> Vec<LlmSafeAction> {
        Vec::new()
    }

    fn invoke<'a>(
        &'a self,
        action: &'a str,
        credentials: &'a Value,
        params: &'a Value,
    ) -> Pin<Box<dyn Future<Output = Result<ActionOutcome, String>> + Send + 'a>>;
}

/// Token-shaped value prefixes that must never survive into outcomes even
/// when a hostile provider echoes credential material inside ordinary
/// string fields. GitHub personal access tokens and Stripe secret keys use
/// these well-known markers.
const VALUE_TOKEN_PREFIXES: &[&str] = &[
    "github_pat_",
    "sk_live_",
    "sk_test_",
    "rk_live_",
    "rk_test_",
    "whsec_",
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
];

/// Minimum number of body characters after a token prefix before the value
/// is treated as live credential material rather than prose.
const MIN_TOKEN_BODY_LEN: usize = 8;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redact_strips_credential_keys_at_any_depth() {
        let hostile = json!({
            "account": "123",
            "nested": {
                "access_key_id": "AKIAHOSTILE",
                "session_token": "tok",
                "my_secret_value": "hidden",
                "keep": "visible"
            },
            "list": [{ "secret_access_key": "hidden-too" }],
            "authorization": "Bearer x",
            "password": "p",
            "key": "s3-object-key-is-legit"
        });
        let rendered = redact_credentials(&hostile).to_string();
        assert!(rendered.contains("visible"));
        assert!(rendered.contains("123"));
        assert!(rendered.contains("s3-object-key-is-legit"));
        assert!(!rendered.contains("AKIAHOSTILE"));
        assert!(!rendered.contains("hidden-too"));
        assert!(!rendered.contains("\"token\""));
        assert!(!rendered.to_lowercase().contains("secret"));
        assert!(!rendered.to_lowercase().contains("authorization"));
        assert!(!rendered.to_lowercase().contains("password"));
    }

    #[test]
    fn registry_exposes_aws_with_full_catalog_coverage() {
        let names = implemented_action_names("aws");
        assert_eq!(names.len(), 13);
        assert_eq!(names, crate::providers::aws::AWS_IMPLEMENTED_ACTIONS);
        assert!(implemented_action_names("nonexistent").is_empty());
    }

    #[test]
    fn registry_exposes_github_and_stripe_action_sets() {
        let github_names = implemented_action_names("github");
        assert_eq!(
            github_names,
            crate::providers::github::GITHUB_IMPLEMENTED_ACTIONS
        );
        assert_eq!(github_names.len(), 9);

        let stripe_names = implemented_action_names("stripe");
        assert_eq!(
            stripe_names,
            crate::providers::stripe::STRIPE_IMPLEMENTED_ACTIONS
        );
        assert_eq!(stripe_names.len(), 10);
    }

    #[test]
    fn embedded_token_values_are_scrubbed_from_outcomes() {
        // Synthetic fixtures are assembled from fragments so the source file
        // never contains a contiguous credential-shaped literal (GitHub push
        // protection scans committed text, not runtime values).
        let sk_test = concat!("sk_", "test_51AbcdefGhijklMn");
        let ghp = concat!("ghp_", "ABCDEFGHIJKLMNOPQRSTUVWXYZ12");
        let whsec = concat!("whsec_", "0123456789abcdef");
        let github_pat = concat!("github_pat_", "11AAAAAAA0bbbbbbbbbbCCCCCCCCCC");
        let sk_live = ["sk_", "live_9f8e7d6c5b4a3210fedcba98"].concat();
        let hostile = json!({
            "note": format!("leaked {sk_test} and {ghp}"),
            "webhook": whsec,
            "nested": { "field": format!("prefix {github_pat}") },
            "list": [sk_live],
            "clean": "the quick brown fox"
        });
        let rendered = redact_credentials(&hostile).to_string();
        for fragment in [
            "sk_test_",
            "sk_live_",
            "ghp_",
            "whsec_",
            "github_pat_",
            "[redacted]",
        ] {
            if fragment == "[redacted]" {
                assert!(rendered.contains(fragment));
            } else {
                assert!(!rendered.contains(fragment), "leaked {fragment}");
            }
        }
        assert!(rendered.contains("the quick brown fox"));
    }

    #[test]
    fn unknown_action_is_rejected_before_any_lookup() {
        // Registry-level gating is pure: an unimplemented action key never
        // reaches connection resolution or Vault. Verified through the same
        // check invoke_registered performs first.
        let names = implemented_action_names("aws");
        assert!(!names.contains(&"s3.buckets.delete"));
        assert!(names.contains(&"sts.caller_identity.get"));
    }

    #[test]
    fn llm_safe_actions_cover_implemented_surface_without_secret_material() {
        for provider in ["aws", "github", "stripe"] {
            let actions = llm_safe_actions(provider);
            let names: Vec<&str> = actions.iter().map(|a| a.name.as_str()).collect();
            assert_eq!(
                names.len(),
                implemented_action_names(provider).len(),
                "chat-safe metadata count mismatch for {provider}"
            );
            for key in implemented_action_names(provider) {
                assert!(names.contains(key), "missing chat-safe metadata for {key}");
            }

            let rendered = serde_json::to_string(&actions).expect("serialize in tests");
            for banned in [
                "secret",
                "token",
                "vault",
                "access_key",
                "api_key",
                "password",
                "authorization",
            ] {
                assert!(
                    !rendered.to_lowercase().contains(banned),
                    "chat-safe metadata leaked {banned} for {provider}"
                );
            }
        }
        assert!(llm_safe_actions("nonexistent").is_empty());
    }

    #[test]
    fn registry_registers_every_generic_provider_slug() {
        let slugs = [
            "ahrefs", "airtable", "algolia", "amplitude",
            "apollo", "attio", "beehiiv", "bigcommerce",
            "blogger", "box", "cal_com", "calendly",
            "canny", "canva", "clickfunnels", "clickup",
            "cloudflare", "coinbase", "confluence", "crunchbase",
            "cursor", "databricks", "devin", "dhl",
            "docusign", "dropbox", "facebook_pages", "fitbit",
            "calendar", "drive", "google_forms", "google_photos",
            "grafana", "greenhouse", "hex", "highlevel",
            "hubspot", "hugging_face", "intercom", "jotform",
            "last_fm", "lemlist", "loops_so", "luma",
            "mailchimp", "mastodon", "mercury", "moonpay",
            "motion", "n8n", "netlify", "newscatcher",
            "notion", "onedrive", "outlook", "outlook_calendar",
            "paddle", "paypal", "phantombuster", "philips_hue",
            "pinterest", "posthog", "postmark", "printful",
            "quickbooks", "raindrop", "ramp", "readwise",
            "reddit", "resend", "salesforce", "sentry",
            "sharepoint", "slack", "smartsheet", "splunk",
            "spotify", "square", "squarespace", "streak",
            "supabase", "todoist", "trello", "typeform",
            "upstash_redis", "vercel", "webflow", "whop",
            "wise", "woocommerce", "x", "xero_accounting",
            "ynab", "youtube_analytics", "zendesk", "zoom",
            "plain", "ashby", "bluesky", "instagram",
            "ups", "fedex", "monday", "linear",
        ];
        let registered = registry();
        assert!(registered.len() >= slugs.len());
        for slug in slugs {
            assert!(
                registered.iter().any(|adapter| adapter.provider() == slug),
                "provider {slug} missing from registry"
            );
        }
    }
}

mod part1;
mod part2;

use part1::*;
pub use part2::*;
