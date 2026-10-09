use super::*;

impl ConfigManager {
    pub fn new(pool: DbPool) -> Self {
        Self { pool: Arc::new(pool) }
    }

    pub(super) fn resolve_bot_identity(&self, bot_id: &uuid::Uuid) -> (uuid::Uuid, uuid::Uuid) {
        if bot_id == &uuid::Uuid::nil() {
            return (uuid::Uuid::nil(), uuid::Uuid::nil());
        }
        if let Ok(mut conn) = self.pool.get() {
            if let Ok(row) = diesel::sql_query(
                "SELECT org_id, branch_id FROM bots WHERE id = $1"
            )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .get_result::<BotIdentityRow>(&mut conn)
            {
                return (row.org_id, row.branch_id);
            }
        }
        (uuid::Uuid::nil(), uuid::Uuid::nil())
    }

    pub(super) fn vault_path(org_id: &uuid::Uuid, branch_id: &uuid::Uuid, bot_id: &uuid::Uuid) -> String {
        format!("gbo/{}/{}/{}", org_id, branch_id, bot_id)
    }

    /// Maps bot-config `llm-*` keys to fields of the central `gbo/llm`
    /// Vault secret — the SINGLE point of update for LLM provider settings
    /// (url/model/key/provider). Every bot and the Vibe agent loop inherit
    /// from it; per-bot Vault entries and the DB only fill fields the
    /// central secret does not define, so rotating a provider key happens
    /// in exactly one place.
    pub(super) fn central_llm_field(key: &str) -> Option<&'static str> {
        match key {
            "llm-url" => Some("url"),
            "llm-key" => Some("openai_key"),
            "llm-model" => Some("model"),
            "llm-provider" => Some("provider"),
            "embedding-key" => Some("embedding_key"),
            _ => None,
        }
    }

    pub(super) fn central_llm_value(key: &str) -> Option<String> {
        let field = Self::central_llm_field(key)?;
        let value = Self::read_vault_value("gbo/llm", field)?;
        if value.is_empty() {
            None
        } else {
            Some(value)
        }
    }

    // ── Vault helpers (for sensitive keys) ──────────────────────────

    pub(super) fn read_vault_value(path: &str, key: &str) -> Option<String> {
        let sm = SecretsManager::get_clone().ok()?;
        if !sm.is_enabled() { return None; }
        let sm_clone = sm.clone();
        let path_owned = path.to_string();
        let key_owned = key.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
            let result = if let Ok(rt) = rt {
                rt.block_on(async move { sm_clone.get_value(&path_owned, &key_owned).await.ok() })
            } else { None };
            let _ = tx.send(result);
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).ok().flatten()
    }

    pub(super) fn read_vault_all(path: &str) -> Option<std::collections::HashMap<String, String>> {
        let sm = SecretsManager::get_clone().ok()?;
        if !sm.is_enabled() { return None; }
        let sm_clone = sm.clone();
        let path_owned = path.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
            let result = if let Ok(rt) = rt {
                rt.block_on(async move { sm_clone.get_secret(&path_owned).await.ok() })
            } else { None };
            let _ = tx.send(result);
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).ok().flatten()
    }

    pub(super) fn write_vault_value(path: &str, key: &str, value: &str) -> Result<(), String> {
        let sm = SecretsManager::get_clone().map_err(|e| format!("Vault not available: {}", e))?;
        if !sm.is_enabled() { return Err("Vault is not enabled".into()); }
        let sm_clone = sm.clone();
        let path_owned = path.to_string();
        let key_owned = key.to_string();
        let value_owned = value.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
            let result = if let Ok(rt) = rt {
                rt.block_on(async move {
                    let mut data = sm_clone.get_secret(&path_owned).await.unwrap_or_default();
                    data.insert(key_owned.clone(), value_owned.clone());
                    sm_clone.put_secret(&path_owned, data).await.ok()
                })
            } else { None };
            let _ = tx.send(result);
        });
        rx.recv_timeout(std::time::Duration::from_secs(5)).ok().flatten().ok_or_else(|| "Failed to write config to Vault".into())
    }

    // ── DB helpers (for non-sensitive keys) ─────────────────────────

    pub(super) fn read_db_value(&self, bot_id: &uuid::Uuid, key: &str) -> Option<String> {
        if let Ok(mut conn) = self.pool.get() {
            // Prefer the bot's real branch row. drive_monitor syncs config.csv
            // under the workspace branch (which resolves to nil for slug
            // "default"), so an unfiltered LIMIT 1 can return a stale
            // nil-branch duplicate and shadow the bot's own config.
            let (_, branch_id) = self.resolve_bot_identity(bot_id);
            let row: Option<ConfigRow> = diesel::sql_query(
                "SELECT config_value FROM bot_configuration \
                 WHERE bot_id = $1 AND config_key = $2 AND branch_id = $3 \
                 ORDER BY updated_at DESC LIMIT 1"
            )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Text, _>(key)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .get_result(&mut conn).ok();
            if let Some(r) = row {
                if !is_placeholder_value(&r.config_value) {
                    return Some(r.config_value);
                }
            }
            // fallback: any branch for this bot
            let row: Option<ConfigRow> = diesel::sql_query(
                "SELECT config_value FROM bot_configuration \
                 WHERE bot_id = $1 AND config_key = $2 LIMIT 1"
            )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Text, _>(key)
            .get_result(&mut conn).ok();
            if let Some(r) = row {
                if !is_placeholder_value(&r.config_value) {
                    return Some(r.config_value);
                }
            }
            // fallback: nil bot default
            let row: Option<ConfigRow> = diesel::sql_query(
                "SELECT config_value FROM bot_configuration \
                 WHERE bot_id = $1 AND config_key = $2 LIMIT 1"
            )
            .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::nil())
            .bind::<diesel::sql_types::Text, _>(key)
            .get_result(&mut conn).ok();
            if let Some(r) = row {
                if !is_placeholder_value(&r.config_value) {
                    return Some(r.config_value);
                }
            }
        }
        None
    }

    pub(super) fn read_db_all(&self, bot_id: &uuid::Uuid) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        if let Ok(mut conn) = self.pool.get() {
            if let Ok(rows) = diesel::sql_query(
                "SELECT config_key, config_value FROM bot_configuration \
                 WHERE bot_id = $1"
            )
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .load::<ConfigRowPair>(&mut conn)
            {
                for row in rows {
                    if !is_placeholder_value(&row.config_value) {
                        map.insert(row.config_key, row.config_value);
                    }
                }
            }
            // also get nil-bot defaults (not overwriting bot-specific)
            if let Ok(rows) = diesel::sql_query(
                "SELECT config_key, config_value FROM bot_configuration \
                 WHERE bot_id = $1"
            )
            .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::nil())
            .load::<ConfigRowPair>(&mut conn)
            {
                for row in rows {
                    if !is_placeholder_value(&row.config_value) && !map.contains_key(&row.config_key) {
                        map.insert(row.config_key, row.config_value);
                    }
                }
            }
        }
        map
    }

    pub(super) fn write_db_value(
        &self,
        bot_id: &uuid::Uuid,
        key: &str,
        value: &str,
        branch_id: Option<uuid::Uuid>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut conn) = self.pool.get() {
            let branch = branch_id.unwrap_or_else(uuid::Uuid::nil);
            diesel::sql_query(
                "INSERT INTO bot_configuration (id, bot_id, config_key, config_value, created_at, updated_at, branch_id) \
                 VALUES ($1, $2, $3, $4, NOW(), NOW(), $5) \
                 ON CONFLICT (branch_id, bot_id, config_key) DO UPDATE SET config_value = $4, updated_at = NOW()"
            )
            .bind::<diesel::sql_types::Uuid, _>(uuid::Uuid::new_v4())
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .bind::<diesel::sql_types::Text, _>(key)
            .bind::<diesel::sql_types::Text, _>(value)
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .execute(&mut conn)?;
        }
        Ok(())
    }

    // ── public API ────────────────────────────────────────────────

    /// Read a config value.
    /// Sensitive keys → Vault, fallback DB, fallback env, fallback default.
    /// Non-sensitive keys → DB, fallback env, fallback default.
    pub fn get_config(
        &self,
        bot_id: &uuid::Uuid,
        key: &str,
        default: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Central LLM secret first: `gbo/llm` is the tenant-wide source of
        // truth for llm-url/llm-key/llm-model/llm-provider. Per-bot Vault
        // paths and the DB are only fallbacks for undefined fields.
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
            // fallback: try DB (legacy data)
            if let Some(value) = self.read_db_value(bot_id, key) {
                return Ok(value);
            }
        } else {
            if let Some(value) = self.read_db_value(bot_id, key) {
                return Ok(value);
            }
        }

        let env_key = key.to_uppercase().replace('-', "_");
        if let Ok(val) = std::env::var(&env_key) {
            if !val.is_empty() {
                return Ok(val);
            }
        }

        Ok(default.unwrap_or("").to_string())
    }
}
