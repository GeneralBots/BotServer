use super::*;

impl PrivacyLevel {
    /// Video visible to everyone
    pub const PUBLIC: &'static str = "PUBLIC_TO_EVERYONE";
    /// Video visible only to mutual followers
    pub const FRIENDS: &'static str = "MUTUAL_FOLLOW_FRIENDS";
    /// Video visible only to creator
    pub const PRIVATE: &'static str = "SELF_ONLY";
    /// Video visible to followers only
    pub const FOLLOWERS: &'static str = "FOLLOWER_OF_CREATOR";
}

// ============================================================================
// Scopes
// ============================================================================
