use super::*;

impl InjectionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SystemPromptLeak => "system_prompt_leak",
            Self::RoleManipulation => "role_manipulation",
            Self::InstructionOverride => "instruction_override",
            Self::DelimiterInjection => "delimiter_injection",
            Self::EncodingBypass => "encoding_bypass",
            Self::ContextManipulation => "context_manipulation",
            Self::JailbreakAttempt => "jailbreak_attempt",
            Self::DataExfiltration => "data_exfiltration",
            Self::PromptLeaking => "prompt_leaking",
            Self::RecursiveInjection => "recursive_injection",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::SystemPromptLeak => "Attempt to reveal system instructions",
            Self::RoleManipulation => "Attempt to change AI role or persona",
            Self::InstructionOverride => "Attempt to override safety instructions",
            Self::DelimiterInjection => "Use of special delimiters to escape context",
            Self::EncodingBypass => "Use of encoding to bypass filters",
            Self::ContextManipulation => "Attempt to manipulate conversation context",
            Self::JailbreakAttempt => "Attempt to bypass safety restrictions",
            Self::DataExfiltration => "Attempt to extract sensitive information",
            Self::PromptLeaking => "Attempt to leak prompt or instructions",
            Self::RecursiveInjection => "Nested or recursive injection attempt",
        }
    }

    pub fn threat_level(&self) -> ThreatLevel {
        match self {
            Self::SystemPromptLeak | Self::PromptLeaking => ThreatLevel::High,
            Self::RoleManipulation | Self::JailbreakAttempt => ThreatLevel::High,
            Self::InstructionOverride => ThreatLevel::Critical,
            Self::DelimiterInjection => ThreatLevel::Medium,
            Self::EncodingBypass => ThreatLevel::Medium,
            Self::ContextManipulation => ThreatLevel::Low,
            Self::DataExfiltration => ThreatLevel::Critical,
            Self::RecursiveInjection => ThreatLevel::High,
        }
    }
}
