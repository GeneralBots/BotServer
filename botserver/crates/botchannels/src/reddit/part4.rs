use super::*;

impl std::fmt::Display for RedditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NetworkError(e) => write!(f, "Network error: {}", e),
            Self::ApiError(e) => write!(f, "Reddit API error: {}", e),
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::ConfigError(e) => write!(f, "Configuration error: {}", e),
            Self::NotAuthenticated => write!(f, "Not authenticated with Reddit"),
            Self::PostNotFound => write!(f, "Post not found"),
            Self::RateLimited => write!(f, "Rate limited by Reddit API"),
        }
    }
}
impl std::error::Error for RedditError {}
pub fn create_reddit_config(
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    user_agent: &str,
) -> RedditConfig {
    RedditConfig {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        redirect_uri: redirect_uri.to_string(),
        user_agent: user_agent.to_string(),
        username: None,
        password: None,
    }
}
pub fn create_script_config(
    client_id: &str,
    client_secret: &str,
    username: &str,
    password: &str,
    user_agent: &str,
) -> RedditConfig {
    RedditConfig {
        client_id: client_id.to_string(),
        client_secret: client_secret.to_string(),
        redirect_uri: String::new(),
        user_agent: user_agent.to_string(),
        username: Some(username.to_string()),
        password: Some(password.to_string()),
    }
}
