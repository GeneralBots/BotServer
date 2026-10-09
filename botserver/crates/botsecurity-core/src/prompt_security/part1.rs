use super::*;

impl Default for PromptSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_input_length: MAX_INPUT_LENGTH,
            max_output_length: MAX_OUTPUT_LENGTH,
            block_on_injection: true,
            sanitize_inputs: true,
            validate_outputs: true,
            log_suspicious_activity: true,
            jailbreak_detection: true,
            content_filtering: true,
        }
    }
}
