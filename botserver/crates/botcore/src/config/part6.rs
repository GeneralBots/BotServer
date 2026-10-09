use super::*;

impl ConfigManager {
    pub fn get_bot_config_value(
        &self,
        bot_id: &uuid::Uuid,
        key: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(value) = Self::central_llm_value(key) {
            return Ok(value);
        }
        if is_sensitive_key(key) {
            let (org_id, branch_id) = self.resolve_bot_identity(bot_id);
            let nil = uuid::Uuid::nil();
            let paths = [
                Self::vault_path(&org_id, &branch_id, bot_id),
                Self::vault_path(&nil, &nil, bot_id),
            ];
            for path in &paths {
                if let Some(value) = Self::read_vault_value(path, key) {
                    if !value.starts_with("1:") {
                        return Ok(value);
                    }
                    let master_key = load_master_encryption_key();
                    let key_bytes = derive_scope_key(&master_key, "config", bot_id);
                    if let Ok(decrypted) = decrypt_field(&value, &key_bytes) {
                        return Ok(decrypted);
                    }
                    log::warn!("Failed to decrypt legacy value for '{}' in Vault for bot {}", key, bot_id);
                }
            }
        }
        self.read_db_value(bot_id, key)
            .ok_or_else(|| "Config key not found".into())
    }

    /// Return all config (Vault sensitive + DB non-sensitive) for a bot.
    pub fn get_all_config(
        &self,
        bot_id: &uuid::Uuid,
    ) -> std::collections::HashMap<String, String> {
        let mut map = self.read_db_all(bot_id);
        let master_key = load_master_encryption_key();

        let nil = uuid::Uuid::nil();
        let (org_id, branch_id) = self.resolve_bot_identity(bot_id);
        let paths = [
            Self::vault_path(&org_id, &branch_id, bot_id),
            Self::vault_path(&nil, &nil, bot_id),
        ];
        for path in &paths {
            if let Some(vault_data) = Self::read_vault_all(path) {
                for (k, v) in vault_data {
                    if is_sensitive_key(&k) && !map.contains_key(&k) {
                        if v.starts_with("1:") {
                            let key_bytes = derive_scope_key(&master_key, "config", bot_id);
                            if let Ok(decrypted) = decrypt_field(&v, &key_bytes) {
                                map.insert(k, decrypted);
                            }
                        } else {
                            map.insert(k, v);
                        }
                    }
                }
            }
        }

        map
    }

    pub fn set_config(
        &self,
        bot_id: &uuid::Uuid,
        key: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.set_config_with_branch(bot_id, key, value, None)
    }

    /// Write a config value.
    /// Sensitive keys → Vault. Non-sensitive keys → bot_configuration table.
    pub fn set_config_with_branch(
        &self,
        bot_id: &uuid::Uuid,
        key: &str,
        value: &str,
        branch_id: Option<uuid::Uuid>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if is_sensitive_key(key) {
            let (org_id, branch_id) = self.resolve_bot_identity(bot_id);
            let path = Self::vault_path(&org_id, &branch_id, bot_id);
            Self::write_vault_value(&path, key, value)
                .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)
        } else {
            self.write_db_value(bot_id, key, value, branch_id)
        }
    }
}
