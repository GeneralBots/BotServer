use super::*;

impl OutputIssue {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SystemPromptLeaked => "system_prompt_leaked",
            Self::SensitiveDataExposed => "sensitive_data_exposed",
            Self::MaliciousContent => "malicious_content",
            Self::ExcessiveLength => "excessive_length",
            Self::InvalidFormat => "invalid_format",
            Self::HarmfulContent => "harmful_content",
        }
    }
}
