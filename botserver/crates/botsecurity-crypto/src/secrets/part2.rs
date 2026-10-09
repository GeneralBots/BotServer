use super::*;

impl SecretBytes {
    pub fn new(secret: Vec<u8>) -> Self {
        Self { inner: secret }
    }

    pub fn expose_secret(&self) -> &[u8] {
        &self.inner
    }

    pub fn expose_secret_mut(&mut self) -> &mut Vec<u8> {
        &mut self.inner
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
impl Drop for SecretBytes {
    fn drop(&mut self) {
        zeroize_bytes(&mut self.inner);
    }
}
impl Clone for SecretBytes {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl fmt::Debug for SecretBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED BYTES]")
    }
}
impl From<Vec<u8>> for SecretBytes {
    fn from(v: Vec<u8>) -> Self {
        Self::new(v)
    }
}
impl From<&[u8]> for SecretBytes {
    fn from(s: &[u8]) -> Self {
        Self::new(s.to_vec())
    }
}
