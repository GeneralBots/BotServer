use super::*;

impl From<&str> for CodeLanguage {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "python" | "py" => Self::Python,
            "javascript" | "js" | "node" => Self::JavaScript,
            _ => Self::Bash,
        }
    }
}
impl CodeLanguage {
    pub fn file_extension(&self) -> &str {
        match self {
            Self::Python => "py",
            Self::JavaScript => "js",
            Self::Bash => "sh",
        }
    }

    pub fn interpreter(&self) -> &str {
        match self {
            Self::Python => "python3",
            Self::JavaScript => "node",
            Self::Bash => "bash",
        }
    }

    pub fn lxc_image(&self) -> &str {
        match self {
            Self::Python => "gb-sandbox-python",
            Self::JavaScript => "gb-sandbox-node",
            Self::Bash => "gb-sandbox-base",
        }
    }
}
