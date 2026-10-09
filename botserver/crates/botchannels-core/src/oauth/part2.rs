use super::*;

impl OAuthToken {
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => Utc::now() >= expires,
            None => false,
        }
    }

    pub fn needs_refresh(&self) -> bool {
        match self.expires_at {
            Some(expires) => {
                let buffer = chrono::Duration::minutes(5);
                Utc::now() >= expires - buffer
            }
            None => false,
        }
    }
}
