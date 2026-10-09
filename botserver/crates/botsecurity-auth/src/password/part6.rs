use super::*;

pub(super) fn calculate_strength(password: &str, issues: &[PasswordIssue]) -> PasswordStrength {
    if !issues.is_empty() {
        let critical_issues = issues.iter().any(|i| {
            matches!(
                i,
                PasswordIssue::TooShort { .. }
                    | PasswordIssue::CommonPassword
                    | PasswordIssue::Compromised
            )
        });
        if critical_issues {
            return PasswordStrength::VeryWeak;
        }
        return PasswordStrength::Weak;
    }

    let length = password.len();
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    let unique_chars: HashSet<char> = password.chars().collect();

    let mut score = 0;

    if length >= 8 {
        score += 1;
    }
    if length >= 12 {
        score += 1;
    }
    if length >= 16 {
        score += 1;
    }
    if length >= 20 {
        score += 1;
    }

    if has_uppercase {
        score += 1;
    }
    if has_lowercase {
        score += 1;
    }
    if has_digit {
        score += 1;
    }
    if has_special {
        score += 2;
    }

    if unique_chars.len() >= 10 {
        score += 1;
    }
    if unique_chars.len() >= 15 {
        score += 1;
    }

    match score {
        0..=3 => PasswordStrength::VeryWeak,
        4..=5 => PasswordStrength::Weak,
        6..=7 => PasswordStrength::Fair,
        8..=9 => PasswordStrength::Strong,
        _ => PasswordStrength::VeryStrong,
    }
}
pub(super) fn estimate_crack_time(password: &str) -> String {
    let length = password.len();
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());

    let mut charset_size = 0;
    if has_lowercase {
        charset_size += 26;
    }
    if has_uppercase {
        charset_size += 26;
    }
    if has_digit {
        charset_size += 10;
    }
    if has_special {
        charset_size += 32;
    }
    if charset_size == 0 {
        charset_size = 26;
    }

    let guesses_per_second: f64 = 10_000_000_000.0;
    let combinations = (charset_size as f64).powi(length as i32);
    let seconds = combinations / guesses_per_second / 2.0;

    if seconds < 1.0 {
        "instantly".into()
    } else if seconds < 60.0 {
        format!("{:.0} seconds", seconds)
    } else if seconds < 3600.0 {
        format!("{:.0} minutes", seconds / 60.0)
    } else if seconds < 86400.0 {
        format!("{:.0} hours", seconds / 3600.0)
    } else if seconds < 31536000.0 {
        format!("{:.0} days", seconds / 86400.0)
    } else if seconds < 31536000.0 * 100.0 {
        format!("{:.0} years", seconds / 31536000.0)
    } else if seconds < 31536000.0 * 1000.0 {
        "centuries".into()
    } else {
        "millennia+".into()
    }
}
pub fn hash_password(password: &str) -> Result<String> {
    let hasher = PasswordHasher2::with_defaults()?;
    hasher.hash(password)
}
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let hasher = PasswordHasher2::with_defaults()?;
    hasher.verify(password, hash)
}
pub fn validate_password(password: &str) -> PasswordValidationResult {
    let hasher = match PasswordHasher2::with_defaults() {
        Ok(h) => h,
        Err(e) => {
            warn!("Failed to create password hasher: {e}");
            return PasswordValidationResult {
                is_valid: false,
                strength: PasswordStrength::VeryWeak,
                score: 0,
                issues: vec![],
                suggestions: vec!["Internal error during validation".into()],
                crack_time_display: "unknown".into(),
            };
        }
    };
    hasher.validate(password, None, None, &[])
}
pub fn generate_secure_password(length: usize) -> String {
    use rand::Rng;

    const UPPERCASE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const LOWERCASE: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const DIGITS: &[u8] = b"0123456789";
    const SPECIAL: &[u8] = b"!@#$%^&*()_+-=[]{}|;:,.<>?";

    let length = length.max(16);
    let mut rng = rand::rng();
    let mut password = Vec::with_capacity(length);

    password.push(UPPERCASE[rng.random_range(0..UPPERCASE.len())]);
    password.push(LOWERCASE[rng.random_range(0..LOWERCASE.len())]);
    password.push(DIGITS[rng.random_range(0..DIGITS.len())]);
    password.push(SPECIAL[rng.random_range(0..SPECIAL.len())]);

    let all_chars: Vec<u8> = [UPPERCASE, LOWERCASE, DIGITS, SPECIAL].concat();
    for _ in 4..length {
        password.push(all_chars[rng.random_range(0..all_chars.len())]);
    }

    for i in (1..password.len()).rev() {
        let j = rng.random_range(0..=i);
        password.swap(i, j);
    }

    String::from_utf8(password).unwrap_or_else(|_| {
        debug!("Generated password contained invalid UTF-8, regenerating");
        generate_secure_password(length)
    })
}
pub fn generate_recovery_codes(count: usize) -> Vec<String> {
    use rand::Rng;

    const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::rng();
    let mut codes = Vec::with_capacity(count);

    for _ in 0..count {
        let code: String = (0..8)
            .map(|_| CHARS[rng.random_range(0..CHARS.len())] as char)
            .collect();
        let formatted = format!("{}-{}", &code[..4], &code[4..]);
        codes.push(formatted);
    }

    codes
}
