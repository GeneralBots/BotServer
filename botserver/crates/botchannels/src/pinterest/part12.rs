use super::*;

impl PinterestScopes {
    pub const BOARDS_READ: &'static str = "boards:read";
    pub const BOARDS_WRITE: &'static str = "boards:write";
    pub const PINS_READ: &'static str = "pins:read";
    pub const PINS_WRITE: &'static str = "pins:write";
    pub const USER_ACCOUNTS_READ: &'static str = "user_accounts:read";
    pub const ADS_READ: &'static str = "ads:read";
    pub const ADS_WRITE: &'static str = "ads:write";

    /// Get recommended scopes for content posting
    pub fn content_posting_scopes() -> &'static str {
        "boards:read,boards:write,pins:read,pins:write,user_accounts:read"
    }

    /// Get all available scopes
    pub fn all_scopes() -> &'static str {
        "boards:read,boards:write,pins:read,pins:write,user_accounts:read,ads:read,ads:write"
    }
}
