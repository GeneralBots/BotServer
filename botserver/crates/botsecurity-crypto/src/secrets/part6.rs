use super::*;

impl SecretsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_api_key(&mut self, name: impl Into<String>, key: ApiKey) {
        self.api_keys.insert(name.into(), key);
    }

    pub fn get_api_key(&self, name: &str) -> Option<&ApiKey> {
        self.api_keys.get(name)
    }

    pub fn remove_api_key(&mut self, name: &str) -> Option<ApiKey> {
        self.api_keys.remove(name)
    }

    pub fn set_database_credentials(&mut self, creds: DatabaseCredentials) {
        self.database_credentials = Some(creds);
    }

    pub fn database_credentials(&self) -> Option<&DatabaseCredentials> {
        self.database_credentials.as_ref()
    }

    pub fn set_jwt_secret(&mut self, secret: JwtSecret) {
        self.jwt_secret = Some(secret);
    }

    pub fn jwt_secret(&self) -> Option<&JwtSecret> {
        self.jwt_secret.as_ref()
    }

    pub fn add_custom_secret(&mut self, name: impl Into<String>, secret: impl Into<SecretString>) {
        self.custom_secrets.insert(name.into(), secret.into());
    }

    pub fn get_custom_secret(&self, name: &str) -> Option<&SecretString> {
        self.custom_secrets.get(name)
    }

    pub fn remove_custom_secret(&mut self, name: &str) -> Option<SecretString> {
        self.custom_secrets.remove(name)
    }

    pub fn clear(&mut self) {
        self.api_keys.clear();
        self.database_credentials = None;
        self.jwt_secret = None;
        self.custom_secrets.clear();
    }
}
impl fmt::Debug for SecretsStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretsStore")
            .field("api_keys_count", &self.api_keys.len())
            .field("has_database_credentials", &self.database_credentials.is_some())
            .field("has_jwt_secret", &self.jwt_secret.is_some())
            .field("custom_secrets_count", &self.custom_secrets.len())
            .finish()
    }
}
#[inline(never)]
pub(super) fn zeroize_string(s: &mut String) {
    // Overwrite with zeros using safe code
    let len = s.len();
    s.clear();
    s.reserve(len);
    for _ in 0..len {
        s.push('\0');
    }
    s.clear();
}
#[inline(never)]
pub(super) fn zeroize_bytes(v: &mut Vec<u8>) {
    // Overwrite with zeros using safe code
    for byte in v.iter_mut() {
        *byte = 0;
    }
    v.clear();
}
pub fn redact_sensitive_data(text: &str) -> String {
    let patterns = [
        (r"password[=:]\s*\S+", "password=[REDACTED]"),
        (r"api[_-]?key[=:]\s*\S+", "api_key=[REDACTED]"),
        (r"token[=:]\s*\S+", "token=[REDACTED]"),
        (r"secret[=:]\s*\S+", "secret=[REDACTED]"),
        (r"Bearer\s+\S+", "Bearer [REDACTED]"),
        (r"Basic\s+\S+", "Basic [REDACTED]"),
    ];

    let mut result = text.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = regex::Regex::new(&format!("(?i){}", pattern)) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    result
}
pub fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    let sensitive_keywords = [
        "password",
        "passwd",
        "secret",
        "token",
        "api_key",
        "apikey",
        "auth",
        "credential",
        "private",
        "key",
        "cert",
        "certificate",
    ];

    sensitive_keywords.iter().any(|kw| lower.contains(kw))
}
