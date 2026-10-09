use super::*;

impl std::fmt::Display for OrganizationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Organization not found"),
            Self::AlreadyExists => write!(f, "Organization already exists"),
            Self::InvalidName(n) => write!(f, "Invalid organization name: {}", n),
            Self::PermissionDenied(p) => write!(f, "Permission denied: {}", p),
            Self::MemberLimitReached => write!(f, "Member limit reached"),
            Self::InvalidRole(r) => write!(f, "Invalid role: {}", r),
            Self::InvalidGroup(g) => write!(f, "Invalid group: {}", g),
            Self::DatabaseError(e) => write!(f, "Database error: {}", e),
        }
    }
}
impl std::error::Error for OrganizationError {}

// ============================================================================
// Helper Functions
// ============================================================================
/// Generate a URL-safe slug from a name
pub(super) fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() || c == '-' || c == '_' {
                '-'
            } else {
                '\0'
            }
        })
        .filter(|c| *c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ============================================================================
// Permission Constants
// ============================================================================
