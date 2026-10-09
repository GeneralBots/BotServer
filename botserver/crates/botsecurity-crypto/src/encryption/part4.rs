use super::*;

impl EncryptionKey {
    pub fn generate(purpose: KeyPurpose) -> Self {
        let mut rng = rand::rng();
        let key_data: Vec<u8> = (0..KEY_SIZE).map(|_| rng.random()).collect();

        Self {
            id: Uuid::new_v4().to_string(),
            key_data,
            created_at: chrono::Utc::now(),
            expires_at: None,
            is_primary: false,
            purpose,
        }
    }

    pub fn generate_primary(purpose: KeyPurpose, expiry_days: u32) -> Self {
        let mut key = Self::generate(purpose);
        key.is_primary = true;
        key.expires_at = Some(
            chrono::Utc::now() + chrono::Duration::days(expiry_days as i64),
        );
        key
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            chrono::Utc::now() > expires
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.is_expired() && self.key_data.len() == KEY_SIZE
    }
}
