use super::*;

impl<T: Serialize> ApiResponse<T> {
    pub(super) fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}
impl ApiResponse<()> {
    pub(super) fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}
