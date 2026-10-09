use super::*;

impl SecretString {
    pub fn new(secret: String) -> Self {
        Self { inner: secret }
    }
}
impl std::str::FromStr for SecretString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: s.to_string(),
        })
    }
}
impl SecretString {

    pub fn expose_secret(&self) -> &str {
        &self.inner
    }

    pub fn expose_secret_mut(&mut self) -> &mut String {
        &mut self.inner
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
impl Drop for SecretString {
    fn drop(&mut self) {
        zeroize_string(&mut self.inner);
    }
}
impl Clone for SecretString {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
impl fmt::Display for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
impl From<String> for SecretString {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}
impl From<&str> for SecretString {
    fn from(s: &str) -> Self {
        s.parse().unwrap_or_default()
    }
}
