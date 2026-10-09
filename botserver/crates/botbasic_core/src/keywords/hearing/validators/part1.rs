use super::*;

#[must_use]
pub fn validate_input(input: &str, input_type: &InputType) -> ValidationResult {
    let trimmed = input.trim();

    match input_type {
        InputType::Any
        | InputType::QrCode
        | InputType::File
        | InputType::Image
        | InputType::Audio
        | InputType::Video
        | InputType::Document
        | InputType::Login => ValidationResult::valid(trimmed.to_string()),
        InputType::Email => validate_email(trimmed),
        InputType::Date => validate_date(trimmed),
        InputType::Name => validate_name(trimmed),
        InputType::Integer => validate_integer(trimmed),
        InputType::Float => validate_float(trimmed),
        InputType::Boolean => validate_boolean(trimmed),
        InputType::Hour => validate_hour(trimmed),
        InputType::Money => validate_money(trimmed),
        InputType::Mobile => validate_mobile(trimmed),
        InputType::Zipcode => validate_zipcode(trimmed),
        InputType::Language => validate_language(trimmed),
        InputType::Cpf => validate_cpf(trimmed),
        InputType::Cnpj => validate_cnpj(trimmed),
        InputType::Url => validate_url(trimmed),
        InputType::Uuid => validate_uuid(trimmed),
        InputType::Color => validate_color(trimmed),
        InputType::CreditCard => validate_credit_card(trimmed),
        InputType::Password => validate_password(trimmed),
        InputType::Menu(options) => validate_menu(trimmed, options),
    }
}
pub(super) fn validate_email(input: &str) -> ValidationResult {
    let Some(email_regex) = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$").ok() else {
        return ValidationResult::invalid("internal validation error".to_string());
    };

    if email_regex.is_match(input) {
        ValidationResult::valid(input.to_lowercase())
    } else {
        ValidationResult::invalid(InputType::Email.error_message())
    }
}
pub(super) fn validate_date(input: &str) -> ValidationResult {
    let formats = [
        "%d/%m/%Y", "%d-%m-%Y", "%Y-%m-%d", "%Y/%m/%d", "%d.%m.%Y", "%m/%d/%Y", "%d %b %Y",
        "%d %B %Y",
    ];

    for format in &formats {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(input, format) {
            return ValidationResult::valid_with_metadata(
                date.format("%Y-%m-%d").to_string(),
                serde_json::json!({
                    "original": input,
                    "parsed_format": *format
                }),
            );
        }
    }

    let lower = input.to_lowercase();
    let today = chrono::Local::now().date_naive();

    if lower == "today" || lower == "hoje" {
        return ValidationResult::valid(today.format("%Y-%m-%d").to_string());
    }
    if lower == "tomorrow" || lower == "amanhã" || lower == "amanha" {
        return ValidationResult::valid(
            (today + chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        );
    }
    if lower == "yesterday" || lower == "ontem" {
        return ValidationResult::valid(
            (today - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string(),
        );
    }

    ValidationResult::invalid(InputType::Date.error_message())
}
pub(super) fn validate_name(input: &str) -> ValidationResult {
    let Some(name_regex) = Regex::new(r"^[\p{L}\s\-']+$").ok() else {
        return ValidationResult::invalid("internal validation error".to_string());
    };

    if input.len() < 2 {
        return ValidationResult::invalid("Name must be at least 2 characters".to_string());
    }

    if input.len() > 100 {
        return ValidationResult::invalid("Name is too long".to_string());
    }

    if name_regex.is_match(input) {
        let normalized = input
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        ValidationResult::valid(normalized)
    } else {
        ValidationResult::invalid(InputType::Name.error_message())
    }
}
pub(super) fn validate_integer(input: &str) -> ValidationResult {
    let cleaned = input.replace([',', '.', ' '], "").trim().to_string();

    match cleaned.parse::<i64>() {
        Ok(num) => ValidationResult::valid_with_metadata(
            num.to_string(),
            serde_json::json!({ "value": num }),
        ),
        Err(_) => ValidationResult::invalid(InputType::Integer.error_message()),
    }
}
pub(super) fn validate_float(input: &str) -> ValidationResult {
    let cleaned = input.replace(' ', "").replace(',', ".").trim().to_string();

    match cleaned.parse::<f64>() {
        Ok(num) => ValidationResult::valid_with_metadata(
            format!("{:.2}", num),
            serde_json::json!({ "value": num }),
        ),
        Err(_) => ValidationResult::invalid(InputType::Float.error_message()),
    }
}
pub(super) fn validate_boolean(input: &str) -> ValidationResult {
    let lower = input.to_lowercase();

    let true_values = [
        "yes",
        "y",
        "true",
        "1",
        "sim",
        "s",
        "si",
        "oui",
        "ja",
        "da",
        "ok",
        "yeah",
        "yep",
        "sure",
        "confirm",
        "confirmed",
        "accept",
        "agreed",
        "agree",
    ];

    let false_values = [
        "no", "n", "false", "0", "não", "nao", "non", "nein", "net", "nope", "cancel", "deny",
        "denied", "reject", "declined", "disagree",
    ];

    if true_values.contains(&lower.as_str()) {
        ValidationResult::valid_with_metadata(
            "true".to_string(),
            serde_json::json!({ "value": true }),
        )
    } else if false_values.contains(&lower.as_str()) {
        ValidationResult::valid_with_metadata(
            "false".to_string(),
            serde_json::json!({ "value": false }),
        )
    } else {
        ValidationResult::invalid(InputType::Boolean.error_message())
    }
}
pub(super) fn validate_hour(input: &str) -> ValidationResult {
    let Some(time_24_regex) = Regex::new(r"^([01]?\d|2[0-3]):([0-5]\d)$").ok() else {
        return ValidationResult::invalid("internal validation error".to_string());
    };
    if let Some(caps) = time_24_regex.captures(input) {
        let hour: u32 = caps[1].parse().unwrap_or_default();
        let minute: u32 = caps[2].parse().unwrap_or_default();
        return ValidationResult::valid_with_metadata(
            format!("{:02}:{:02}", hour, minute),
            serde_json::json!({ "hour": hour, "minute": minute }),
        );
    }

    if let Some(caps) = Regex::new(
        r"^(1[0-2]|0?[1-9]):([0-5]\d)\s*(AM|PM|am|pm|a\.m\.|p\.m\.)$",
    ).ok().and_then(|re| re.captures(input)) {
        let mut hour: u32 = caps[1].parse().unwrap_or_default();
        let minute: u32 = caps[2].parse().unwrap_or_default();
        let period = caps[3].to_uppercase();

        if period.starts_with('P') && hour != 12 {
            hour += 12;
        } else if period.starts_with('A') && hour == 12 {
            hour = 0;
        }

        return ValidationResult::valid_with_metadata(
            format!("{:02}:{:02}", hour, minute),
            serde_json::json!({ "hour": hour, "minute": minute }),
        );
    }

    ValidationResult::invalid(InputType::Hour.error_message())
}
pub(super) fn validate_money(input: &str) -> ValidationResult {
    let cleaned = input
        .replace("R$", "")
        .replace(['$', '€', '£', '¥', ' '], "")
        .trim()
        .to_string();

    let normalized = if cleaned.contains(',') && cleaned.contains('.') {
        let last_comma = cleaned.rfind(',').unwrap_or(0);
        let last_dot = cleaned.rfind('.').unwrap_or(0);

        if last_comma > last_dot {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };

    match normalized.parse::<f64>() {
        Ok(amount) if amount >= 0.0 => ValidationResult::valid_with_metadata(
            format!("{:.2}", amount),
            serde_json::json!({ "value": amount }),
        ),
        _ => ValidationResult::invalid(InputType::Money.error_message()),
    }
}
pub(super) fn validate_mobile(input: &str) -> ValidationResult {
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.len() < 10 || digits.len() > 15 {
        return ValidationResult::invalid(InputType::Mobile.error_message());
    }

    let formatted = match digits.len() {
        11 => format!("({}) {}-{}", &digits[0..2], &digits[2..7], &digits[7..11]),
        10 => format!("({}) {}-{}", &digits[0..3], &digits[3..6], &digits[6..10]),
        _ => format!("+{digits}"),
    };

    ValidationResult::valid_with_metadata(
        formatted.clone(),
        serde_json::json!({ "digits": digits, "formatted": formatted }),
    )
}
