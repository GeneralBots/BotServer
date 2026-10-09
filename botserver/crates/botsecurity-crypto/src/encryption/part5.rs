use super::*;

impl EncryptionManager {
    pub fn new(config: EncryptionConfig) -> Result<Self> {
        let manager = Self {
            config,
            keys: Arc::new(RwLock::new(HashMap::new())),
            primary_dek_id: Arc::new(RwLock::new(None)),
            primary_kek_id: Arc::new(RwLock::new(None)),
        };

        Ok(manager)
    }

    pub async fn initialize(&self) -> Result<()> {
        let dek = EncryptionKey::generate_primary(
            KeyPurpose::DataEncryption,
            self.config.key_rotation_days,
        );
        let dek_id = dek.id.clone();

        let kek = EncryptionKey::generate_primary(
            KeyPurpose::KeyEncryption,
            self.config.key_rotation_days,
        );
        let kek_id = kek.id.clone();

        {
            let mut keys = self.keys.write().await;
            keys.insert(dek.id.clone(), dek);
            keys.insert(kek.id.clone(), kek);
        }

        *self.primary_dek_id.write().await = Some(dek_id.clone());
        *self.primary_kek_id.write().await = Some(kek_id.clone());

        info!("Encryption manager initialized with DEK {} and KEK {}", dek_id, kek_id);
        Ok(())
    }

    pub async fn encrypt(&self, plaintext: &[u8]) -> Result<EncryptedData> {
        if !self.config.enabled {
            return Err(anyhow!("Encryption is disabled"));
        }

        let key_id = self
            .primary_dek_id
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("No primary DEK available"))?;

        let keys = self.keys.read().await;
        let key = keys
            .get(&key_id)
            .ok_or_else(|| anyhow!("DEK not found"))?;

        if !key.is_valid() {
            return Err(anyhow!("DEK is invalid or expired"));
        }

        let encrypted = encrypt_aes_gcm(plaintext, &key.key_data)?;

