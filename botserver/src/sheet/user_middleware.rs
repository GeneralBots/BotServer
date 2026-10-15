//! Bridges the platform auth extension into the Sheet handler scope (#789).
//!
//! The platform auth middleware inserts `AuthenticatedUser` into request
//! extensions. This middleware converts it into the sheet-scoped
//! [`SheetUser`] so the `botsheet` crate never depends on the security crate.

use axum::{body::Body, http::Request, middleware::Next, response::Response};
use botsheet::auth::SheetUser;

#[cfg(feature = "security")]
pub async fn sheet_user_middleware(mut request: Request<Body>, next: Next) -> Response {
    let user = request
        .extensions()
        .get::<crate::security::auth::AuthenticatedUser>()
        .cloned();
    let sheet_user = match user {
        Some(u) if u.user_id != uuid::Uuid::nil() => SheetUser {
            id: u.user_id.to_string(),
            name: u.username.clone(),
            authenticated: true,
        },
        _ => SheetUser::anonymous(),
    };
    request.extensions_mut().insert(sheet_user);
    next.run(request).await
}

#[cfg(not(feature = "security"))]
pub async fn sheet_user_middleware(mut request: Request<Body>, next: Next) -> Response {
    request.extensions_mut().insert(SheetUser::anonymous());
    next.run(request).await
}
