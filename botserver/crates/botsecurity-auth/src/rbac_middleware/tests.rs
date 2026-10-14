use super::*;

use super::*;

#[test]
fn test_route_permission_exact_match() {
    let route = RoutePermission::new("/api/users", "GET", "users.read");

    assert!(route.matches_path("/api/users"));
    assert!(!route.matches_path("/api/users/123"));
    assert!(!route.matches_path("/api/user"));
}

#[test]
fn test_route_permission_param_match() {
    let route = RoutePermission::new("/api/users/{id}", "GET", "users.read");

    assert!(route.matches_path("/api/users/123"));
    assert!(route.matches_path("/api/users/abc"));
    assert!(!route.matches_path("/api/users"));
    assert!(!route.matches_path("/api/users/123/profile"));
}

#[test]
fn test_route_permission_wildcard_match() {
    let route = RoutePermission::new("/api/drive/**", "GET", "drive.read");

    assert!(route.matches_path("/api/drive"));
    assert!(route.matches_path("/api/drive/files"));
    assert!(route.matches_path("/api/drive/files/123"));
    assert!(route.matches_path("/api/drive/a/b/c/d"));
    assert!(!route.matches_path("/api/mail"));
}

#[test]
fn test_route_permission_single_wildcard() {
    let route = RoutePermission::new("/api/*/info", "GET", "info.read");

    assert!(route.matches_path("/api/users/info"));
    assert!(route.matches_path("/api/bots/info"));
}

#[tokio::test]
async fn test_people_routes_match_and_allow_authenticated() {
    let routes = build_default_route_permissions();
    let manager = RbacManager::with_defaults();
    manager.register_routes(routes).await;
    let user = AuthenticatedUser::new(Uuid::new_v4(), "test@example.com".to_string());
    let allowed_base = manager
        .check_route_access("/api/people", "POST", &user)
        .await
        .is_allowed();
    assert!(allowed_base, "POST /api/people should be allowed for authenticated user");
    let allowed_skill = manager
        .check_route_access("/api/people/123/skills", "POST", &user)
        .await
        .is_allowed();
    assert!(allowed_skill, "POST /api/people/:id/skills should be allowed");
}

#[tokio::test]
async fn test_tenant_cloud_reads_allowed_and_admin_surfaces_stay_denied() {
    let routes = build_default_route_permissions();
    let manager = RbacManager::with_defaults();
    manager.register_routes(routes).await;
    let user = AuthenticatedUser::new(Uuid::new_v4(), "owner@example.com".to_string())
        .with_role(Role::User);

    for path in [
        "/api/cloud/bots",
        "/api/cloud/organizations",
        "/api/cloud/services",
        "/api/cloud/invoices",
        "/api/cloud/plans",
        "/api/cloud/payment-cards",
        "/api/cloud/tenant/settings/byok",
    ] {
        let decision = manager.check_route_access(path, "GET", &user).await;
        assert!(
            decision.is_allowed(),
            "GET {path} should be allowed for an authenticated tenant user: {}",
            decision.reason
        );
    }

    for path in ["/api/cloud/vouchers", "/api/cloud/domains"] {
        let decision = manager.check_route_access(path, "GET", &user).await;
        assert!(
            !decision.is_allowed(),
            "GET {path} is a super-admin surface and must stay denied"
        );
    }
}

#[tokio::test]
async fn test_caldav_verbs_allowed_for_authenticated_client() {
    // Regression for #1335/#1336: Thunderbird, Apple Calendar and DAVx5
    // authenticate with HTTP Basic and speak DAV verbs against /caldav. No
    // route permission existed for those verbs, so RBAC answered "No
    // matching route permission found" (403) even with a valid credential
    // and no client could ever sync. The DAV router keeps its own
    // per-calendar authorization, so a plain authenticated user must be
    // admitted here.
    let routes = build_default_route_permissions();
    let manager = RbacManager::with_defaults();
    manager.register_routes(routes).await;
    let user = AuthenticatedUser::new(Uuid::new_v4(), "calendar@example.com".to_string())
        .with_role(Role::User);

    for (path, method) in [
        ("/.well-known/caldav", "GET"),
        ("/caldav", "OPTIONS"),
        ("/caldav", "PROPFIND"),
        ("/caldav", "REPORT"),
        ("/caldav", "GET"),
        ("/caldav/calendars/user/calendar.ics", "GET"),
        ("/caldav/calendars/user/event.ics", "PUT"),
        ("/caldav/calendars/user/event.ics", "DELETE"),
        ("/caldav/calendars/user", "PROPFIND"),
    ] {
        let decision = manager.check_route_access(path, method, &user).await;
        assert!(
            decision.is_allowed(),
            "{method} {path} must be allowed for an authenticated DAV client"
        );
    }
}

