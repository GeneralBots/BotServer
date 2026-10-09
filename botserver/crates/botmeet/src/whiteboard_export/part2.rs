use super::*;

impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(s) => write!(f, "Invalid format: {s}"),
            Self::RenderError(s) => write!(f, "Render error: {s}"),
            Self::RenderFailed(s) => write!(f, "Render failed: {s}"),
            Self::IoError(s) => write!(f, "IO error: {s}"),
            Self::EmptyCanvas => write!(f, "Empty canvas"),
            Self::InvalidDimensions => write!(f, "Invalid dimensions"),
        }
    }
}
impl std::error::Error for ExportError {}
