

/// Snapchat Marketing API provider
pub struct SnapchatProvider {
    pub(crate) client: reqwest::Client,
}

impl SnapchatProvider {
    /// Provider endpoints are compile-time constants: tokens are scoped to
    /// these hosts and must never be sent to a runtime-configured one.
    pub(crate) const API_BASE: &str = "https://adsapi.snapchat.com/v1";
    pub(crate) const OAUTH_BASE: &str = "https://accounts.snapchat.com";
}
