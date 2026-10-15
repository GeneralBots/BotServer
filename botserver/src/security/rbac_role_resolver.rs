//! HTTP role-resolution middleware (fix #843 continuation).
//!
//! The auth providers (SaaS cloud JWT, Zitadel) assign roles from token
//! claims only — cloud JWTs always yield `Role::User`, so a user who IS an
//! admin in `rbac_user_groups` still gets 403 on `/api/admin/**`,
//! `/api/monitoring/**`, `/api/ui/analytics/**` etc. (the "403 storm" seen in
//! the suite console).
//!
//! This middleware runs between authentication and the RBAC check: for every
//! authenticated non-admin user it resolves the effective role from the DB
//! (group membership, with an email fallback for identity-id drift) and, when
//! admin, upgrades the `AuthenticatedUser` in the request extensions so the
//! RBAC middleware's `has_role(Role::Admin)` passes. Non-admin users are
//! untouched and keep the correct 403s.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::sync::Arc;

use botcore::shared::state::AppState;
use crate::security::auth::{AuthenticatedUser, Role};
use crate::security::user_role::{resolve_user_role_with_email, ROLE_ADMIN};

pub async fn rbac_role_resolver_middleware(mut request: Request<Body>, next: Next) -> Response {
    let should_resolve = {
        let has_state = request.extensions().get::<Arc<AppState>>().is_some();
        let user = request.extensions().get::<AuthenticatedUser>();
        match (has_state, user) {
            (true, Some(u)) => u.is_authenticated() && !u.is_admin(),
            _ => false,
        }
    };

    if should_resolve {
        let state = request
            .extensions()
            .get::<Arc<AppState>>()
            .cloned();
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned();
        if let (Some(state), Some(user)) = (state, user) {
            let role = resolve_user_role_with_email(&state.conn, user.user_id, user.email.as_deref());
            if role == ROLE_ADMIN {
                let upgraded = user.with_role(Role::Admin);
                request.extensions_mut().insert(upgraded);
            }
        }
    }

    next.run(request).await
}
