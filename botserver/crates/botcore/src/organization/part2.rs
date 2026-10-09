use super::*;

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            allow_public_bots: false,
            require_2fa: false,
            allowed_email_domains: Vec::new(),
            default_user_role: "member".to_string(),
            max_members: None,
            sso_enabled: false,
            sso_provider: None,
            audit_log_retention_days: 90,
            ip_safelist: Vec::new(),
            custom_branding: None,
        }
    }
}