        Ok(EncryptedData {
            algorithm: self.config.algorithm.as_str().to_string(),
            nonce: encrypted.0,
            ciphertext: encrypted.1,
            key_id: Some(key_id),
            version: 1,
        })
    }

    pub async fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>> {
        if !self.config.enabled {
            return Err(anyhow!("Encryption is disabled"));
        }

        let key_id = encrypted
            .key_id
            .as_ref()
            .ok_or_else(|| anyhow!("No key ID in encrypted data"))?;

        let keys = self.keys.read().await;
        let key = keys
            .get(key_id)
            .ok_or_else(|| anyhow!("DEK not found: {key_id}"))?;

        decrypt_aes_gcm(&encrypted.nonce, &encrypted.ciphertext, &key.key_data)
    }

    pub async fn encrypt_string(&self, plaintext: &str) -> Result<String> {
        let encrypted = self.encrypt(plaintext.as_bytes()).await?;
        Ok(encrypted.to_string_compact())
    }

    pub async fn decrypt_string(&self, encrypted: &str) -> Result<String> {
        let encrypted_data = EncryptedData::from_string_compact(encrypted)?;
        let decrypted = self.decrypt(&encrypted_data).await?;
        String::from_utf8(decrypted).map_err(|e| anyhow!("Invalid UTF-8: {e}"))
    }

    pub async fn encrypt_with_envelope(&self, plaintext: &[u8]) -> Result<EnvelopeEncryptedData> {
        if !self.config.envelope_encryption {
            return Err(anyhow!("Envelope encryption is disabled"));
        }

        let dek = EncryptionKey::generate(KeyPurpose::DataEncryption);
        let encrypted_data = encrypt_aes_gcm(plaintext, &dek.key_data)?;

        let kek_id = self
            .primary_kek_id
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow!("No primary KEK available"))?;

        let keys = self.keys.read().await;
        let kek = keys
            .get(&kek_id)
            .ok_or_else(|| anyhow!("KEK not found"))?;

        if !kek.is_valid() {
            return Err(anyhow!("KEK is invalid or expired"));
        }

        let encrypted_dek = encrypt_aes_gcm(&dek.key_data, &kek.key_data)?;

        Ok(EnvelopeEncryptedData {
            encrypted_dek: EncryptedData {
                algorithm: self.config.algorithm.as_str().to_string(),
                nonce: encrypted_dek.0,
                ciphertext: encrypted_dek.1,
                key_id: Some(kek_id.clone()),
                version: 1,
            },
            encrypted_data: EncryptedData {
                algorithm: self.config.algorithm.as_str().to_string(),
                nonce: encrypted_data.0,
                ciphertext: encrypted_data.1,
                key_id: None,
                version: 1,
            },
            kek_id,
        })
    }

    pub async fn decrypt_with_envelope(&self, envelope: &EnvelopeEncryptedData) -> Result<Vec<u8>> {
        if !self.config.envelope_encryption {
            return Err(anyhow!("Envelope encryption is disabled"));
        }

        let keys = self.keys.read().await;
        let kek = keys
            .get(&envelope.kek_id)
            .ok_or_else(|| anyhow!("KEK not found: {}", envelope.kek_id))?;

        let dek_bytes = decrypt_aes_gcm(
            &envelope.encrypted_dek.nonce,
            &envelope.encrypted_dek.ciphertext,
            &kek.key_data,
        )?;

        decrypt_aes_gcm(
            &envelope.encrypted_data.nonce,
            &envelope.encrypted_data.ciphertext,
            &dek_bytes,
        )
    }

    pub async fn rotate_keys(&self) -> Result<(String, String)> {
        let new_dek = EncryptionKey::generate_primary(
            KeyPurpose::DataEncryption,
            self.config.key_rotation_days,
        );
        let new_dek_id = new_dek.id.clone();

        let new_kek = EncryptionKey::generate_primary(
            KeyPurpose::KeyEncryption,
            self.config.key_rotation_days,
        );
        let new_kek_id = new_kek.id.clone();

        {
            let mut keys = self.keys.write().await;

            if let Some(old_dek_id) = self.primary_dek_id.read().await.as_ref() {
                if let Some(old_key) = keys.get_mut(old_dek_id) {
                    old_key.is_primary = false;
                }
            }

            if let Some(old_kek_id) = self.primary_kek_id.read().await.as_ref() {
                if let Some(old_key) = keys.get_mut(old_kek_id) {
                    old_key.is_primary = false;
                }
            }

            keys.insert(new_dek.id.clone(), new_dek);
            keys.insert(new_kek.id.clone(), new_kek);
        }

        *self.primary_dek_id.write().await = Some(new_dek_id.clone());
        *self.primary_kek_id.write().await = Some(new_kek_id.clone());

        info!("Rotated encryption keys: DEK={new_dek_id}, KEK={new_kek_id}");

        Ok((new_dek_id, new_kek_id))
    }

    pub async fn get_key(&self, key_id: &str) -> Option<EncryptionKey> {
        let keys = self.keys.read().await;
        keys.get(key_id).cloned()
    }

    pub async fn add_key(&self, key: EncryptionKey) {
        let mut keys = self.keys.write().await;
        keys.insert(key.id.clone(), key);
    }

    pub async fn remove_expired_keys(&self) -> usize {
        let mut keys = self.keys.write().await;
        let initial_count = keys.len();

        let primary_dek = self.primary_dek_id.read().await.clone();
        let primary_kek = self.primary_kek_id.read().await.clone();

        keys.retain(|id, key| {
            if Some(id.clone()) == primary_dek || Some(id.clone()) == primary_kek {
                return true;
            }
            !key.is_expired()
        });

        let removed = initial_count - keys.len();
        if removed > 0 {
            info!("Removed {removed} expired encryption keys");
        }
        removed
    }

    pub fn config(&self) -> &EncryptionConfig {
        &self.config
    }
}
pub(super) fn encrypt_aes_gcm(plaintext: &[u8], key: &[u8]) -> Result<(String, String)> {
    if key.len() != KEY_SIZE {
        return Err(anyhow!("Invalid key size: expected {KEY_SIZE}, got {}", key.len()));
    }

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);

    let mut rng = rand::rng();
    let nonce_bytes: [u8; NONCE_SIZE] = std::array::from_fn(|_| rng.random());
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| anyhow!("Encryption failed: {e}"))?;

    let nonce_b64 = BASE64.encode(nonce_bytes);
    let ciphertext_b64 = BASE64.encode(ciphertext);

    Ok((nonce_b64, ciphertext_b64))
}
pub(super) fn decrypt_aes_gcm(nonce_b64: &str, ciphertext_b64: &str, key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != KEY_SIZE {
        return Err(anyhow!("Invalid key size: expected {KEY_SIZE}, got {}", key.len()));
    }

    let nonce_bytes = BASE64
        .decode(nonce_b64)
        .map_err(|e| anyhow!("Invalid nonce encoding: {e}"))?;

    if nonce_bytes.len() != NONCE_SIZE {
        return Err(anyhow!("Invalid nonce size"));
    }

    let ciphertext = BASE64
        .decode(ciphertext_b64)
        .map_err(|e| anyhow!("Invalid ciphertext encoding: {e}"))?;

    let key = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| anyhow!("Decryption failed: {e}"))
}
