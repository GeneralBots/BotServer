use super::*;

impl Permissions {
    // Organization permissions
    pub const ORG_MANAGE: &'static str = "org:manage";
    pub const ORG_BILLING: &'static str = "org:billing";
    pub const ORG_MEMBERS: &'static str = "org:members";
    pub const ORG_SETTINGS: &'static str = "org:settings";

    // Bot permissions
    pub const BOT_CREATE: &'static str = "bot:create";
    pub const BOT_EDIT: &'static str = "bot:edit";
    pub const BOT_DELETE: &'static str = "bot:delete";
    pub const BOT_PUBLISH: &'static str = "bot:publish";

    // KB permissions
    pub const KB_READ: &'static str = "kb:read";
    pub const KB_WRITE: &'static str = "kb:write";
    pub const KB_ADMIN: &'static str = "kb:admin";

    // App permissions
    pub const APP_CREATE: &'static str = "app:create";
    pub const APP_EDIT: &'static str = "app:edit";
    pub const APP_DELETE: &'static str = "app:delete";

    // Analytics permissions
    pub const ANALYTICS_VIEW: &'static str = "analytics:view";
    pub const ANALYTICS_EXPORT: &'static str = "analytics:export";
}
/// Default permissions for each role
pub fn default_permissions_for_role(role: &str) -> Vec<&'static str> {
    match role {
        "owner" => vec![
            Permissions::ORG_MANAGE,
            Permissions::ORG_BILLING,
            Permissions::ORG_MEMBERS,
            Permissions::ORG_SETTINGS,
            Permissions::BOT_CREATE,
            Permissions::BOT_EDIT,
            Permissions::BOT_DELETE,
            Permissions::BOT_PUBLISH,
            Permissions::KB_READ,
            Permissions::KB_WRITE,
            Permissions::KB_ADMIN,
            Permissions::APP_CREATE,
            Permissions::APP_EDIT,
            Permissions::APP_DELETE,
            Permissions::ANALYTICS_VIEW,
            Permissions::ANALYTICS_EXPORT,
        ],
        "admin" => vec![
            Permissions::ORG_MEMBERS,
            Permissions::ORG_SETTINGS,
            Permissions::BOT_CREATE,
            Permissions::BOT_EDIT,
            Permissions::BOT_DELETE,
            Permissions::BOT_PUBLISH,
            Permissions::KB_READ,
            Permissions::KB_WRITE,
            Permissions::KB_ADMIN,
            Permissions::APP_CREATE,
            Permissions::APP_EDIT,
            Permissions::APP_DELETE,
            Permissions::ANALYTICS_VIEW,
            Permissions::ANALYTICS_EXPORT,
        ],
        "member" => vec![
            Permissions::BOT_CREATE,
            Permissions::BOT_EDIT,
            Permissions::KB_READ,
            Permissions::KB_WRITE,
            Permissions::APP_CREATE,
            Permissions::APP_EDIT,
            Permissions::ANALYTICS_VIEW,
        ],
        "viewer" => vec![Permissions::KB_READ, Permissions::ANALYTICS_VIEW],
        "guest" => vec![Permissions::KB_READ],
        _ => vec![],
    }
}
