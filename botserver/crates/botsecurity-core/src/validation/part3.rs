use super::*;

pub fn validate_required<'a>(value: Option<&'a str>, field_name: &str) -> Result<&'a str, ValidationError> {
    match value {
        Some(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(ValidationError::Required(field_name.to_string())),
    }
}
pub fn validate_string_required(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        Err(ValidationError::Required(field_name.to_string()))
    } else {
        Ok(())
    }
}
pub fn validate_length(
    value: &str,
    field_name: &str,
    min: Option<usize>,
    max: Option<usize>,
) -> Result<(), ValidationError> {
    let len = value.len();

    if let Some(min_len) = min {
        if len < min_len {
            return Err(ValidationError::TooShort {
                field: field_name.to_string(),
                min: min_len,
                actual: len,
            });
        }
    }

    if let Some(max_len) = max {
        if len > max_len {
            return Err(ValidationError::TooLong {
                field: field_name.to_string(),
                max: max_len,
                actual: len,
            });
        }
    }

    Ok(())
}
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    if email.len() > 254 {
        return Err(ValidationError::InvalidEmail(email.to_string()));
    }

    if EMAIL_REGEX.as_ref().is_some_and(|re| re.is_match(email)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidEmail(email.to_string()))
    }
}
pub fn validate_url(url: &str) -> Result<(), ValidationError> {
    if url.len() > 2048 {
        return Err(ValidationError::InvalidUrl(url.to_string()));
    }

    if URL_REGEX.as_ref().is_some_and(|re| re.is_match(url)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidUrl(url.to_string()))
    }
}
pub fn validate_uuid(uuid: &str) -> Result<(), ValidationError> {
    if UUID_REGEX.as_ref().is_some_and(|re| re.is_match(uuid)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidUuid(uuid.to_string()))
    }
}
pub fn validate_phone(phone: &str) -> Result<(), ValidationError> {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();

    if PHONE_REGEX.as_ref().is_some_and(|re| re.is_match(&digits)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidPhone(phone.to_string()))
    }
}
pub fn validate_alphanumeric(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if ALPHANUMERIC_REGEX.as_ref().is_some_and(|re| re.is_match(value)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat {
            field: field_name.to_string(),
            expected: "alphanumeric characters only".to_string(),
        })
    }
}
pub fn validate_slug(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if SLUG_REGEX.as_ref().is_some_and(|re| re.is_match(value)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat {
            field: field_name.to_string(),
            expected: "lowercase alphanumeric with hyphens".to_string(),
        })
    }
}
pub fn validate_username(value: &str) -> Result<(), ValidationError> {
    if USERNAME_REGEX.as_ref().is_some_and(|re| re.is_match(value)) {
        Ok(())
    } else {
        Err(ValidationError::InvalidFormat {
            field: "username".to_string(),
            expected: "3-32 chars, starting with letter, alphanumeric with _ and -".to_string(),
        })
    }
}
pub fn validate_range<T: PartialOrd + std::fmt::Display>(
    value: T,
    field_name: &str,
    min: Option<T>,
    max: Option<T>,
) -> Result<(), ValidationError> {
    let min_str = min.as_ref().map(|m| m.to_string()).unwrap_or_else(|| "-∞".to_string());
    let max_str = max.as_ref().map(|m| m.to_string()).unwrap_or_else(|| "∞".to_string());

    if let Some(ref min_val) = min {
        if value < *min_val {
            return Err(ValidationError::InvalidRange {
                field: field_name.to_string(),
                min: min_str,
                max: max_str,
            });
        }
    }

    if let Some(ref max_val) = max {
        if value > *max_val {
            return Err(ValidationError::InvalidRange {
                field: field_name.to_string(),
                min: min_str,
                max: max_str,
            });
        }
    }

    Ok(())
}
pub fn validate_one_of<T: PartialEq + std::fmt::Debug>(
    value: &T,
    field_name: &str,
    allowed: &[T],
) -> Result<(), ValidationError> {
    if allowed.contains(value) {
        Ok(())
    } else {
        Err(ValidationError::InvalidValue {
            field: field_name.to_string(),
            message: format!("must be one of {:?}", allowed),
        })
    }
}
pub fn validate_not_in<T: PartialEq + std::fmt::Debug>(
    value: &T,
    field_name: &str,
    forbidden: &[T],
    reason: &str,
) -> Result<(), ValidationError> {
    if forbidden.contains(value) {
        Err(ValidationError::Forbidden {
            field: field_name.to_string(),
            reason: reason.to_string(),
        })
    } else {
        Ok(())
    }
}
pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    if password.len() < 8 {
        return Err(ValidationError::TooShort {
            field: "password".to_string(),
            min: 8,
            actual: password.len(),
        });
    }

    if password.len() > 128 {
        return Err(ValidationError::TooLong {
            field: "password".to_string(),
            max: 128,
            actual: password.len(),
        });
    }

    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    let strength_score = [has_uppercase, has_lowercase, has_digit, has_special]
        .iter()
        .filter(|&&x| x)
        .count();

    if strength_score < 3 {
        return Err(ValidationError::InvalidValue {
            field: "password".to_string(),
            message: "must contain at least 3 of: uppercase, lowercase, digit, special char".to_string(),
        });
    }

    Ok(())
}
pub fn sanitize_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}
pub fn strip_html_tags(input: &str) -> String {
    static HTML_TAG_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"<[^>]*>").ok());

    match HTML_TAG_REGEX.as_ref() {
        Some(re) => re.replace_all(input, "").to_string(),
        // Fail closed: without the sanitizer, strip everything that could be a tag.
        None => String::new(),
    }
}
pub fn validate_no_html(value: &str, field_name: &str) -> Result<(), ValidationError> {
    if value.contains('<') || value.contains('>') {
        Err(ValidationError::Forbidden {
            field: field_name.to_string(),
            reason: "HTML tags are not allowed".to_string(),
        })
    } else {
        Ok(())
    }
}
pub fn validate_no_script_injection(value: &str, field_name: &str) -> Result<(), ValidationError> {
    let lower = value.to_lowercase();

    let dangerous_patterns = [
        "javascript:",
        "vbscript:",
        "data:",
        "onclick",
        "onerror",
        "onload",
        "onmouseover",
        "onfocus",
        "onblur",
        "<script",
        "</script",
        "eval(",
        "expression(",
    ];

    for pattern in dangerous_patterns {
        if lower.contains(pattern) {
            return Err(ValidationError::Forbidden {
                field: field_name.to_string(),
                reason: format!("Potentially dangerous pattern '{}' detected", pattern),
            });
        }
    }

    Ok(())
}
