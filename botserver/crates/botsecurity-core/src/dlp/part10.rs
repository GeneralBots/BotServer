use super::*;

pub(super) fn extract_context(content: &str, start: usize, end: usize, padding: usize) -> String {
    let context_start = start.saturating_sub(padding);
    let context_end = (end + padding).min(content.len());

    let mut context = String::new();

    if context_start > 0 {
        context.push_str("...");
    }

    context.push_str(&content[context_start..context_end]);

    if context_end < content.len() {
        context.push_str("...");
    }

    context
}
pub fn validate_credit_card_checksum(number: &str) -> bool {
    let digits: Vec<u32> = number
        .chars()
        .filter(|c| c.is_ascii_digit())
        .filter_map(|c| c.to_digit(10))
        .collect();

    if digits.len() < 13 || digits.len() > 19 {
        return false;
    }

    let mut sum = 0;
    let mut double = false;

    for digit in digits.iter().rev() {
        let mut d = *digit;
        if double {
            d *= 2;
            if d > 9 {
                d -= 9;
            }
        }
        sum += d;
        double = !double;
    }

    sum % 10 == 0
}
pub fn create_default_policies() -> Vec<DlpPolicy> {
    vec![
        DlpPolicy::new(
            "Block Credit Cards",
            vec![SensitiveDataType::CreditCard],
            DlpAction::Block,
        )
        .with_description("Block transmission of credit card numbers"),
        DlpPolicy::new(
            "Block SSN",
            vec![SensitiveDataType::Ssn],
            DlpAction::Block,
        )
        .with_description("Block transmission of Social Security Numbers"),
        DlpPolicy::new(
            "Warn on API Keys",
            vec![
                SensitiveDataType::ApiKey,
                SensitiveDataType::AwsAccessKey,
                SensitiveDataType::AwsSecretKey,
            ],
            DlpAction::Warn,
        )
        .with_description("Warn when API keys or secrets are detected"),
        DlpPolicy::new(
            "Redact Emails",
            vec![SensitiveDataType::Email],
            DlpAction::Redact,
        )
        .with_description("Redact email addresses in outbound content")
        .with_scope(PolicyScope::Outbound),
        DlpPolicy::new(
            "Block Private Keys",
            vec![SensitiveDataType::PrivateKey],
            DlpAction::Block,
        )
        .with_description("Block transmission of private keys"),
    ]
}
