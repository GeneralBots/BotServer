use super::*;

impl std::fmt::Display for PathGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathTraversal => write!(f, "Path traversal attempt detected"),
            Self::AbsolutePath => write!(f, "Absolute paths are not allowed"),
            Self::InvalidComponent => write!(f, "Invalid path component"),
            Self::EmptyPath => write!(f, "Empty path is not allowed"),
            Self::OutsideAllowedRoot => write!(f, "Path is outside allowed root directory"),
            Self::SymlinkNotAllowed => write!(f, "Symbolic links are not allowed"),
            Self::HiddenFileNotAllowed => write!(f, "Hidden files are not allowed"),
            Self::InvalidExtension => write!(f, "File extension is not allowed"),
            Self::PathTooLong => write!(f, "Path exceeds maximum length"),
            Self::NullByte => write!(f, "Path contains null byte"),
        }
    }
}
impl std::error::Error for PathGuardError {}
