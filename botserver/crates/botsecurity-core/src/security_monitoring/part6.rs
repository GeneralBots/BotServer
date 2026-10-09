use super::*;

impl LockoutRecord {
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn remaining_time(&self) -> Duration {
        if self.is_expired() {
            Duration::zero()
        } else {
            self.expires_at - Utc::now()
        }
    }
}
