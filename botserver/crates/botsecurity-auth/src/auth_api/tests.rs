use super::{AuthConfig, AuthError};
use super::super::types::*;
use axum::http::StatusCode;

#[test]
fn test_role_permissions() {
    assert!(!Role::Anonymous.has_permission(&Permission::Read));
    assert!(Role::User.has_permission(&Permission::Read));
    assert!(Role::User.has_permission(&Permission::AccessApi));
    assert!(!Role::User.has_permission(&Permission::Write));

    assert!(Role::Admin.has_permission(&Permission::ManageUsers));
    assert!(Role::Admin.has_permission(&Permission::Delete));

    assert!(Role::SuperAdmin.has_permission(&Permission::ManageSecrets));
}

#[test]
fn test_role_hierarchy() {
    assert!(Role::SuperAdmin.is_at_least(&Role::Admin));
    assert!(Role::Admin.is_at_least(&Role::Moderator));
    assert!(Role::BotOwner.is_at_least(&Role::BotOperator));
    assert!(Role::BotOperator.is_at_least(&Role::BotViewer));
    assert!(!Role::User.is_at_least(&Role::Admin));
}

#[test]
fn test_authenticated_user_builder() {
    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "testuser".to_string())
        .with_email("test@example.com")
        .with_role(Role::Admin)
        .with_metadata("key", "value");

    assert_eq!(user.email, Some("test@example.com".to_string()));
    assert!(user.has_role(&Role::Admin));
    assert_eq!(user.metadata.get("key"), Some(&"value".to_string()));
}

#[test]
fn test_user_permissions() {
    let admin =
        AuthenticatedUser::new(uuid::Uuid::new_v4(), "admin".to_string()).with_role(Role::Admin);

    assert!(admin.has_permission(&Permission::ManageUsers));
    assert!(admin.has_permission(&Permission::Delete));
    assert!(admin.is_admin());

    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string());
    assert!(user.has_permission(&Permission::Read));
    assert!(!user.has_permission(&Permission::ManageUsers));
    assert!(!user.is_admin());
}

#[test]
fn test_anonymous_user() {
    let anon = AuthenticatedUser::anonymous();
    assert!(!anon.is_authenticated());
    assert!(anon.has_role(&Role::Anonymous));
    assert!(!anon.has_permission(&Permission::Read));
}

#[test]
fn test_service_user() {
    let service = AuthenticatedUser::service("scheduler");
    assert!(service.has_role(&Role::Service));
    assert!(service.has_permission(&Permission::ExecuteTasks));
}

#[test]
fn test_bot_user() {
    let bot_id = uuid::Uuid::new_v4();
    let bot = AuthenticatedUser::bot_user(bot_id, "test-bot");
    assert!(bot.is_bot());
    assert!(bot.has_permission(&Permission::SendMessages));
    assert_eq!(bot.current_bot_id, Some(bot_id));
}

#[test]
fn test_auth_config_paths() {
    let config = AuthConfig::default();

    assert!(config.is_anonymous_allowed("/health"));
    assert!(config.is_anonymous_allowed("/api/health"));
    assert!(!config.is_anonymous_allowed("/api/users"));

    assert!(config.is_public_path("/static"));
    assert!(config.is_public_path("/static/css/style.css"));
    assert!(!config.is_public_path("/api/private"));
}

#[test]
fn test_auth_config_glob_paths() {
    let config = AuthConfig::default().add_anonymous_path("/api/bots/*/access");

    // Wildcard scopes exactly one route shape.
    assert!(config.is_anonymous_allowed("/api/bots/foo/access"));
    assert!(config.is_anonymous_allowed("/api/bots/default/access"));
    // Sibling routes under /api/bots must NOT be caught by the glob.
    assert!(!config.is_anonymous_allowed("/api/bots/list"));
    assert!(!config.is_anonymous_allowed("/api/bots/config"));
    assert!(!config.is_anonymous_allowed("/api/bots"));
    assert!(!config.is_anonymous_allowed("/api/bots/foo/config"));
    assert!(!config.is_anonymous_allowed("/api/bots/a/access/extra"));
}

#[test]
fn test_auth_config_trailing_slash_is_noop() {
    // A trailing-slash entry must not widen the namespace: the legacy
    // matcher produced a `//` prefix that matches nothing. Pin that the
    // anonymous allowlist never exposes sibling routes via a trailing
    // slash — the previous `/api/bots/` entry was a silent no-op.
    let config = AuthConfig::default().add_anonymous_path("/api/bots/");
    assert!(!config.is_anonymous_allowed("/api/bots/list"));
    assert!(!config.is_anonymous_allowed("/api/bots/foo/access"));
    assert!(!config.is_anonymous_allowed("/api/bots"));
}

