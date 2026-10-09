use super::*;

impl ApiKey {
    pub fn new(key: impl Into<SecretString>, provider: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            provider: provider.into(),
        }
    }

    pub fn expose_key(&self) -> &str {
        self.key.expose_secret()
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn is_empty(&self) -> bool {
        self.key.is_empty()
    }

    pub fn masked(&self) -> String {
        let key = self.key.expose_secret();
        if key.len() <= 8 {
            return "*".repeat(key.len());
        }
        format!(
            "{}...{}",
            &key[..4],
            &key[key.len() - 4..]
        )
    }
}
impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiKey")
            .field("key", &"[REDACTED]")
            .field("provider", &self.provider)
            .finish()
    }
}
impl fmt::Display for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ApiKey({}, {})", self.provider, self.masked())
    }
}
