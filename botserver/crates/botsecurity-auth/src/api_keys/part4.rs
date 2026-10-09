use super::*;

impl ApiKey {
    pub fn is_valid(&self) -> bool {
        if self.status != ApiKeyStatus::Active && self.status != ApiKeyStatus::Rotating {
            return false;
        }

        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }

        if self.status == ApiKeyStatus::Rotating {
            if let Some(deadline) = self.rotation_deadline {
                if Utc::now() > deadline {
                    return false;
                }
            }
        }

        true
    }

    pub fn has_scope(&self, scope: &ApiKeyScope) -> bool {
        if self.scopes.contains(&ApiKeyScope::Admin) {
            return true;
        }

        for s in &self.scopes {
            if s.includes(scope) {
                return true;
            }
        }

        false
    }

    pub fn has_any_scope(&self, scopes: &[ApiKeyScope]) -> bool {
        scopes.iter().any(|s| self.has_scope(s))
    }

    pub fn has_all_scopes(&self, scopes: &[ApiKeyScope]) -> bool {
        scopes.iter().all(|s| self.has_scope(s))
    }

    pub fn is_ip_allowed(&self, ip: &str) -> bool {
        match &self.allowed_ips {
            Some(ips) if !ips.is_empty() => ips.iter().any(|allowed| {
                if allowed.contains('/') {
                    matches_cidr(ip, allowed)
                } else {
                    allowed == ip
                }
            }),
            _ => true,
        }
    }

    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        match &self.allowed_origins {
            Some(origins) if !origins.is_empty() => origins.iter().any(|allowed| {
                if allowed == "*" {
                    true
                } else if allowed.starts_with("*.") {
                    let suffix = allowed.strip_prefix("*").unwrap_or(allowed);
                    origin.ends_with(suffix)
                } else {
                    allowed == origin
                }
            }),
            _ => true,
        }
    }

    pub fn time_until_expiry(&self) -> Option<Duration> {
        self.expires_at.map(|e| e - Utc::now())
    }

    pub fn is_expiring_soon(&self, days: i64) -> bool {
        if let Some(remaining) = self.time_until_expiry() {
            remaining < Duration::days(days)
        } else {
            false
        }
    }
}
