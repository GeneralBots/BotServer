use super::*;

impl std::fmt::Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Server => write!(f, "server"),
            Self::Client => write!(f, "client"),
            Self::Generator => write!(f, "generator"),
            Self::Designer => write!(f, "designer"),
            Self::Validation => write!(f, "validation"),
            Self::Runtime => write!(f, "runtime"),
        }
    }
}
