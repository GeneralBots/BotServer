use axum::response::IntoResponse;

pub use botsecurity::*;
pub use botsecurity_core::response_error::ResponseError;

pub mod code_scan_fixes;
pub mod rbac_role_resolver;
pub mod user_role;

pub use rbac_role_resolver::*;

/// Renders a sanitized error as a boxed response error.
///
/// Handler signatures used to carry the whole `SafeErrorResponse`; the boxed
/// wrapper keeps every `Result` in the call chain small
/// (`clippy::result_large_err`) while rendering an identical response.
pub fn boxed_error(error: crate::security::error_sanitizer::SafeErrorResponse) -> ResponseError {
    ResponseError::new(error.into_response())
}
