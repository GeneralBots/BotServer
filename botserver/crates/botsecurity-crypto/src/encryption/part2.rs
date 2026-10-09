use super::*;

impl EncryptionAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aes256Gcm => "AES-256-GCM",
        }
    }
}
