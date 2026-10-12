//! Microsoft Graph client for OneDrive.
//!
//! Sync uses the `delta` endpoint, not `children`: the first call walks the
//! tree (`@odata.nextLink` until exhausted) and hands back an
//! `@odata.deltaLink`; every later call replays just the changes since that
//! link. That is what makes this affordable — a `files.list` poll is O(corpus)
//! and burns the org's Graph quota.
//!
//! Parsing is separated from the HTTP calls so it can be unit-tested without a
//! network.

use crate::external::types::{DeltaPage, ExternalFile, SyncError};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

/// Graph endpoint used for the change feed.
const GRAPH_BASE: &str = "https://graph.microsoft.com/v1.0";
/// First call of a delta run (no cursor yet).
const DELTA_ROOT: &str = "https://graph.microsoft.com/v1.0/me/drive/root/delta";
/// Token endpoint (tenant is part of the path).
const TOKEN_PATH: &str = "/oauth2/v2.0/token";

/// A token set as returned by the Microsoft token endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenSet {
    /// Bearer access token.
    pub access_token: String,
    /// Refresh token, when offline access was granted.
    pub refresh_token: Option<String>,
    /// Absolute expiry, derived from `expires_in`.
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

impl TokenResponse {
    fn into_token_set(self, now: DateTime<Utc>) -> TokenSet {
        TokenSet {
            access_token: self.access_token,
            refresh_token: self.refresh_token,
            expires_at: now + chrono::Duration::seconds(self.expires_in.unwrap_or(3600)),
        }
    }
}

/// Build the authorization URL the browser is sent to.
///
/// `state` is the CSRF value minted by the connect handler and echoed back on
/// the callback; it is what binds the callback to this branch and provider.
pub fn authorize_url(tenant_id: &str, client_id: &str, redirect_uri: &str, state: &str) -> String {
    let scopes = "offline_access Files.Read.All User.Read";
    format!(
        "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize\
?client_id={client}&response_type=code&redirect_uri={redirect}&response_mode=query\
&scope={scopes}&state={state}",
        tenant = tenant_id,
        client = percent_encode(client_id),
        redirect = percent_encode(redirect_uri),
        scopes = percent_encode(scopes),
        state = percent_encode(state),
    )
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<TokenSet, SyncError> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("scope", "offline_access Files.Read.All User.Read"),
    ];
    post_token(http, tenant_id, &form).await
}

/// Refresh an expired access token.
pub async fn refresh_token(
    http: &reqwest::Client,
    tenant_id: &str,
    client_id: &str,
    client_secret: &str,
    refresh: &str,
) -> Result<TokenSet, SyncError> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh),
        ("scope", "offline_access Files.Read.All User.Read"),
    ];
    post_token(http, tenant_id, &form).await
}

async fn post_token(
    http: &reqwest::Client,
    tenant_id: &str,
    form: &[(&str, &str)],
) -> Result<TokenSet, SyncError> {
    let url = format!("https://login.microsoftonline.com/{tenant_id}{TOKEN_PATH}");
    let res = http
        .post(&url)
        .form(form)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("token request failed: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        // Entra answers an expired or revoked refresh token with 400/401 and
        // `invalid_grant`; the only recovery is the user reconnecting.
        if status == 400 || status == 401 {
            return Err(SyncError::Auth(truncate(&body)));
        }
        let kind = classify_status(status.as_u16(), &body);
        return Err(kind);
    }
    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| SyncError::Transient(format!("token response unreadable: {e}")))?;
    Ok(parsed.into_token_set(Utc::now()))
}

/// Fetch one page of the change feed. `cursor` is a previously stored
/// `@odata.deltaLink`; `None` starts (or restarts) a full traversal.
pub async fn fetch_page(
    http: &reqwest::Client,
    access_token: &str,
    cursor: Option<&str>,
) -> Result<DeltaPage, SyncError> {
    let url = cursor.unwrap_or(DELTA_ROOT).to_string();
    let res = http
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("delta request failed: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_status(status.as_u16(), &body));
    }
    let json: Value = serde_json::from_str(&body)
        .map_err(|e| SyncError::Transient(format!("delta response unreadable: {e}")))?;
    Ok(parse_delta(&json))
}

/// Download the bytes of one item.
pub async fn download(
    http: &reqwest::Client,
    access_token: &str,
    remote_id: &str,
) -> Result<Vec<u8>, SyncError> {
    let url = format!("{GRAPH_BASE}/me/drive/items/{}/content", percent_encode(remote_id));
    let res = http
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("download request failed: {e}")))?;
    let status = res.status();
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(classify_status(status.as_u16(), &body));
    }
    res.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| SyncError::Transient(format!("download body failed: {e}")))
}

/// Turn a Graph error into the right retry decision.
pub fn classify_status(status: u16, body: &str) -> SyncError {
    if status == 401 || status == 403 {
        // A revoked consent or an expired refresh token: retrying is pointless
        // until the user reconnects.
        return SyncError::Auth(truncate(body));
    }
    if status == 429 {
        return SyncError::RateLimited(None);
    }
    if status >= 500 {
        return SyncError::Transient(format!("graph {status}: {}", truncate(body)));
    }
    SyncError::Permanent(format!("graph {status}: {}", truncate(body)))
}

