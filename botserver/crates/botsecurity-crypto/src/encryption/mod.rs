pub(crate) use aes_gcm::{ aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce, };
pub(crate) use anyhow::{anyhow, Result};
pub(crate) use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
pub(crate) use rand::Rng;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use sha2::{Digest, Sha256};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use tracing::info;
pub(crate) use uuid::Uuid;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const SALT_SIZE: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub algorithm: EncryptionAlgorithm,
    pub key_rotation_days: u32,
    pub envelope_encryption: bool,
    pub compress_before_encrypt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    Aes256Gcm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    pub algorithm: String,
    pub nonce: String,
    pub ciphertext: String,
    pub key_id: Option<String>,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionKey {
    pub id: String,
    pub key_data: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub is_primary: bool,
    pub purpose: KeyPurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPurpose {
    DataEncryption,
    KeyEncryption,
    Signing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeEncryptedData {
    pub encrypted_dek: EncryptedData,
    pub encrypted_data: EncryptedData,
    pub kek_id: String,
}

pub struct EncryptionManager {
    config: EncryptionConfig,
    keys: Arc<RwLock<HashMap<String, EncryptionKey>>>,
    primary_dek_id: Arc<RwLock<Option<String>>>,
    primary_kek_id: Arc<RwLock<Option<String>>>,
}

static MASTER_KEY: std::sync::OnceLock<std::sync::RwLock<Option<Vec<u8>>>> = std::sync::OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_aes_gcm() {
        // #1297 — key material must never be a compile-time constant, even in
        // tests: derive it from the OS CSPRNG like production does.
        let key = EncryptionKey::generate(KeyPurpose::DataEncryption).key_data;
        let plaintext = b"Hello, World!";

        let (nonce, ciphertext) = encrypt_aes_gcm(plaintext, &key).expect("Encrypt failed");
        let decrypted = decrypt_aes_gcm(&nonce, &ciphertext, &key).expect("Decrypt failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encryption_key_generation() {
        let key = EncryptionKey::generate(KeyPurpose::DataEncryption);

        assert_eq!(key.key_data.len(), KEY_SIZE);
        assert!(!key.is_primary);
        assert!(!key.is_expired());
        assert!(key.is_valid());
    }

    #[test]
    fn test_encryption_key_primary() {
        let key = EncryptionKey::generate_primary(KeyPurpose::DataEncryption, 30);

        assert!(key.is_primary);
        assert!(key.expires_at.is_some());
        assert!(!key.is_expired());
    }

    #[test]
    fn test_encrypted_data_compact_format() {
        let data = EncryptedData {
            algorithm: "AES-256-GCM".into(),
            nonce: "abc123".into(),
            ciphertext: "xyz789".into(),
            key_id: Some("key-1".into()),
            version: 1,
        };

        let compact = data.to_string_compact();
        let parsed = EncryptedData::from_string_compact(&compact).expect("Parse failed");

        assert_eq!(parsed.version, data.version);
        assert_eq!(parsed.algorithm, data.algorithm);
        assert_eq!(parsed.nonce, data.nonce);
        assert_eq!(parsed.ciphertext, data.ciphertext);
        assert_eq!(parsed.key_id, data.key_id);
    }

    #[test]
    fn test_derive_key_from_password() {
        let password = String::from_utf8_lossy(b"test_password").to_string();
        let salt = generate_salt();

        let key1 = derive_key_from_password(&password, &salt).expect("Derive failed");
        let key2 = derive_key_from_password(&password, &salt).expect("Derive failed");

        assert_eq!(key1, key2);
        assert_eq!(key1.len(), KEY_SIZE);
    }

    #[test]
    fn test_field_encryption() {
        // #1297 — random per-run key instead of a hardcoded one.
        let key = EncryptionKey::generate(KeyPurpose::DataEncryption).key_data;
        let plaintext = "sensitive_data";

        let encrypted = encrypt_field(plaintext, &key).expect("Encrypt failed");
        let decrypted = decrypt_field(&encrypted, &key).expect("Decrypt failed");

        assert_eq!(decrypted, plaintext);
        assert!(encrypted.starts_with("1:"));
    }

    #[test]
    fn test_hash_for_search() {
        let pepper = b"secret_pepper";
        let value = "searchable_value";

        let hash1 = hash_for_search(value, pepper);
        let hash2 = hash_for_search(value, pepper);

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64);
    }

    #[tokio::test]
    async fn test_encryption_manager() {
        let config = EncryptionConfig::default();
        let manager = EncryptionManager::new(config).expect("Manager creation failed");
        manager.initialize().await.expect("Init failed");

        let plaintext = b"Test data to encrypt";
        let encrypted = manager.encrypt(plaintext).await.expect("Encrypt failed");
        let decrypted = manager.decrypt(&encrypted).await.expect("Decrypt failed");

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_envelope_encryption() {
        let config = EncryptionConfig {
            envelope_encryption: true,
            ..EncryptionConfig::default()
        };
        let manager = EncryptionManager::new(config).expect("Manager creation failed");
        manager.initialize().await.expect("Init failed");

        let plaintext = b"Sensitive data with envelope encryption";
        let envelope = manager
            .encrypt_with_envelope(plaintext)
            .await
            .expect("Envelope encrypt failed");
        let decrypted = manager
            .decrypt_with_envelope(&envelope)
            .await
            .expect("Envelope decrypt failed");

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_string_encryption() {
        let config = EncryptionConfig::default();
        let manager = EncryptionManager::new(config).expect("Manager creation failed");
        manager.initialize().await.expect("Init failed");

        let plaintext = "Hello, encrypted world!";
        let encrypted = manager
            .encrypt_string(plaintext)
            .await
            .expect("Encrypt failed");
        let decrypted = manager
            .decrypt_string(&encrypted)
            .await
            .expect("Decrypt failed");

        assert_eq!(decrypted, plaintext);
    }

    #[tokio::test]
    async fn test_key_rotation() {
        let config = EncryptionConfig::default();
        let manager = EncryptionManager::new(config).expect("Manager creation failed");
        manager.initialize().await.expect("Init failed");

        let old_dek = manager.primary_dek_id.read().await.clone();

        let (new_dek, _new_kek) = manager.rotate_keys().await.expect("Rotation failed");

        assert_ne!(old_dek, Some(new_dek));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

use part5::*;
pub use part6::*;
pub use part7::*;
