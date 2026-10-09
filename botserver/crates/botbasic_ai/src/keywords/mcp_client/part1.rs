use super::*;

impl Default for McpServerType {
    fn default() -> Self {
        Self::Custom("unknown".to_string())
    }
}
impl From<&str> for McpServerType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "database" | "db" => Self::Database,
            "filesystem" | "fs" | "file" => Self::Filesystem,
            "web" | "http" | "rest" | "api" => Self::Web,
            "email" | "mail" | "smtp" | "imap" => Self::Email,
            "slack" => Self::Slack,
            "teams" | "microsoft-teams" => Self::Teams,
            "analytics" | "data" => Self::Analytics,
            "search" | "elasticsearch" | "opensearch" => Self::Search,
            "storage" | "s3" | "blob" | "gcs" => Self::Storage,
            "compute" | "lambda" | "function" => Self::Compute,
            other => Self::Custom(other.to_string()),
        }
    }
}
impl std::fmt::Display for McpServerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database => write!(f, "database"),
            Self::Filesystem => write!(f, "filesystem"),
            Self::Web => write!(f, "web"),
            Self::Email => write!(f, "email"),
            Self::Slack => write!(f, "slack"),
            Self::Teams => write!(f, "teams"),
            Self::Analytics => write!(f, "analytics"),
            Self::Search => write!(f, "search"),
            Self::Storage => write!(f, "storage"),
            Self::Compute => write!(f, "compute"),
            Self::Custom(s) => write!(f, "{s}"),
        }
    }
}
