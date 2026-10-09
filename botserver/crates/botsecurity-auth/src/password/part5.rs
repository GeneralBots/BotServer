use super::*;

impl PasswordHasher2 {
    pub fn new(argon2_config: Argon2Config, password_config: PasswordConfig) -> Result<Self> {
        let params = Params::new(
            argon2_config.memory_cost_kib,
            argon2_config.time_cost,
            argon2_config.parallelism,
            Some(argon2_config.output_length),
        )
        .map_err(|e| anyhow!("Invalid Argon2 parameters: {e}"))?;

        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        Ok(Self {
            argon2,
            config: password_config,
        })
    }

    pub fn with_defaults() -> Result<Self> {
        Self::new(Argon2Config::default(), PasswordConfig::default())
    }

    pub fn hash(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {e}"))?;

        Ok(hash.to_string())
    }

    pub fn verify(&self, password: &str, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash format: {e}"))?;

        match self.argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::Password) => Ok(false),
            Err(e) => Err(anyhow!("Password verification failed: {e}")),
        }
    }

    pub fn needs_rehash(&self, hash: &str) -> Result<bool> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash format: {e}"))?;

        let algorithm = parsed_hash.algorithm;
        if algorithm != argon2::ARGON2ID_IDENT {
            return Ok(true);
        }

        if let Some(m_param) = parsed_hash.params.get_str("m") {
            if let Ok(memory) = m_param.parse::<u32>() {
                if memory < self.argon2.params().m_cost() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub fn validate(
        &self,
        password: &str,
        username: Option<&str>,
        email: Option<&str>,
        previous_hashes: &[String],
    ) -> PasswordValidationResult {
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        let length = password.len();
        if length < self.config.min_length {
            issues.push(PasswordIssue::TooShort {
                min: self.config.min_length,
                actual: length,
            });
        }
        if length > self.config.max_length {
            issues.push(PasswordIssue::TooLong {
                max: self.config.max_length,
                actual: length,
            });
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password.chars().any(|c| !c.is_alphanumeric());

        if self.config.require_uppercase && !has_uppercase {
            issues.push(PasswordIssue::MissingUppercase);
            suggestions.push("Add uppercase letters (A-Z)".into());
        }
        if self.config.require_lowercase && !has_lowercase {
            issues.push(PasswordIssue::MissingLowercase);
            suggestions.push("Add lowercase letters (a-z)".into());
        }
        if self.config.require_digit && !has_digit {
            issues.push(PasswordIssue::MissingDigit);
            suggestions.push("Add numbers (0-9)".into());
        }
        if self.config.require_special && !has_special {
            issues.push(PasswordIssue::MissingSpecial);
            suggestions.push("Add special characters (!@#$%^&*)".into());
        }

        let unique_chars: HashSet<char> = password.chars().collect();
        if unique_chars.len() < self.config.min_unique_chars {
            issues.push(PasswordIssue::InsufficientUniqueChars {
                min: self.config.min_unique_chars,
                actual: unique_chars.len(),
            });
            suggestions.push("Use more varied characters".into());
        }

        if has_consecutive_chars(password, self.config.max_consecutive_chars) {
            issues.push(PasswordIssue::TooManyConsecutiveChars {
                max: self.config.max_consecutive_chars,
            });
            suggestions.push("Avoid repeating characters".into());
        }

        if is_common_password(password) {
            issues.push(PasswordIssue::CommonPassword);
            suggestions.push("Choose a less common password".into());
        }

        let password_lower = password.to_lowercase();
        if let Some(uname) = username {
            if !uname.is_empty() && password_lower.contains(&uname.to_lowercase()) {
                issues.push(PasswordIssue::ContainsUsername);
                suggestions.push("Remove your username from the password".into());
            }
        }

        if let Some(mail) = email {
            let email_parts: Vec<&str> = mail.split('@').collect();
            if let Some(local_part) = email_parts.first() {
                if !local_part.is_empty() && password_lower.contains(&local_part.to_lowercase()) {
                    issues.push(PasswordIssue::ContainsEmail);
                    suggestions.push("Remove your email from the password".into());
                }
            }
        }

        for prev_hash in previous_hashes.iter().take(self.config.password_history_count) {
            if self.verify(password, prev_hash).unwrap_or(false) {
                issues.push(PasswordIssue::RecentlyUsed);
                suggestions.push("Choose a password you haven't used before".into());
                break;
            }
        }

        let strength = calculate_strength(password, &issues);
        let score = strength.score();
        let is_valid = issues.is_empty() && strength.is_acceptable();
        let crack_time_display = estimate_crack_time(password);

        PasswordValidationResult {
            is_valid,
            strength,
            score,
            issues,
            suggestions,
            crack_time_display,
        }
    }

    pub fn config(&self) -> &PasswordConfig {
        &self.config
    }
}
pub(super) fn has_consecutive_chars(password: &str, max: usize) -> bool {
    let chars: Vec<char> = password.chars().collect();
    let mut count = 1;

    for i in 1..chars.len() {
        if chars[i] == chars[i - 1] {
            count += 1;
            if count > max {
                return true;
            }
        } else {
            count = 1;
        }
    }
    false
}
pub(super) fn is_common_password(password: &str) -> bool {
    const COMMON_PASSWORDS: &[&str] = &[
        "password",
        "123456",
        "12345678",
        "qwerty",
        "abc123",
        "monkey",
        "1234567",
        "letmein",
        "trustno1",
        "dragon",
        "baseball",
        "iloveyou",
        "master",
        "sunshine",
        "ashley",
        "bailey",
        "shadow",
        "123123",
        "654321",
        "superman",
        "qazwsx",
        "michael",
        "football",
        "password1",
        "password123",
        "welcome",
        "welcome1",
        "admin",
        "admin123",
        "root",
        "toor",
        "pass",
        "test",
        "guest",
        "changeme",
        "default",
        "secret",
        "login",
        "passw0rd",
        "p@ssword",
        "p@ssw0rd",
        "qwerty123",
        "azerty",
        "000000",
        "111111",
        "1234567890",
        "0987654321",
    ];

    let lower = password.to_lowercase();
    COMMON_PASSWORDS.iter().any(|&common| lower == common || lower.contains(common))
}
