use super::{
    config::AuthConfig,
    error::AuthError,
    types::{AuthenticatedUser, Permission, PublicPathAllowed, Role},
    auth_api_auth::authenticate_with_extracted_data,
    auth_api_utils::ExtractedAuthData,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::auth_provider::AuthProviderRegistry;

#[derive(Clone)]
pub struct AuthMiddlewareState {
    pub config: Arc<AuthConfig>,
    pub provider_registry: Arc<AuthProviderRegistry>,
}

impl AuthMiddlewareState {
    pub fn new(config: Arc<AuthConfig>, provider_registry: Arc<AuthProviderRegistry>) -> Self {
        Self {
            config,
            provider_registry,
        }
    }
}

pub async fn auth_middleware_with_providers(
    mut request: Request<Body>,
    next: Next,
    state: AuthMiddlewareState,
) -> Response {
    let path = request.uri().path().to_string();
    let method = request.method().to_string();

    info!("Processing {} {}", method, path);

    if state.config.is_public_path(&path) || state.config.is_anonymous_allowed(&path) {
        info!("Path is public/anonymous, skipping auth: {}", path);
        // Public/anonymous paths must still resolve a supplied Bearer token
        // so handlers (e.g. the bot access check) can tell real users apart
        // from anonymous callers. Fall back to anonymous when the token is
        // absent or invalid; never reject on a bad token here.
        let extracted = ExtractedAuthData::from_request(&request, &state.config);
        let user = match authenticate_with_extracted_data(
            extracted,
            &state.config,
            &state.provider_registry,
        )
        .await
        {
            Ok(user) => user,
            Err(e) => {
                info!("Public path token resolution failed, allowing anonymous: {:?}", e);
                AuthenticatedUser::anonymous()
            }
        };
        request.extensions_mut().insert(user);
        request.extensions_mut().insert(PublicPathAllowed);
        let response = next.run(request).await;
        info!("Public path response status: {}", response.status());
        return response;
    }

    if let Some(expected) = state.config.internal_token.as_deref() {
        let supplied = request
            .headers()
            .get(&state.config.internal_token_header)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        if !supplied.is_empty() && supplied == expected {
            info!("Internal token accepted for {} {}", method, path);
            request.extensions_mut().insert(AuthenticatedUser::service("internal"));
            request.extensions_mut().insert(PublicPathAllowed);
            let response = next.run(request).await;
            info!("Internal path response status: {}", response.status());
            return response;
        }
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    info!(
        "Authorization header: {:?}",
        auth_header.as_ref().map(|h| {
            // The header is caller-supplied: never slice it mid-character.
            botlib::text::truncate_with_suffix(h, 30, "...")
        })
    );

    let extracted = ExtractedAuthData::from_request(&request, &state.config);
    let user =
        authenticate_with_extracted_data(extracted, &state.config, &state.provider_registry).await;

    match user {
        Ok(authenticated_user) => {
            info!(
                "Success: user={} roles={:?}",
                authenticated_user.username, authenticated_user.roles
            );
            request.extensions_mut().insert(authenticated_user);
            next.run(request).await
        }
        Err(e) => {
            if !state.config.require_auth {
                info!("Failed but not required, allowing anonymous: {:?}", e);
                request
                    .extensions_mut()
                    .insert(AuthenticatedUser::anonymous());
                return next.run(request).await;
            }
            info!("Failed: {:?}", e);
            e.into_response()
        }
    }
}

pub async fn auth_middleware(
    State(config): State<std::sync::Arc<AuthConfig>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let path = request.uri().path().to_string();

    if config.is_public_path(&path) || config.is_anonymous_allowed(&path) {
        request
            .extensions_mut()
            .insert(AuthenticatedUser::anonymous());
        return Ok(next.run(request).await);
    }

    match super::auth_api_utils::extract_user_from_request(&request, &config) {
        Ok(user) => {
            request.extensions_mut().insert(user);
            Ok(next.run(request).await)
        }
        Err(e) => {
            if !config.require_auth {
                request
                    .extensions_mut()
                    .insert(AuthenticatedUser::anonymous());
                return Ok(next.run(request).await);
            }
            Err(e)
        }
    }
}

pub async fn require_auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.is_authenticated() {
        return Err(AuthError::MissingToken);
    }

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub fn require_permission(
    permission: Permission,
) -> impl Fn(Request<Body>) -> Result<Request<Body>, AuthError> + Clone {
    move |request: Request<Body>| {
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned()
            .unwrap_or_else(AuthenticatedUser::anonymous);

        if !user.has_permission(&permission) {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(request)
    }
}

pub fn require_role(
    role: Role,
) -> impl Fn(Request<Body>) -> Result<Request<Body>, AuthError> + Clone {
    move |request: Request<Body>| {
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned()
            .unwrap_or_else(AuthenticatedUser::anonymous);

        if !user.has_role(&role) {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(request)
    }
}

pub fn require_admin() -> impl Fn(Request<Body>) -> Result<Request<Body>, AuthError> + Clone {
    move |request: Request<Body>| {
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned()
            .unwrap_or_else(AuthenticatedUser::anonymous);

        if !user.is_admin() {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(request)
    }
}

pub fn require_bot_access(
    bot_id: Uuid,
) -> impl Fn(Request<Body>) -> Result<Request<Body>, AuthError> + Clone {
    move |request: Request<Body>| {
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned()
            .unwrap_or_else(AuthenticatedUser::anonymous);

        if !user.can_access_bot(&bot_id) {
            return Err(AuthError::BotAccessDenied);
        }

        Ok(request)
    }
}

pub fn require_bot_permission(
    bot_id: Uuid,
    permission: Permission,
) -> impl Fn(Request<Body>) -> Result<Request<Body>, AuthError> + Clone {
    move |request: Request<Body>| {
        let user = request
            .extensions()
            .get::<AuthenticatedUser>()
            .cloned()
            .unwrap_or_else(AuthenticatedUser::anonymous);

        if !user.has_bot_permission(&bot_id, &permission) {
            return Err(AuthError::InsufficientPermissions);
        }

        Ok(request)
    }
}

pub async fn require_permission_middleware(
    permission: Permission,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.has_permission(&permission) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

pub async fn require_role_middleware(
    role: Role,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.has_role(&role) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

pub async fn admin_only_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.is_admin() {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

pub async fn bot_scope_middleware(
    Path(bot_id): Path<Uuid>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.can_access_bot(&bot_id) {
        return Err(AuthError::BotAccessDenied);
    }

    let user = user.with_current_bot(bot_id);
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

pub async fn bot_owner_middleware(
    Path(bot_id): Path<Uuid>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.can_manage_bot(&bot_id) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}

pub async fn bot_operator_middleware(
    Path(bot_id): Path<Uuid>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let user = request
        .extensions()
        .get::<AuthenticatedUser>()
        .cloned()
        .unwrap_or_else(AuthenticatedUser::anonymous);

    if !user.can_operate_bot(&bot_id) {
        return Err(AuthError::InsufficientPermissions);
    }

    Ok(next.run(request).await)
}
