//! Split from `zitadel.rs` (#1504).

use super::*;

impl MockZitadel {
    pub async fn start(port: u16) -> Result<Self> {
        let listener = std::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .context("Failed to bind MockZitadel port")?;

        let server = MockServer::builder().listener(listener).start().await;
        let issuer = format!("http://127.0.0.1:{port}");

        let mock = Self {
            server,
            port,
            expectations: new_expectation_store(),
            users: Arc::new(Mutex::new(HashMap::new())),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            issuer,
        };

        mock.setup_discovery_endpoint().await;
        mock.setup_jwks_endpoint().await;

        Ok(mock)
    }

    pub(crate) async fn setup_discovery_endpoint(&self) {
        let base_url = self.url();

        let discovery = OIDCDiscovery {
            issuer: base_url.clone(),
            authorization_endpoint: format!("{base_url}/oauth/v2/authorize"),
            token_endpoint: format!("{base_url}/oauth/v2/token"),
            userinfo_endpoint: format!("{base_url}/oidc/v1/userinfo"),
            introspection_endpoint: format!("{base_url}/oauth/v2/introspect"),
            revocation_endpoint: format!("{base_url}/oauth/v2/revoke"),
            jwks_uri: format!("{base_url}/oauth/v2/keys"),
            response_types_supported: vec![
                "code".to_string(),
                "token".to_string(),
                "id_token".to_string(),
                "code token".to_string(),
                "code id_token".to_string(),
                "token id_token".to_string(),
                "code token id_token".to_string(),
            ],
            subject_types_supported: vec!["public".to_string()],
            id_token_signing_alg_values_supported: vec!["RS256".to_string()],
            scopes_supported: vec![
                "openid".to_string(),
                "profile".to_string(),
                "email".to_string(),
                "offline_access".to_string(),
            ],
            token_endpoint_auth_methods_supported: vec![
                "client_secret_basic".to_string(),
                "client_secret_post".to_string(),
                "private_key_jwt".to_string(),
            ],
            claims_supported: vec![
                "sub".to_string(),
                "aud".to_string(),
                "exp".to_string(),
                "iat".to_string(),
                "iss".to_string(),
                "name".to_string(),
                "email".to_string(),
                "email_verified".to_string(),
                "preferred_username".to_string(),
            ],
        };

        Mock::given(method("GET"))
            .and(path("/.well-known/openid-configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&discovery))
            .mount(&self.server)
            .await;
    }

    pub(crate) async fn setup_jwks_endpoint(&self) {
        let jwks = serde_json::json!({
            "keys": [{
                "kty": "RSA",
                "use": "sig",
                "kid": "test-key-1",
                "alg": "RS256",
                "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
                "e": "AQAB"
            }]
        });

        Mock::given(method("GET"))
            .and(path("/oauth/v2/keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&jwks))
            .mount(&self.server)
            .await;
    }

    #[must_use]
    pub fn create_test_user(&self, email: &str) -> TestUser {
        let user = TestUser {
            id: Uuid::new_v4().to_string(),
            email: email.to_string(),
            name: email.split('@').next().unwrap_or("User").to_string(),
            ..Default::default()
        };

        self.users
            .lock()
            .unwrap()
            .insert(email.to_string(), user.clone());

        user
    }

    #[must_use]
    pub fn create_user(&self, user: TestUser) -> TestUser {
        self.users
            .lock()
            .unwrap()
            .insert(user.email.clone(), user.clone());
        user
    }

    pub async fn expect_login(&self, email: &str, password: &str) -> String {
        let user = self
            .users
            .lock()
            .unwrap()
            .get(email)
            .cloned()
            .unwrap_or_else(|| {
                let u = TestUser {
                    email: email.to_string(),
                    password: password.to_string(),
                    ..Default::default()
                };
                self.users
                    .lock()
                    .unwrap()
                    .insert(email.to_string(), u.clone());
                u
            });

        let access_token = format!("test_access_{}", Uuid::new_v4());
        let refresh_token = format!("test_refresh_{}", Uuid::new_v4());
        let expires_in = 3600u64;

        self.tokens
            .lock()
            .unwrap()
            .insert(access_token.clone(), TokenInfo { active: true });

        let token_response = TokenResponse {
            access_token: access_token.clone(),
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: Some(refresh_token),
            id_token: Some(self.create_mock_id_token(&user)),
            scope: "openid profile email".to_string(),
        };

        Mock::given(method("POST"))
            .and(path("/oauth/v2/token"))
            .and(body_string_contains(format!("username={email}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(&token_response))
            .mount(&self.server)
            .await;

        access_token
    }

    pub async fn mount_cloud_flow(&self, user_id: &str) {
        let session_id = format!("test_session_{}", Uuid::new_v4());
        let session_token = format!("test_session_token_{}", Uuid::new_v4());
        let svc_token = format!("test_service_{}", Uuid::new_v4());

        Mock::given(method("POST"))
            .and(path("/management/v1/users/_search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"totalResult": 0, "result": []})),
            )
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/management/v1/users/human"))
            .respond_with(
                ResponseTemplate::new(201).set_body_json(serde_json::json!({"userId": user_id})),
            )
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(r"^/v2/users/[^/]+/password$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/oauth/v2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": svc_token,
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "urn:zitadel:iam:org:project:role:me",
            })))
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v2/sessions"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "sessionId": session_id,
                "sessionToken": session_token,
            })))
            .mount(&self.server)
            .await;

        Mock::given(method("GET"))
            .and(path(format!("/v2/sessions/{}", session_id)))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "session": {
                    "factors": {
                        "user": { "id": user_id, "loginName": "test-user" }
                    }
                }
            })))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_token_refresh(&self) {
        let access_token = format!("test_access_{}", Uuid::new_v4());
        let refresh_token = format!("test_refresh_{}", Uuid::new_v4());

        let token_response = TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            refresh_token: Some(refresh_token),
            id_token: None,
            scope: "openid profile email".to_string(),
        };

        Mock::given(method("POST"))
            .and(path("/oauth/v2/token"))
            .and(body_string_contains("grant_type=refresh_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&token_response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_introspect(&self, token: &str, active: bool) {
        let response = if active {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            IntrospectionResponse {
                active: true,
                scope: Some("openid profile email".to_string()),
                client_id: Some("test-client".to_string()),
                username: Some("test@example.com".to_string()),
                token_type: Some("Bearer".to_string()),
                exp: Some(now + 3600),
                iat: Some(now),
                sub: Some(Uuid::new_v4().to_string()),
                aud: Some("test-client".to_string()),
                iss: Some(self.issuer.clone()),
            }
        } else {
            IntrospectionResponse {
                active: false,
                scope: None,
                client_id: None,
                username: None,
                token_type: None,
                exp: None,
                iat: None,
                sub: None,
                aud: None,
                iss: None,
            }
        };

        Mock::given(method("POST"))
            .and(path("/oauth/v2/introspect"))
            .and(body_string_contains(format!("token={token}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_any_introspect_active(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let response = IntrospectionResponse {
            active: true,
            scope: Some("openid profile email".to_string()),
            client_id: Some("test-client".to_string()),
            username: Some("test@example.com".to_string()),
            token_type: Some("Bearer".to_string()),
            exp: Some(now + 3600),
            iat: Some(now),
            sub: Some(Uuid::new_v4().to_string()),
            aud: Some("test-client".to_string()),
            iss: Some(self.issuer.clone()),
        };

        Mock::given(method("POST"))
            .and(path("/oauth/v2/introspect"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_userinfo(&self, token: &str, user: &TestUser) {
        let response = UserInfoResponse {
            sub: user.id.clone(),
            email: user.email.clone(),
            email_verified: true,
            name: user.name.clone(),
            preferred_username: user.email.clone(),
            roles: Some(user.roles.clone()),
        };

        Mock::given(method("GET"))
            .and(path("/oidc/v1/userinfo"))
            .and(header("authorization", format!("Bearer {token}").as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_any_userinfo(&self) {
        let response = UserInfoResponse {
            sub: Uuid::new_v4().to_string(),
            email: "test@example.com".to_string(),
            email_verified: true,
            name: "Test User".to_string(),
            preferred_username: "test@example.com".to_string(),
            roles: Some(vec!["user".to_string()]),
        };

        Mock::given(method("GET"))
            .and(path("/oidc/v1/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_revoke(&self) {
        Mock::given(method("POST"))
            .and(path("/oauth/v2/revoke"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_auth_error(&self, error: &str, description: &str) {
        let response = ErrorResponse {
            error: error.to_string(),
            error_description: description.to_string(),
        };

        Mock::given(method("POST"))
            .and(path("/oauth/v2/token"))
            .respond_with(ResponseTemplate::new(401).set_body_json(&response))
            .mount(&self.server)
            .await;
    }

    pub async fn expect_invalid_credentials(&self) {
        self.expect_auth_error("invalid_grant", "Invalid username or password")
            .await;
    }
}
