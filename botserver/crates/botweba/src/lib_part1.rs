use super::*;

impl WebaState {
    pub fn new() -> Self {
        Self {
            apps: RwLock::new(HashMap::new()),
            pages: RwLock::new(HashMap::new()),
            _components: RwLock::new(HashMap::new()),
        }
    }
}
impl Default for WebaState {
    fn default() -> Self {
        Self::new()
    }
}
