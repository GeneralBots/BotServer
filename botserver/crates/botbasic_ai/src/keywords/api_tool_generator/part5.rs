use super::*;

pub fn register_api_tool_keywords(_state: Arc<dyn BasicRuntime>, _user: UserSession, _engine: &mut Engine) {}
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
