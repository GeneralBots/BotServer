use super::*;

impl ApiKeyManager {
    pub async fn cleanup_expired_keys(&self) -> Result<usize> {
        let mut keys = self.keys.write().await;
        let mut hashes = self.key_hashes.write().await;

        let expired_ids: Vec<Uuid> = keys
            .iter()
            .filter(|(_, k)| {
                if let Some(expires) = k.expires_at {
                    Utc::now() > expires
                } else {
                    false
                }
            })
            .map(|(id, _)| *id)
            .collect();

        for id in &expired_ids {
            if let Some(key) = keys.remove(id) {
                hashes.remove(&key.key_hash);
            }
        }

        if !expired_ids.is_empty() {
            info!("Cleaned up {} expired API keys", expired_ids.len());
        }

        Ok(expired_ids.len())
    }

    pub async fn cleanup_rotation_grace_periods(&self) -> Result<usize> {
        let mut keys = self.keys.write().await;
        let mut count = 0;

        for key in keys.values_mut() {
            if key.status == ApiKeyStatus::Rotating {
                if let Some(deadline) = key.rotation_deadline {
                    if Utc::now() > deadline {
                        key.status = ApiKeyStatus::Revoked;
                        count += 1;
                    }
                }
            }
        }

        if count > 0 {
            info!(
                "Expired {} API keys past rotation grace period",
                count
            );
        }

        Ok(count)
    }

    pub async fn get_expiring_keys(&self, days: i64) -> Vec<ApiKey> {
        let keys = self.keys.read().await;
        keys.values()
            .filter(|k| k.status == ApiKeyStatus::Active && k.is_expiring_soon(days))
            .cloned()
            .collect()
    }

    pub fn config(&self) -> &ApiKeyConfig {
        &self.config
    }
}