#[test]
fn test_auth_error_responses() {
    assert_eq!(
        AuthError::MissingToken.status_code(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        AuthError::InsufficientPermissions.status_code(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        AuthError::RateLimited.status_code(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        AuthError::BotAccessDenied.status_code(),
        StatusCode::FORBIDDEN
    );
}

#[test]
fn test_bot_access() {
    let bot_id = uuid::Uuid::new_v4();
    let other_bot_id = uuid::Uuid::new_v4();

    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string())
        .with_bot_access(BotAccess::viewer(bot_id));

    assert!(user.can_access_bot(&bot_id));
    assert!(user.can_view_bot(&bot_id));
    assert!(!user.can_operate_bot(&bot_id));
    assert!(!user.can_manage_bot(&bot_id));
    assert!(!user.can_access_bot(&other_bot_id));

    let admin =
        AuthenticatedUser::new(uuid::Uuid::new_v4(), "admin".to_string()).with_role(Role::Admin);

    assert!(admin.can_access_bot(&bot_id));
    assert!(admin.can_access_bot(&other_bot_id));
}

#[test]
fn test_bot_owner_access() {
    let bot_id = uuid::Uuid::new_v4();

    let owner = AuthenticatedUser::new(uuid::Uuid::new_v4(), "owner".to_string())
        .with_bot_access(BotAccess::owner(bot_id));

    assert!(owner.can_access_bot(&bot_id));
    assert!(owner.can_view_bot(&bot_id));
    assert!(owner.can_operate_bot(&bot_id));
    assert!(owner.can_manage_bot(&bot_id));
}

#[test]
fn test_bot_operator_access() {
    let bot_id = uuid::Uuid::new_v4();

    let operator = AuthenticatedUser::new(uuid::Uuid::new_v4(), "operator".to_string())
        .with_bot_access(BotAccess::operator(bot_id));

    assert!(operator.can_access_bot(&bot_id));
    assert!(operator.can_view_bot(&bot_id));
    assert!(operator.can_operate_bot(&bot_id));
    assert!(!operator.can_manage_bot(&bot_id));
}

#[test]
fn test_bot_permission_check() {
    let bot_id = uuid::Uuid::new_v4();

    let operator = AuthenticatedUser::new(uuid::Uuid::new_v4(), "operator".to_string())
        .with_bot_access(BotAccess::operator(bot_id));

    assert!(operator.has_bot_permission(&bot_id, &Permission::SendMessages));
    assert!(operator.has_bot_permission(&bot_id, &Permission::ViewAnalytics));
    assert!(!operator.has_bot_permission(&bot_id, &Permission::ManageBots));
}

#[test]
fn test_bot_access_expiry() {
    let bot_id = uuid::Uuid::new_v4();
    let past_time = chrono::Utc::now().timestamp() - 3600;

    let expired_access = BotAccess::viewer(bot_id).with_expiry(past_time);
    assert!(expired_access.is_expired());
    assert!(!expired_access.is_valid());

    let future_time = chrono::Utc::now().timestamp() + 3600;
    let valid_access = BotAccess::viewer(bot_id).with_expiry(future_time);
    assert!(!valid_access.is_expired());
    assert!(valid_access.is_valid());
}

#[test]
fn test_accessible_bot_ids() {
    let bot1 = uuid::Uuid::new_v4();
    let bot2 = uuid::Uuid::new_v4();

    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string())
        .with_bot_access(BotAccess::owner(bot1))
        .with_bot_access(BotAccess::viewer(bot2));

    let accessible = user.accessible_bot_ids();
    assert_eq!(accessible.len(), 2);
    assert!(accessible.contains(&bot1));
    assert!(accessible.contains(&bot2));

    let owned = user.owned_bot_ids();
    assert_eq!(owned.len(), 1);
    assert!(owned.contains(&bot1));
}

#[test]
fn test_organization_access() {
    let org_id = uuid::Uuid::new_v4();
    let other_org_id = uuid::Uuid::new_v4();

    let user =
        AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string()).with_organization(org_id);

    assert!(user.can_access_organization(&org_id));
    assert!(!user.can_access_organization(&other_org_id));
}

#[test]
fn test_has_any_permission() {
    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string());

    assert!(user.has_any_permission(&[Permission::Read, Permission::Write]));
    assert!(!user.has_any_permission(&[Permission::Delete, Permission::Admin]));
}

#[test]
fn test_has_all_permissions() {
    let admin =
        AuthenticatedUser::new(uuid::Uuid::new_v4(), "admin".to_string()).with_role(Role::Admin);

    assert!(admin.has_all_permissions(&[
        Permission::Read,
        Permission::Write,
        Permission::Delete
    ]));
    assert!(!admin.has_all_permissions(&[Permission::ManageSecrets]));
}

#[test]
fn test_highest_role() {
    let user = AuthenticatedUser::new(uuid::Uuid::new_v4(), "user".to_string())
        .with_role(Role::Admin)
        .with_role(Role::Moderator);

    assert_eq!(user.highest_role(), &Role::Admin);
}
