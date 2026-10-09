use super::*;

impl Default for McpAuth {
    fn default() -> Self {
        Self {
            auth_type: McpAuthType::None,
            credentials: McpCredentials::None,
        }
    }
}
