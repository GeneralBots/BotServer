
use serde::{Serialize, Deserialize};

pub fn extract_bearer_token(auth_header: &str) -> Option<String> {
    auth_header.strip_prefix("Bearer ").map(|s| s.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwkSet {
    pub keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub use_: Option<String>,
    pub kid: Option<String>,
    pub alg: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
    pub crv: Option<String>,
}

impl JwkSet {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    pub fn add_key(&mut self, key: Jwk) {
        self.keys.push(key);
    }
}

impl Default for JwkSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIntrospectionResponse {
    pub active: bool,
    pub scope: Option<String>,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub nbf: Option<i64>,
    pub jti: Option<String>,
    pub sub: Option<String>,
    pub aud: Option<String>,
    pub iss: Option<String>,
    pub typ: Option<String>,
}

impl TokenIntrospectionResponse {
    pub fn inactive() -> Self {
        Self {
            active: false,
            scope: None,
            client_id: None,
            username: None,
            exp: None,
            iat: None,
            nbf: None,
            jti: None,
            sub: None,
            aud: None,
            iss: None,
            typ: None,
        }
    }

    pub fn from_claims(claims: &crate::jwt::jwt_types::Claims) -> Self {
        Self {
            active: true,
            scope: None,
            client_id: None,
            username: claims.username.clone(),
            exp: Some(claims.exp),
            iat: Some(claims.iat),
            nbf: Some(claims.nbf),
            jti: Some(claims.jti.clone()),
            sub: Some(claims.sub.clone()),
            aud: Some(claims.aud.clone()),
            iss: Some(claims.iss.clone()),
            typ: Some("JWT".to_string()),
        }
    }
}