/// Parse a `delta` response into a page of changes.
///
/// Handles both page shapes: `@odata.nextLink` while the traversal is still
/// running, `@odata.deltaLink` on the page that closes it, and the `deleted`
/// facet for removals.
pub fn parse_delta(json: &Value) -> DeltaPage {
    let mut page = DeltaPage::default();
    if let Some(values) = json.get("value").and_then(Value::as_array) {
        for item in values {
            let Some(id) = item.get("id").and_then(Value::as_str) else {
                continue;
            };
            if item.get("deleted").is_some() {
                page.removed.push(id.to_string());
                continue;
            }
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            if name.is_empty() {
                continue;
            }
            let is_folder = item.get("folder").is_some();
            let path = item
                .get("parentReference")
                .and_then(|p| p.get("path"))
                .and_then(Value::as_str)
                .map(|p| format!("{}/{}", p.trim_matches('/'), name))
                .unwrap_or_else(|| name.clone());
            let mut f = ExternalFile::new(id, name, path, is_folder);
            f.mime_type = item
                .get("file")
                .and_then(|fi| fi.get("mimeType"))
                .and_then(Value::as_str)
                .map(String::from);
            f.size_bytes = item.get("size").and_then(Value::as_i64).unwrap_or(0);
            f.modified_at = item
                .get("lastModifiedDateTime")
                .and_then(Value::as_str)
                .and_then(parse_ts);
            f.revision = item
                .get("eTag")
                .and_then(Value::as_str)
                .map(|s| s.trim_matches(['"', '{', '}']).to_string());
            f.parent_remote_id = item
                .get("parentReference")
                .and_then(|p| p.get("id"))
                .and_then(Value::as_str)
                .map(String::from);
            page.items.push(f);
        }
    }
    page.next_cursor = json
        .get("@odata.nextLink")
        .and_then(Value::as_str)
        .map(String::from);
    page.delta = json
        .get("@odata.deltaLink")
        .and_then(Value::as_str)
        .map(String::from);
    page
}

/// Parse an ISO-8601 timestamp, ignoring anything malformed.
pub(crate) fn parse_ts(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn truncate(body: &str) -> String {
    botlib::text::truncate_with_suffix(body.trim(), 300, "…")
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{:02X}", other),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn delta_page_splits_updates_and_deletions() {
        let json = json!({
            "value": [
                {
                    "id": "01ABC",
                    "name": "invoice.pdf",
                    "size": 2048,
                    "lastModifiedDateTime": "2026-10-01T10:00:00Z",
                    "eTag": "\"tag-1\"",
                    "file": { "mimeType": "application/pdf" },
                    "parentReference": { "id": "root!", "path": "/drive/root:/Reports" }
                },
                { "id": "01GONE", "deleted": { "state": "deleted" } },
                { "name": "no-id" }
            ]
        });
        let page = parse_delta(&json);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.removed, vec!["01GONE".to_string()]);
        let f = &page.items[0];
        assert_eq!(f.remote_id, "01ABC");
        assert_eq!(f.path, "drive/root:/Reports/invoice.pdf");
        assert_eq!(f.size_bytes, 2048);
        assert_eq!(f.mime_type.as_deref(), Some("application/pdf"));
        assert_eq!(f.revision.as_deref(), Some("tag-1"));
        assert!(!f.is_folder);
        assert!(f.modified_at.is_some());
    }

    #[test]
    fn delta_page_recognizes_folders_and_cursors() {
        let paging = json!({
            "@odata.nextLink": "https://graph.microsoft.com/v1.0/me/drive/root/delta?token=2",
            "value": [{ "id": "D1", "name": "Docs", "folder": { "childCount": 3 } }]
        });
        let p = parse_delta(&paging);
        assert!(p.items[0].is_folder);
        assert_eq!(p.items[0].size_bytes, 0);
        assert_eq!(
            p.next_cursor.as_deref(),
            Some("https://graph.microsoft.com/v1.0/me/drive/root/delta?token=2")
        );
        assert!(p.delta.is_none());

        let done = json!({
            "@odata.deltaLink": "https://graph.microsoft.com/v1.0/me/drive/root/delta?token=9",
            "value": []
        });
        let d = parse_delta(&done);
        assert!(d.next_cursor.is_none());
        assert!(d.delta.is_some());
    }

    #[test]
    fn malformed_timestamps_do_not_break_the_page() {
        let json = json!({
            "value": [{ "id": "1", "name": "a.txt", "lastModifiedDateTime": "not-a-date" }]
        });
        let page = parse_delta(&json);
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].modified_at.is_none());
    }

    #[test]
    fn status_classification_drives_retry_behavior() {
        assert!(matches!(classify_status(401, "token expired"), SyncError::Auth(_)));
        assert!(matches!(classify_status(403, "no consent"), SyncError::Auth(_)));
        assert!(matches!(classify_status(429, ""), SyncError::RateLimited(None)));
        assert!(matches!(classify_status(503, "busy"), SyncError::Transient(_)));
        assert!(matches!(classify_status(400, "bad"), SyncError::Permanent(_)));
    }

    #[test]
    fn authorize_url_carries_state_and_offline_scope() {
        let url = authorize_url("tid", "cid", "https://host/cb", "st 1/2");
        assert!(url.starts_with("https://login.microsoftonline.com/tid/oauth2/v2.0/authorize"));
        assert!(url.contains("state=st%201%2F2"));
        assert!(url.contains("scope=offline_access%20Files.Read.All%20User.Read"));
        assert!(url.contains("response_type=code"));
    }

    #[test]
    fn empty_body_parses_to_an_empty_page() {
        let page = parse_delta(&json!({}));
        assert!(page.items.is_empty() && page.removed.is_empty());
        assert!(page.delta.is_none());
    }
}