#[tokio::test]
async fn test_compliance_and_timeclock_allowed_for_plain_user() {
    // Regression for #909/#910/#916: the Compliance and Timeclock suite
    // apps 403'd for logged-in users — compliance was Admin-gated and the
    // /api/timeclock/** prefix was missing from the permission list, so
    // default_deny rejected every request. Both must work for any
    // authenticated (non-admin) user.
    let routes = build_default_route_permissions();
    let manager = RbacManager::with_defaults();
    manager.register_routes(routes).await;
    let user = AuthenticatedUser::new(Uuid::new_v4(), "user@example.com".to_string())
        .with_role(Role::User);

    for (path, method) in [
        ("/api/compliance/checks", "GET"),
        ("/api/compliance/issues", "GET"),
        ("/api/compliance/audit-log", "GET"),
        ("/api/compliance/risks", "GET"),
        ("/api/compliance/training", "GET"),
        ("/api/timeclock/forms/overtime", "POST"),
        ("/api/timeclock/clock", "POST"),
        ("/api/timeclock/records", "GET"),
    ] {
        let decision = manager.check_route_access(path, method, &user).await;
        assert!(
            decision.is_allowed(),
            "{} {} should be allowed for authenticated user: {}",
            method,
            path,
            decision.reason
        );
    }
}

#[test]
fn test_resource_acl_owner_access() {
    let owner_id = Uuid::new_v4();
    let other_id = Uuid::new_v4();

    let acl = ResourceAcl::new("file", "123").with_owner(owner_id);

    assert!(acl.check_access(Some(owner_id), &[], "read"));
    assert!(acl.check_access(Some(owner_id), &[], "write"));
    assert!(acl.check_access(Some(owner_id), &[], "delete"));
    assert!(!acl.check_access(Some(other_id), &[], "read"));
}

#[test]
fn test_resource_acl_user_permissions() {
    let user_id = Uuid::new_v4();
    let mut acl = ResourceAcl::new("file", "123");

    acl.grant_user(user_id, "read");

    assert!(acl.check_access(Some(user_id), &[], "read"));
    assert!(!acl.check_access(Some(user_id), &[], "write"));
}

#[test]
fn test_resource_acl_group_permissions() {
    let user_id = Uuid::new_v4();
    let mut acl = ResourceAcl::new("file", "123");

    acl.grant_group("editors", "write");

    assert!(acl.check_access(Some(user_id), &["editors".into()], "write"));
    assert!(!acl.check_access(Some(user_id), &["viewers".into()], "write"));
}

#[test]
fn test_resource_acl_public_permissions() {
    let mut acl = ResourceAcl::new("file", "123");

    acl.set_public("read");

    assert!(acl.check_access(None, &[], "read"));
    assert!(!acl.check_access(None, &[], "write"));
}

#[test]
fn test_resource_acl_admin_access() {
    let user_id = Uuid::new_v4();
    let mut acl = ResourceAcl::new("file", "123");

    acl.grant_user(user_id, "admin");

    assert!(acl.check_access(Some(user_id), &[], "read"));
    assert!(acl.check_access(Some(user_id), &[], "write"));
    assert!(acl.check_access(Some(user_id), &[], "delete"));
}

#[test]
fn test_access_decision_result() {
    let allow = AccessDecisionResult::allow("Test allow");
    assert!(allow.is_allowed());

    let deny = AccessDecisionResult::deny("Test deny");
    assert!(!deny.is_allowed());
}

#[test]
fn test_resource_permission_builders() {
    let read = ResourcePermission::read("file", "123");
    assert_eq!(read.permission, "read");

    let write = ResourcePermission::write("file", "123");
    assert_eq!(write.permission, "write");

    let delete = ResourcePermission::delete("file", "123");
    assert_eq!(delete.permission, "delete");
}

#[tokio::test]
async fn test_rbac_manager_creation() {
    let manager = RbacManager::with_defaults();
    let routes = build_default_route_permissions();

    manager.register_routes(routes).await;

    let user = AuthenticatedUser::anonymous();
    let decision = manager
        .check_route_access("/api/health", "GET", &user)
        .await;

    assert!(decision.is_allowed());
}

#[tokio::test]
async fn test_user_groups() {
    let manager = RbacManager::with_defaults();
    let user_id = Uuid::new_v4();

    manager.set_user_groups(user_id, vec!["editors".into(), "viewers".into()]).await;

    let groups = manager.get_user_groups(user_id).await;
    assert_eq!(groups.len(), 2);
    assert!(groups.contains(&"editors".into()));
}

#[tokio::test]
async fn test_resource_acl_management() {
    let manager = RbacManager::with_defaults();
    let owner_id = Uuid::new_v4();

    let acl = ResourceAcl::new("document", "doc-123").with_owner(owner_id);
    manager.set_resource_acl(acl).await;

    let retrieved = manager.get_resource_acl("document", "doc-123").await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.as_ref().and_then(|a| a.owner_id), Some(owner_id));
}

#[cfg(test)]
mod run_route_tests {
    use super::*;
    #[test]
    fn run_get_matches_uuid_path() {
        let r = RoutePermission::new("/api/vibe/run/**", "GET", "");
        assert!(r.matches_path("/api/vibe/run/840abb94-d1ce-43ad-ac70-56443a6fa626"), "uuid path");
        assert!(r.matches_path("/api/vibe/run/abc"), "short id");
    }
}
