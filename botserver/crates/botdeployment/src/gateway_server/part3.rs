use super::*;

impl Default for ContainerRegistry {
    fn default() -> Self {
        Self {
            containers: RwLock::new(HashMap::new()),
        }
    }
}
