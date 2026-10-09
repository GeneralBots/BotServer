use super::*;

impl CsrfManager {
    pub fn new(config: CsrfConfig, secret: &[u8]) -> Result<Self> {
        if secret.len() < 32 {
            return Err(anyhow!("CSRF secret must be at least 32 bytes"));
        }

        Ok(Self {
            config,
            secret: secret.to_vec(),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn from_secret(secret: &str) -> Result<Self> {
        Self::new(CsrfConfig::default(), secret.as_bytes())
    }

    pub async fn generate_token(&self) -> CsrfToken {
        let token = CsrfToken::new(self.config.token_expiry_minutes);

        let mut tokens = self.tokens.write().await;
        tokens.insert(token.token.clone(), token.clone());

        token
    }

    pub async fn generate_token_with_session(&self, session_id: &str) -> CsrfToken {
        let token = CsrfToken::new(self.config.token_expiry_minutes)
            .with_session(session_id.to_string());

        let mut tokens = self.tokens.write().await;
        tokens.insert(token.token.clone(), token.clone());

        token
    }

    pub async fn validate_token(&self, token_value: &str) -> CsrfValidationResult {
        if token_value.is_empty() {
            return CsrfValidationResult::Missing;
        }

        let tokens = self.tokens.read().await;

        match tokens.get(token_value) {
            Some(token) => {
                if token.is_expired() {
                    CsrfValidationResult::Expired
                } else {
                    CsrfValidationResult::Valid
                }
            }
            None => CsrfValidationResult::Invalid,
        }
    }

    pub async fn validate_token_with_session(
        &self,
        token_value: &str,
        session_id: &str,
    ) -> CsrfValidationResult {
        if token_value.is_empty() {
            return CsrfValidationResult::Missing;
        }

        let tokens = self.tokens.read().await;

        match tokens.get(token_value) {
            Some(token) => {
                if token.is_expired() {
                    return CsrfValidationResult::Expired;
                }

                if let Some(ref stored_session) = token.session_id {
                    if stored_session != session_id {
                        return CsrfValidationResult::SessionMismatch;
                    }
                }

                CsrfValidationResult::Valid
            }
            None => CsrfValidationResult::Invalid,
        }
    }

    pub fn validate_double_submit(
        &self,
        cookie_token: &str,
        header_token: &str,
    ) -> CsrfValidationResult {
        if cookie_token.is_empty() || header_token.is_empty() {
            return CsrfValidationResult::Missing;
        }

        if !constant_time_compare(cookie_token, header_token) {
            return CsrfValidationResult::Invalid;
        }

        if !self.verify_signed_token(cookie_token) {
            return CsrfValidationResult::Invalid;
        }

        CsrfValidationResult::Valid
    }

    pub fn generate_signed_token(&self) -> String {
        let token = generate_token();
        let timestamp = Utc::now().timestamp().to_string();
        let data = format!("{token}.{timestamp}");

        let signature = self.sign_data(&data);
        format!("{data}.{signature}")
    }

    pub fn verify_signed_token(&self, signed_token: &str) -> bool {
        let parts: Vec<&str> = signed_token.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        let data = format!("{}.{}", parts[0], parts[1]);
        let provided_signature = parts[2];

        let expected_signature = self.sign_data(&data);
        if !constant_time_compare(&expected_signature, provided_signature) {
            return false;
        }

        if let Ok(timestamp) = parts[1].parse::<i64>() {
            let created = DateTime::from_timestamp(timestamp, 0);
            if let Some(created_time) = created {
                let expiry = created_time + Duration::minutes(self.config.token_expiry_minutes);
                return Utc::now() <= expiry;
            }
        }

        false
    }

    pub(super) fn sign_data(&self, data: &str) -> String {
        // #1368 — HMAC accepts a key of any length; a construction failure
        // means the crypto backend is unusable. Fail closed (an empty
        // signature never validates) instead of aborting the process.
        let mut mac = match HmacSha256::new_from_slice(&self.secret) {
            Ok(mac) => mac,
            Err(e) => {
                warn!("CSRF signing unavailable: {e}");
                return String::new();
            }
        };
        mac.update(data.as_bytes());
        let result = mac.finalize();
        BASE64.encode(result.into_bytes())
    }

    pub async fn revoke_token(&self, token_value: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.remove(token_value);
    }

    pub async fn revoke_session_tokens(&self, session_id: &str) {
        let mut tokens = self.tokens.write().await;
        tokens.retain(|_, t| {
            t.session_id.as_ref().map(|s| s != session_id).unwrap_or(true)
        });
    }

    pub async fn cleanup_expired(&self) -> usize {
        let mut tokens = self.tokens.write().await;
        let initial_count = tokens.len();
        tokens.retain(|_, t| !t.is_expired());
        initial_count - tokens.len()
    }

    pub fn build_cookie(&self, token: &str) -> String {
        let max_age = self.config.token_expiry_minutes * 60;
        let secure = if self.config.cookie_secure {
            "; Secure"
        } else {
            ""
        };
        let same_site = format!("; SameSite={}", self.config.cookie_same_site.as_str());

        format!(
            "{}={}; Path=/; Max-Age={max_age}; HttpOnly{secure}{same_site}",
            self.config.cookie_name, token
        )
    }

    pub fn is_exempt_path(&self, path: &str) -> bool {
        self.config.exempt_paths.iter().any(|p| {
            if p.ends_with('*') {
                let prefix = p.trim_end_matches('*');
                path.starts_with(prefix)
            } else {
                p == path
            }
        })
    }

    pub fn is_exempt_method(&self, method: &str) -> bool {
        self.config
            .exempt_methods
            .iter()
            .any(|m| m.eq_ignore_ascii_case(method))
    }

    pub fn config(&self) -> &CsrfConfig {
        &self.config
    }
}
pub(super) fn generate_token() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..TOKEN_LENGTH).map(|_| rng.random()).collect();
    BASE64.encode(&bytes)
}
pub(super) fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}
pub fn extract_csrf_from_cookie(cookie_header: &str, cookie_name: &str) -> Option<String> {
    for cookie in cookie_header.split(';') {
        let cookie = cookie.trim();
        if let Some((name, value)) = cookie.split_once('=') {
            if name.trim() == cookie_name {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}
pub fn extract_csrf_from_form(body: &str, field_name: &str) -> Option<String> {
    for pair in body.split('&') {
        if let Some((name, value)) = pair.split_once('=') {
            if name == field_name {
                return Some(urlencoding::decode(value).ok()?.to_string());
            }
        }
    }
    None
}
