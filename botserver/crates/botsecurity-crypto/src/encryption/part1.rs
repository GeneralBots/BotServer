use super::*;

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_rotation_days: 90,
            envelope_encryption: true,
            compress_before_encrypt: false,
        }
    }
}
