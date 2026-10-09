use super::*;

pub(super) fn get_manager() -> &'static Arc<RwLock<ProtectionManager>> {
    PROTECTION_MANAGER.get_or_init(|| {
        Arc::new(RwLock::new(ProtectionManager::new(
            ProtectionConfig::default(),
        )))
    })
}
