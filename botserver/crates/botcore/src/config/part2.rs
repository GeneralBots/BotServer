use super::*;

impl DriveConfig {
    pub fn is_valid(&self) -> bool {
        !self.access_key.is_empty() && !self.secret_key.is_empty()
    }
}
