//! Google Drive client (Drive API v3).
//!
//! Same two-phase shape as OneDrive, different vocabulary: the first sync
//! walks `files.list`, then `changes.getStartPageToken` captures a cursor and
//! every later pass is a `changes.list` replay. The page token is the only
//! state kept between runs.
//!
//! `files.list` is used only for the initial traversal. Polling it on a timer
//! is what gets a Google account rate-limited (`userRateLimitExceeded`) and
//! eventually blocked, so the delta path is the one that runs afterwards.

use crate::external::types::{DeltaPage, ExternalFile, SyncError};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

/// Drive API root.
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3";
/// OAuth endpoints.
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// Folder mime type — the only one that is a directory rather than a blob.
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
/// The field mask keeps responses small; the default projection omits nothing
/// we need but is much larger.
const FILE_FIELDS: &str =
    "nextPageToken,files(id,name,mimeType,size,modifiedTime,version,parents,trashed)";
const CHANGE_FIELDS: &str = "nextPageToken,newStartPageToken,\
changes(fileId,removed,file(id,name,mimeType,size,modifiedTime,version,parents,trashed))";

/// A token set as returned by the Google token endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct TokenSet {
    /// Bearer access token.
    pub access_token: String,
    /// Refresh token (issued because we request `access_type=offline`).
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

/// Build the authorization URL. `access_type=offline` + `prompt=consent` is
/// what makes Google issue a refresh token for a desktop-style client.
pub fn authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    format!(
        "{AUTH_URL}?client_id={client}&redirect_uri={redirect}&response_type=code\
&scope={scope}&access_type=offline&prompt=consent&include_granted_scopes=true&state={state}",
        client = percent_encode(client_id),
        redirect = percent_encode(redirect_uri),
        scope = percent_encode("https://www.googleapis.com/auth/drive.readonly"),
        state = percent_encode(state),
    )
}

/// Exchange an authorization code for tokens.
pub async fn exchange_code(
    http: &reqwest::Client,
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
        ("access_type", "offline"),
    ];
    post_token(http, &form).await
}

/// Refresh an expired access token.
pub async fn refresh_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh: &str,
) -> Result<TokenSet, SyncError> {
    let form = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh),
    ];
    post_token(http, &form).await
}

async fn post_token(
    http: &reqwest::Client,
    form: &[(&str, &str)],
) -> Result<TokenSet, SyncError> {
    let res = http
        .post(TOKEN_URL)
        .form(form)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("token request failed: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        // A rejected code or refresh token is a reconnect, not a retry: 400 is
        // what Google answers an invalid_grant with, and treating it as
        // permanent would leave the UI claiming everything is fine.
        if status == 400 || status == 401 {
            return Err(SyncError::Auth(truncate(&body)));
        }
        return Err(classify_status(status.as_u16(), &body));
    }
    let parsed: TokenResponse = serde_json::from_str(&body)
        .map_err(|e| SyncError::Transient(format!("token response unreadable: {e}")))?;
    Ok(parsed.into_token_set(Utc::now()))
}

/// Obtain the cursor a delta run starts from. Called once per connection.
pub async fn start_page_token(http: &reqwest::Client, access_token: &str) -> Result<String, SyncError> {
    let url = format!("{DRIVE_API}/changes/startPageToken");
    let res = http
        .get(&url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("startPageToken failed: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_status(status.as_u16(), &body));
    }
    let json: Value = serde_json::from_str(&body)
        .map_err(|e| SyncError::Transient(format!("startPageToken unreadable: {e}")))?;
    json.get("startPageToken")
        .and_then(Value::as_str)
        .map(String::from)
        .ok_or_else(|| SyncError::Transient("startPageToken missing from response".into()))
}

/// Fetch one page of the change feed. `cursor` is a stored page token.
pub async fn fetch_page(
    http: &reqwest::Client,
    access_token: &str,
    cursor: Option<&str>,
) -> Result<DeltaPage, SyncError> {
    match cursor {
        // No cursor yet: full traversal of the user's files.
        None => fetch_list_page(http, access_token, None).await,
        Some(token) => fetch_change_page(http, access_token, Some(token)).await,
    }
}

/// One page of the initial `files.list` traversal. `page_token` continues a
/// traversal already in progress — it is *not* a stored cursor, which is why
/// the two functions are separate.
pub async fn fetch_list_page(
    http: &reqwest::Client,
    access_token: &str,
    page_token: Option<&str>,
) -> Result<DeltaPage, SyncError> {
    let url = match page_token {
        Some(token) => format!(
            "{DRIVE_API}/files?pageSize=100&pageToken={}&fields={FILE_FIELDS}",
            percent_encode(token)
        ),
        None => format!("{DRIVE_API}/files?pageSize=100&fields={FILE_FIELDS}"),
    };
    get_page(http, &url, access_token).await
}

/// One page of `changes.list`, replayed from a stored start page token.
pub async fn fetch_change_page(
    http: &reqwest::Client,
    access_token: &str,
    page_token: Option<&str>,
) -> Result<DeltaPage, SyncError> {
    let url = match page_token {
        Some(token) => format!(
            "{DRIVE_API}/changes?pageToken={}&fields={CHANGE_FIELDS}",
            percent_encode(token)
        ),
        None => format!("{DRIVE_API}/changes?fields={CHANGE_FIELDS}"),
    };
    get_page(http, &url, access_token).await
}

async fn get_page(
    http: &reqwest::Client,
    url: &str,
    access_token: &str,
) -> Result<DeltaPage, SyncError> {
    let res = http
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| SyncError::Transient(format!("drive request failed: {e}")))?;
    let status = res.status();
    let body = res.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(classify_status(status.as_u16(), &body));
    }
    let json: Value = serde_json::from_str(&body)
        .map_err(|e| SyncError::Transient(format!("drive response unreadable: {e}")))?;
    Ok(parse_page(&json))
}

