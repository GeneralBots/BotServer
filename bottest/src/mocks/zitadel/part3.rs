//! Split from `zitadel.rs` (#1504).

use super::*;

impl MockZitadel {
    pub async fn expect_invalid_client(&self) {
        self.expect_auth_error("invalid_client", "Client authentication failed")
            .await;
    }

    pub(crate) fn create_mock_id_token(&self, user: &TestUser) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let header = base64_url_encode(r#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = base64_url_encode(
            &serde_json::json!({
                "iss": self.issuer,
                "sub": user.id,
                "aud": "test-client",
                "exp": now + 3600,
                "iat": now,
                "email": user.email,
                "email_verified": true,
                "name": user.name,
            })
            .to_string(),
        );
        let signature = base64_url_encode("mock-signature");

        format!("{header}.{payload}.{signature}")
    }

    pub fn invalidate_token(&self, token: &str) {
        if let Some(info) = self.tokens.lock().unwrap().get_mut(token) {
            info.active = false;
        }
    }

    #[must_use]
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn issuer(&self) -> String {
        self.issuer.clone()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn discovery_url(&self) -> String {
        format!("{}/.well-known/openid-configuration", self.url())
    }

    pub fn verify(&self) -> Result<()> {
        let store = self.expectations.lock().unwrap();
        for exp in store.values() {
            exp.verify()?;
        }
        Ok(())
    }

    pub async fn reset(&self) {
        self.server.reset().await;
        self.users.lock().unwrap().clear();
        self.tokens.lock().unwrap().clear();
        self.expectations.lock().unwrap().clear();
        self.setup_discovery_endpoint().await;
        self.setup_jwks_endpoint().await;
    }

    pub async fn received_requests(&self) -> Vec<wiremock::Request> {
        self.server.received_requests().await.unwrap_or_default()
    }
}

pub(crate) fn base64_url_encode(input: &str) -> String {
    use std::io::Write;

    let mut buf = Vec::new();
    {
        let mut encoder = base64_encoder(&mut buf);
        encoder.write_all(input.as_bytes()).unwrap();
    }
    String::from_utf8(buf)
        .unwrap()
        .replace('+', "-")
        .replace('/', "_")
        .replace('=', "")
}

pub(crate) fn base64_encoder(output: &mut Vec<u8>) -> impl std::io::Write + '_ {
    struct Base64Writer<'a>(&'a mut Vec<u8>);

    impl std::io::Write for Base64Writer<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            const ALPHABET: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

            for chunk in buf.chunks(3) {
                let b0 = chunk[0] as usize;
                let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
                let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

                self.0.push(ALPHABET[b0 >> 2]);
                self.0.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)]);

                if chunk.len() > 1 {
                    self.0.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)]);
                } else {
                    self.0.push(b'=');
                }

                if chunk.len() > 2 {
                    self.0.push(ALPHABET[b2 & 0x3f]);
                } else {
                    self.0.push(b'=');
                }
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    Base64Writer(output)
}
