use super::*;

impl Default for ProtectionInstaller {
    fn default() -> Self {
        Self::new().unwrap_or(Self {
            user: "unknown".to_string(),
        })
    }
}
