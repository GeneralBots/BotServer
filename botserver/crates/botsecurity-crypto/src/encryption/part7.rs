use super::*;

pub(super) fn master_key_lock() -> &'static std::sync::RwLock<Option<Vec<u8>>> {
    MASTER_KEY.get_or_init(|| std::sync::RwLock::new(None))
}
/// Tenta carregar a chave do Vault e armazená-la no cache global.
/// Chamado após o Vault provider estar disponível (pós-bootstrap).
pub fn reload_master_key_from_vault() {
    if let Some(provider) = botsecurity_core::get_vault_provider() {
        if let Ok(vault_key) = provider.get_secret("secret/gbo/encryption") {
            if !vault_key.is_empty() {
                let key = if let Ok(bytes) = hex::decode(&vault_key) {
                    if bytes.len() == 32 { bytes } else {
                        let mut hasher = Sha256::new();
                        hasher.update(vault_key.as_bytes());
                        hasher.finalize().to_vec()
                    }
                } else {
                    let mut hasher = Sha256::new();
                    hasher.update(vault_key.as_bytes());
                    hasher.finalize().to_vec()
                };
                if let Ok(mut guard) = master_key_lock().write() {
                    *guard = Some(key);
                    info!("Master encryption key loaded from Vault");
                }
            }
        }
    }
}
/// Carrega a chave mestra de criptografia.
/// Prioridade:
/// 1. Cache global (setado via `reload_master_key_from_vault()`)
/// 2. `MESSAGE_ENCRYPTION_KEY` (hex ou raw string, hasheada para 32 bytes)
/// 3. `JWT_SECRET` (hasheada para 32 bytes, fallback dev)
/// 4. Chave aleatória efêmera (log warning)
pub fn load_master_encryption_key() -> Vec<u8> {
    // 1. Try cached key first (set by reload_master_key_from_vault)
    if let Ok(guard) = master_key_lock().read() {
        if let Some(ref key) = *guard {
            return key.clone();
        }
    }

    // 2. Try env var
    if let Ok(key) = std::env::var("MESSAGE_ENCRYPTION_KEY") {
        if !key.is_empty() {
            if let Ok(bytes) = hex::decode(&key) {
                if bytes.len() == 32 {
                    info!("Master encryption key loaded from MESSAGE_ENCRYPTION_KEY");
                    return bytes;
                }
            }
            let mut hasher = Sha256::new();
            hasher.update(key.as_bytes());
            info!("Master encryption key derived from MESSAGE_ENCRYPTION_KEY");
            return hasher.finalize().to_vec();
        }
    }

    // 3. Fallback to JWT_SECRET
    if let Ok(jwt) = std::env::var("JWT_SECRET") {
        if !jwt.is_empty() {
            info!("Master encryption key derived from JWT_SECRET (dev fallback)");
            let mut hasher = Sha256::new();
            hasher.update(jwt.as_bytes());
            return hasher.finalize().to_vec();
        }
    }

    // 4. Ephemeral fallback (dev default — no Vault/keys configured)
    info!("No MESSAGE_ENCRYPTION_KEY or JWT_SECRET configured. Using ephemeral random key (dev mode).");
    let key: Vec<u8> = rand::random::<[u8; 32]>().to_vec();
    if let Ok(mut guard) = master_key_lock().write() {
        *guard = Some(key.clone());
    }
    key
}
