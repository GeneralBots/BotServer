//! A boxed HTTP failure for handler signatures.
//!
//! Helpers that build their own failure as an [`Response`] used to declare
//! `Result<T, Response>`. A `Response` is around 128 bytes, so every `Result`
//! in the call chain grew by that much on the success path too — enough for
//! `clippy::result_large_err` to fire, and enough to inflate the futures those
//! helpers return. [`ResponseError`] keeps the response behind a pointer and
//! still renders exactly the response the failing branch built.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::fmt;

/// An HTTP failure that is already rendered.
#[derive(Debug)]
pub struct ResponseError(Box<Response>);

impl ResponseError {
    /// Wraps a rendered response.
    pub fn new(response: Response) -> Self {
        Self(Box::new(response))
    }

    /// The status the wrapped response carries.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.0.status()
    }
}

impl From<Response> for ResponseError {
    fn from(response: Response) -> Self {
        Self::new(response)
    }
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP error response with status {}", self.0.status())
    }
}

impl std::error::Error for ResponseError {}

impl IntoResponse for ResponseError {
    fn into_response(self) -> Response {
        *self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_original_status() {
        let error = ResponseError::new(StatusCode::FORBIDDEN.into_response());
        assert_eq!(error.status(), StatusCode::FORBIDDEN);
        assert_eq!(error.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn stays_pointer_sized() {
        assert_eq!(
            std::mem::size_of::<ResponseError>(),
            std::mem::size_of::<Box<Response>>()
        );
    }
}
