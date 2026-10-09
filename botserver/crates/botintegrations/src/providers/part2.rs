use super::*;

/// Replaces embedded token-shaped substrings (for example `sk_test_...` or
/// `ghp_...` values echoed by a provider) with a fixed marker so raw
/// credential material can never reach chat or API payloads.
pub(super) fn scrub_embedded_tokens(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    loop {
        let mut replaced = false;
        for start in 0..rest.len() {
            if !rest.is_char_boundary(start) {
                continue;
            }
            for prefix in VALUE_TOKEN_PREFIXES {
                if !rest[start..].starts_with(prefix) {
                    continue;
                }
                let after_prefix = &rest[start + prefix.len()..];
                let body_len: usize = after_prefix
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
                    .map(char::len_utf8)
                    .sum();
                if body_len < MIN_TOKEN_BODY_LEN {
                    continue;
                }
                result.push_str(&rest[..start]);
                result.push_str("[redacted]");
                rest = &after_prefix[body_len..];
                replaced = true;
                break;
            }
            if replaced {
                break;
            }
        }
        if !replaced {
            break;
        }
    }
    result.push_str(rest);
    result
}
pub(super) fn redact_string_value(value: &str) -> Value {
    Value::String(scrub_embedded_tokens(value))
}
/// Recursively strips keys whose names look like credential material and
/// scrubs embedded token-shaped values, so a hostile provider response can
/// never smuggle secrets into chat or API payloads. S3 object keys are
/// legitimate outcome data and are preserved.
pub fn redact_credentials(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut cleaned = serde_json::Map::new();
            for (key, item) in map {
                if outcome_key_is_sensitive(key) {
                    continue;
                }
                cleaned.insert(key.clone(), redact_credentials(item));
            }
            Value::Object(cleaned)
        }
        Value::Array(items) => Value::Array(items.iter().map(redact_credentials).collect()),
        Value::String(text) => redact_string_value(text),
        other => other.clone(),
    }
}
/// Adapters available in this build. Slice 1 shipped AWS; slice 2 added the
/// GitHub and Stripe JSON REST adapters; the generic declarative engine now
/// serves token/key/basic providers from static specifications.
pub fn registry() -> Vec<Arc<dyn ProviderAdapter>> {
    vec![
        Arc::new(aws::AwsAdapter),
        Arc::new(github::GithubAdapter),
        Arc::new(stripe::StripeAdapter),
        Arc::new(generic::GenericAdapter::new(&generic::commerce::PRINTFUL_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::crm_ops::SALESFORCE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::crm_ops::HIGHLEVEL_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::crm_ops::DOCUSIGN_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::crm_ops::ATTIO_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::crm_ops::CANVA_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::DATABRICKS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::DEVIN_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::HEX_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::N8N_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::POSTMARK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::UPSTASH_REDIS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::NETLIFY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devops::WEBFLOW_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::AHREFS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::ALGOLIA_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::CLOUDFLARE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::CURSOR_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::HUGGING_FACE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::RESEND_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::SUPABASE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::developer::VERCEL_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::COINBASE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::MERCURY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::MOONPAY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::PADDLE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::WISE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finance::QUICKBOOKS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::LAST_FM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::LUMA_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::PHILIPS_HUE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::READWISE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::RAINDROP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::lifestyle::FITBIT_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::AMPLITUDE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::GRAFANA_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::POSTHOG_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::SENTRY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::SPLUNK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::observability::YOUTUBE_ANALYTICS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::payments::SQUARE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::payments::PAYPAL_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::payments::XERO_ACCOUNTING_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::payments::YNAB_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::payments::RAMP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::CAL_COM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::JOTFORM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::MOTION_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::TRELLO_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::TODOIST_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::NOTION_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::CALENDLY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::GOOGLE_DRIVE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::GOOGLE_CALENDAR_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::GOOGLE_PHOTOS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::GOOGLE_FORMS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::OUTLOOK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::OUTLOOK_CALENDAR_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::ONEDRIVE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::SHAREPOINT_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::productivity::CONFLUENCE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::recruiting::GREENHOUSE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::recruiting::CRUNCHBASE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::small_business::BIGCOMMERCE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::small_business::DHL_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::small_business::SQUARESPACE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::small_business::WHOP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::small_business::WOOCOMMERCE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::BEEHIIV_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::LEMLIST_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::LOOPS_SO_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::MAILCHIMP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::PHANTOMBUSTER_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::ZOOM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_messaging::INTERCOM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::SLACK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::REDDIT_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::MASTODON_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::X_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::PINTEREST_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::SPOTIFY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::NEWSCATCHER_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::BLOGGER_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::social_platforms::FACEBOOK_PAGES_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::APOLLO_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::CANNY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::STREAK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::ZENDESK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::HUBSPOT_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::startups::CLICKFUNNELS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::SMARTSHEET_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::CLICKUP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::TYPEFORM_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::DROPBOX_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::BOX_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::workdocs::AIRTABLE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::MINTLIFY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::LOGFIRE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::GRAIN_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::DESCRIPT_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::GWSADMIN_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::devplatform::GCLOUD_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::SNOWFLAKE_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::ROBINHOOD_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::EXPENSIFY_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::LINKEDIN_ADS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::GOOGLE_ADS_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::LIGHTSPEED_X_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::community::EIGHT_SLEEP_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::community::MOLTBOOK_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::community::ARENA_SPEC)),
        Arc::new(generic::GenericAdapter::new(&generic::finplatform::RIPPLING_SPEC)),
        Arc::new(instagram::InstagramAdapter),
        Arc::new(plain::PlainAdapter),
        Arc::new(ashby::AshbyAdapter),
        Arc::new(bluesky::BlueskyAdapter),
        Arc::new(carriers::CarriersAdapter::ups()),
        Arc::new(carriers::CarriersAdapter::fedex()),
        Arc::new(monday::MondayAdapter),
        Arc::new(linear::LinearAdapter),
    ]
}
/// Implemented action keys for `provider`, or an empty slice when the
/// provider has no adapter. Plain string lists keep this helper usable from
/// crates that must not depend on axum types.
pub fn implemented_action_names(provider: &str) -> &'static [&'static str] {
    registry()
        .iter()
        .find(|adapter| adapter.provider() == provider)
        .map(|adapter| adapter.implemented_actions())
        .unwrap_or(&[])
}
/// LLM-safe action metadata for `provider`, filtered to actions that are
/// both implemented and chat-executable (see
/// [`ProviderAdapter::safe_action_catalog`]). Names use the exact catalog
/// action key accepted by the `integrations.invoke` command, so the block is
/// directly actionable. Returns an empty vector for unknown providers.
pub fn llm_safe_actions(provider: &str) -> Vec<LlmSafeAction> {
    registry()
        .into_iter()
        .find(|adapter| adapter.provider() == provider)
        .map(|adapter| adapter.safe_action_catalog())
        .unwrap_or_default()
}
/// Resolves the active connection, loads its credentials strictly from Vault
/// and executes `provider.action` through the registered adapter.
///
/// The full tenant scope (org, branch, bot, owner) is enforced by the
/// connection lookup itself; no caller-supplied scope component is trusted.
pub async fn invoke_registered(
    state: &IntegrationState,
    scope: &ConnectionScope,
    provider: &str,
    action: &str,
    params: &Value,
) -> Result<ActionOutcome, String> {
    let adapter = registry()
        .into_iter()
        .find(|candidate| candidate.provider() == provider)
        .ok_or_else(|| ERR_UNKNOWN_ACTION.to_string())?;

    // OneDrive and Google Drive read the shared external-drive mirror: one
    // account, connected once from Drive -> External, native delta sync and
    // per-branch isolation. Resolving them through a per-bot Vault connection
    // would create a second token store for the same files, one that never
    // advances a cursor and therefore re-lists the whole account per call.
    if let Some(mirrored) =
        crate::external_mirror::invoke(state, scope, provider, action, params).await
    {
        return mirrored;
    }

    if !adapter.implemented_actions().contains(&action) {
        return Err(ERR_ACTION_NOT_AVAILABLE.to_string());
    }
    if !params.is_object() {
        return Err(format!(
            "{ERR_INVALID_REQUEST}: params must be a JSON object"
        ));
    }

    let row = {
        let mut conn = state
            .pool
            .get()
            .map_err(|_| ERR_STORAGE_UNAVAILABLE.to_string())?;
        repository::find_active_by_provider(&mut conn, scope, provider)
            .map_err(|error| {
                log::error!("action connection lookup failed for {provider}: {error:?}");
                ERR_STORAGE_UNAVAILABLE.to_string()
            })?
            .ok_or_else(|| ERR_NO_ACTIVE_CONNECTION.to_string())?
    };

    let credentials = state
        .vault
        .load_strict(&row.vault_path)
        .await
        .map_err(|error| {
            log::error!("vault strict load failed before {provider}.{action}: {error:?}");
            ERR_VAULT_UNAVAILABLE.to_string()
        })?;

    let outcome = adapter
        .invoke(action, &credentials, params)
        .await
        .inspect_err(|error| {
            log::warn!("provider action {provider}.{action} rejected: {error}");
        })?;

    Ok(ActionOutcome {
        summary: outcome.summary,
        data: redact_credentials(&outcome.data),
        truncated: outcome.truncated,
    })
}
