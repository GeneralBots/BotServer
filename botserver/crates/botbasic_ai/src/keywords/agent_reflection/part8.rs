use super::*;

pub(super) fn read_bot_config_from_vault(bot_id: &Uuid) -> Option<HashMap<String, String>> {
    use botcoresecrets::manager::SecretsManager;
    let sm = SecretsManager::get_clone().ok()?;
    if !sm.is_enabled() {
        return None;
    }
    let path = format!("gbo/{}/{}/{}", uuid::Uuid::nil(), uuid::Uuid::nil(), bot_id);
    let sm_clone = sm.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(async move { sm_clone.get_secret(&path).await.ok() })
        } else {
            None
        };
        let _ = tx.send(result);
    });
    rx.recv().ok().flatten()
}
pub(super) fn write_bot_config_to_vault(bot_id: &Uuid, key: &str, value: &str) -> Result<(), String> {
    use botcoresecrets::manager::SecretsManager;
    let sm = SecretsManager::get_clone().map_err(|e| format!("Vault not available: {}", e))?;
    if !sm.is_enabled() {
        return Err("Vault is not enabled".into());
    }
    let path = format!("gbo/{}/{}/{}", uuid::Uuid::nil(), uuid::Uuid::nil(), bot_id);
    let sm_clone = sm.clone();
    let path_owned = path;
    let key_owned = key.to_string();
    let value_owned = value.to_string();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = if let Ok(rt) = rt {
            rt.block_on(async move {
                let mut data = sm_clone.get_secret(&path_owned).await.unwrap_or_default();
                data.insert(key_owned.clone(), value_owned.clone());
                sm_clone.put_secret(&path_owned, data).await.ok()
            })
        } else {
            None
        };
        let _ = tx.send(result);
    });
    rx.recv().ok().flatten().ok_or_else(|| "Failed to write config to Vault".into())
}
