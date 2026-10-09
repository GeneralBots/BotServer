use super::*;

impl SyncResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }
}
