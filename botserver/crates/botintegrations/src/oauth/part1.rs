use super::*;

pub(crate) fn provider_config(slug: &str) -> Option<OAuthProviderConfig> {
    let (authorize_url, token_url, scopes, basic_client_auth) = match slug {
        "hubspot" => (
            "https://app.hubspot.com/oauth/authorize",
            "https://api.hubapi.com/oauth/v1/token",
            "crm.objects.contacts.write crm.objects.deals.read",
            false,
        ),
        "intercom" => (
            "https://app.intercom.io/oauth",
            "https://api.intercom.io/auth/eagle/token",
            "",
            false,
        ),
        "todoist" => (
            "https://todoist.com/oauth/authorize",
            "https://todoist.com/oauth/access_token",
            "data:read_write",
            false,
        ),
        "zoom" => (
            "https://zoom.us/oauth/authorize",
            "https://zoom.us/oauth/token",
            "",
            true,
        ),
        "notion" => (
            "https://api.notion.com/v1/oauth/authorize",
            "https://api.notion.com/v1/oauth/token",
            "",
            true,
        ),
        "google_cloud" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/cloud-platform.readonly",
            false,
        ),
        "google_workspace_admin" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/admin.directory.user https://www.googleapis.com/auth/admin.directory.group.readonly",
            false,
        ),
        "drive" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/drive.readonly",
            false,
        ),
        "calendar" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/calendar.events",
            false,
        ),
        "google_photos" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/photoslibrary.readonly",
            false,
        ),
        "google_forms" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/forms.body",
            false,
        ),
        "outlook" | "outlook_calendar" | "onedrive" | "sharepoint" => (
            "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
            "https://login.microsoftonline.com/common/oauth2/v2.0/token",
            "offline_access https://graph.microsoft.com/.default",
            true,
        ),
        "xero_accounting" => (
            "https://identity.xero.com/connect/authorize",
            "https://identity.xero.com/connect/token",
            "openid profile email accounting.transactions accounting.contacts",
            true,
        ),
        "highlevel" => (
            "https://marketplace.gohighlevel.com/oauth/chooselocation",
            "https://services.leadconnectorhq.com/oauth/token",
            "",
            true,
        ),
        "docusign" => (
            "https://account.docusign.com/oauth/auth",
            "https://account-d.docusign.com/oauth/token",
            "signature impersonation",
            true,
        ),
        "salesforce" => (
            "https://login.salesforce.com/services/oauth2/authorize",
            "https://login.salesforce.com/services/oauth2/token",
            "full refresh_token",
            false,
        ),
        "quickbooks" => (
            "https://appcenter.intuit.com/connect/oauth2",
            "https://oauth.platform.intuit.com/oauth2/v1/tokens/bearer",
            "com.intuit.quickbooks.accounting",
            false,
        ),
        "box" => (
            "https://account.box.com/api/oauth2/authorize",
            "https://api.box.com/oauth2/token",
            "",
            false,
        ),
        "airtable" => (
            "https://airtable.com/oauth2/v1/authorize",
            "https://airtable.com/oauth2/v1/token",
            "data.records:read data.records:write schema.bases:read",
            true,
        ),
        "webflow" => (
            "https://webflow.com/oauth/authorize",
            "https://api.webflow.com/oauth/access_token",
            "",
            false,
        ),
        "blogger" => (
            "https://accounts.google.com/o/oauth2/v2/auth",
            "https://oauth2.googleapis.com/token",
            "https://www.googleapis.com/auth/blogger",
            false,
        ),
        "confluence" => (
            "https://auth.atlassian.com/authorize",
            "https://auth.atlassian.com/oauth/token",
            "read:confluence-content.all write:confluence-content offline_access",
            false,
        ),
        "basecamp" => (
            "https://launchpad.37signals.com/authorization/new",
            "https://launchpad.37signals.com/authorization/token",
            "",
            false,
        ),
        "fitbit" => (
            "https://www.fitbit.com/oauth2/authorize",
            "https://api.fitbit.com/oauth2/token",
            "activity heartrate location nutrition profile settings sleep social weight",
            true,
        ),
        "twitch" => (
            "https://id.twitch.tv/oauth2/authorize",
            "https://id.twitch.tv/oauth2/token",
            "",
            true,
        ),
        "canva" => (
            "https://www.canva.com/api/oauth/authorize",
            "https://api.canva.com/rest/v1/oauth/token",
            "asset:read asset:write",
            true,
        ),
        "clickfunnels" => (
            "https://accounts.myclickfunnels.com/api/authorize",
            "https://api.myclickfunnels.com/api/oauth/token",
            "",
            false,
        ),
        // Instagram publishing runs on the Instagram Graph API through a
        // Meta app, so the consent screen is Facebook Login. Both entries
        // share one Meta app; scopes are comma-separated per Meta's spec.
        "instagram" => (
            "https://www.facebook.com/v21.0/dialog/oauth",
            "https://graph.facebook.com/v21.0/oauth/access_token",
            "instagram_basic,instagram_content_publish,pages_show_list,pages_read_engagement,business_management",
            false,
        ),
        "facebook_pages" => (
            "https://www.facebook.com/v21.0/dialog/oauth",
            "https://graph.facebook.com/v21.0/oauth/access_token",
            "pages_show_list,pages_read_engagement,pages_manage_posts",
            false,
        ),
        _ => return None,
    };
    Some(OAuthProviderConfig {
        authorize_url,
        token_url,
        scopes,
        basic_client_auth,
    })
}
pub(super) fn signing_key() -> Result<String, Response> {
    let token = botcoresecrets::internal_api_token();
    if token.trim().is_empty() {
        return Err(error_response(StatusCode::SERVICE_UNAVAILABLE, IntegrationError::Validation("oauth signing key is not configured".to_string())));
    }
    Ok(token)
}
pub(super) fn sign(payload: &str, key: &str) -> Result<String, Response> {
    // HMAC-SHA256 accepts a key of any length, so a construction failure means
    // the crypto backend is unusable; surface it instead of panicking.
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|e| {
        log::error!("oauth state signing key rejected by HMAC: {e}");
        error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            IntegrationError::Validation("oauth state signing unavailable".to_string()),
        )
    })?;
    mac.update(payload.as_bytes());
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes()))
}
pub(super) fn error_response(status: StatusCode, error: IntegrationError) -> Response {
    (status, Json(json!({ "detail": error.to_string() }))).into_response()
}
pub(super) fn parse_uuid(value: &str) -> Result<Uuid, Response> {
    Uuid::parse_str(value)
        .map_err(|_| error_response(StatusCode::BAD_REQUEST, IntegrationError::Validation("bot_id must be a UUID".to_string())))
}
pub(super) fn urlencode(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}
/// Providers whose consent and token endpoints belong to Meta. Their code
/// exchange returns a token that expires in about an hour, which is shorter
/// than any publish workflow, so [`callback`] upgrades it in place.
pub(super) fn is_meta_provider(provider: &str) -> bool {
    matches!(provider, "instagram" | "facebook_pages")
}
/// Replaces a short-lived Meta user token with the long-lived one. Returns
/// `None` on any failure so the caller can keep the short-lived token rather
/// than failing the whole connect flow.
pub(super) async fn exchange_long_lived_meta_token(
    http: &reqwest::Client,
    config: &OAuthProviderConfig,
    client_id: &str,
    client_secret: &str,
    short_lived: &str,
) -> Option<(String, Option<i64>)> {
    let response = http
        .get(config.token_url)
        .query(&[
            ("grant_type", "fb_exchange_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("fb_exchange_token", short_lived),
        ])
        .send()
        .await
        .map_err(|error| log::warn!("meta long-lived token exchange failed: {error}"))
        .ok()?;
    if !response.status().is_success() {
        log::warn!(
            "meta long-lived token exchange returned {}",
            response.status()
        );
        return None;
    }
    let body: Value = response.json().await.ok()?;
    let token = body.get("access_token").and_then(Value::as_str)?;
    if token.trim().is_empty() {
        return None;
    }
    Some((
        token.to_string(),
        body.get("expires_in").and_then(Value::as_i64),
    ))
}
pub(super) fn request_base(headers: &axum::http::HeaderMap) -> Option<String> {
    let host = headers
        .get(header::HOST)?
        .to_str()
        .ok()?
        .to_string();
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("https");
    Some(format!("{scheme}://{host}"))
}
