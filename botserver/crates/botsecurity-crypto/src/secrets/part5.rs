use super::*;

impl JwtSecret {
    pub fn new(secret: impl Into<SecretBytes>, algorithm: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            algorithm: algorithm.into(),
        }
    }

    pub fn expose_secret(&self) -> &[u8] {
        self.secret.expose_secret()
    }

    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }
}
impl fmt::Debug for JwtSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JwtSecret")
            .field("secret", &"[REDACTED]")
            .field("algorithm", &self.algorithm)
            .finish()
    }
}