/// Download the bytes of one file.
pub async fn download(
    http: &reqwest::Client,
    access_token: &str,
    remote_id: &str,
) -> Result<Vec<u8>, SyncError> {
    let url = format!("{DRIVE_API}/files/{}?alt=media", percent_encode(remote_id));
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

/// Turn a Drive error into the right retry decision.
///
/// Google signals throttling with 403 as often as 429, and the reason is only
/// in the body — so the status alone must not be trusted here.
pub fn classify_status(status: u16, body: &str) -> SyncError {
    if status == 401 {
        return SyncError::Auth(truncate(body));
    }
    if status == 429 || (status == 403 && body.contains("rateLimitExceeded")) {
        return SyncError::RateLimited(None);
    }
    if status == 403 {
        // No consent for this scope, or the app is not allowed to see the file.
        return SyncError::Auth(truncate(body));
    }
    if status >= 500 {
        return SyncError::Transient(format!("drive {status}: {}", truncate(body)));
    }
    SyncError::Permanent(format!("drive {status}: {}", truncate(body)))
}

/// Parse either a `files.list` or a `changes.list` response.
///
/// Both shapes are accepted by the same function on purpose: the first sync
/// walks files, later syncs walk changes, and the sync engine should not care
/// which one it is reading.
pub fn parse_page(json: &Value) -> DeltaPage {
    let mut page = DeltaPage::default();
    if let Some(files) = json.get("files").and_then(Value::as_array) {
        for file in files {
            if let Some(f) = parse_file(file) {
                page.items.push(f);
            }
        }
    }
    if let Some(changes) = json.get("changes").and_then(Value::as_array) {
        for change in changes {
            let Some(id) = change
                .get("fileId")
                .and_then(Value::as_str)
                .or_else(|| change.get("file").and_then(|f| f.get("id")).and_then(Value::as_str))
            else {
                continue;
            };
            if change.get("removed").and_then(Value::as_bool).unwrap_or(false) {
                page.removed.push(id.to_string());
                continue;
            }
            // A change for a trashed file is a removal for our purposes: the
            // user deleted it from their Drive and we must not keep serving it.
            match change.get("file") {
                Some(file) if file.get("trashed").and_then(Value::as_bool) == Some(true) => {
                    page.removed.push(id.to_string());
                }
                // The parent comes from the file's own `parents` list; the
                // change entry's `fileId` is the item itself, not its folder.
                Some(file) => page.items.extend(parse_file(file)),
                None => {}
            }
        }
    }
    page.next_cursor = json.get("nextPageToken").and_then(Value::as_str).map(String::from);
    page.delta = json
        .get("newStartPageToken")
        .and_then(Value::as_str)
        .map(String::from);
    page
}

/// Parse one file entry, or `None` when it carries nothing usable.
fn parse_file(file: &Value) -> Option<ExternalFile> {
    let id = file.get("id").and_then(Value::as_str)?;
    let name = file.get("name").and_then(Value::as_str).unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let mime = file.get("mimeType").and_then(Value::as_str);
    let is_folder = mime == Some(FOLDER_MIME);
    // Drive is flat: the path is derived from the parent chain by the caller
    // when it walks, so the name alone is the best single-page answer here.
    let mut f = ExternalFile::new(id, name, name, is_folder);
    f.mime_type = mime.map(String::from);
    f.size_bytes = file
        .get("size")
        .and_then(|s| match s {
            Value::String(v) => v.parse::<i64>().ok(),
            Value::Number(n) => n.as_i64(),
            _ => None,
        })
        .unwrap_or(0);
    f.modified_at = file
        .get("modifiedTime")
        .and_then(Value::as_str)
        .and_then(crate::external::onedrive::parse_ts);
    f.revision = file.get("version").and_then(Value::as_str).map(String::from);
    f.parent_remote_id = file
        .get("parents")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .map(String::from);
    Some(f)
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
#[path = "gdrive_tests.rs"]
mod tests;
