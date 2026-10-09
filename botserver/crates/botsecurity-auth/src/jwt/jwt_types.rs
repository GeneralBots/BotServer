use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::blacklist::BlacklistStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub issuer: String,
    pub audience: String,
    pub access_token_expiry_minutes: i64,
    pub refresh_token_expiry_days: i64,
    pub algorithm: JwtAlgorithm,
    pub leeway_seconds: u64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            issuer: "general-bots".into(),
            audience: "general-bots-api".into(),
            access_token_expiry_minutes: 15,
            refresh_token_expiry_days: 7,
            algorithm: JwtAlgorithm::HS256,
            leeway_seconds: 60,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
}

impl JwtAlgorithm {
    pub fn to_jsonwebtoken(&self) -> Algorithm {
        match self {
            Self::HS256 => Algorithm::HS256,
            Self::HS384 => Algorithm::HS384,
            Self::HS512 => Algorithm::HS512,
            Self::RS256 => Algorithm::RS256,
            Self::RS384 => Algorithm::RS384,
            Self::RS512 => Algorithm::RS512,
            Self::ES256 => Algorithm::ES256,
            Self::ES384 => Algorithm::ES384,
        }
    }

    pub fn is_symmetric(&self) -> bool {
        matches!(self, Self::HS256 | Self::HS384 | Self::HS512)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    Access,
    Refresh,
    IdToken,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
            Self::IdToken => "id_token",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub nbf: i64,
    pub jti: String,
    #[serde(rename = "type")]
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

impl Claims {
    pub fn new(
        user_id: Uuid,
        issuer: &str,
        audience: &str,
        token_type: TokenType,
        expiry: DateTime<Utc>,
    ) -> anyhow::Result<Self> {
        let now = Utc::now();
        Ok(Self {
            sub: user_id.to_string(),
            iss: issuer.to_string(),
            aud: audience.to_string(),
            exp: expiry.timestamp(),
            iat: now.timestamp(),
            nbf: now.timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: token_type.as_str().to_string(),
            email: None,
            username: None,
            roles: None,
            permissions: None,
            session_id: None,
            organization_id: None,
            org_id: None,
            branch_id: None,
            device_id: None,
        })
    }

    pub fn with_email(mut self, email: String) -> Self {
        self.email = Some(email);
        self
    }

    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = Some(roles);
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = Some(permissions);
        self
    }

    pub fn with_session_id(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_organization_id(mut self, org_id: String) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    pub fn with_tenant_scope(mut self, org_id: String, branch_id: String) -> Self {
        if self.organization_id.is_none() {
            self.organization_id = Some(org_id.clone());
        }
        self.org_id = Some(org_id);
        self.branch_id = Some(branch_id);
        self
    }

    pub fn branch_id(&self) -> Option<&str> {
        self.branch_id.as_deref()
    }

    pub fn tenant_org_id(&self) -> Option<&str> {
        self.org_id.as_deref()
    }

    pub fn with_device_id(mut self, device_id: String) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn user_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub).map_err(|e| anyhow!("Invalid user ID in claims: {e}"))
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    pub fn is_access_token(&self) -> bool {
        self.token_type == TokenType::Access.as_str()
    }

    pub fn is_refresh_token(&self) -> bool {
        self.token_type == TokenType::Refresh.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Debug, Clone)]
pub enum JwtKey {
    Symmetric(Vec<u8>),
    RsaPrivate(Vec<u8>),
    RsaPublic(Vec<u8>),
    EcPrivate(Vec<u8>),
    EcPublic(Vec<u8>),
}

impl JwtKey {
    pub fn from_secret(secret: &str) -> Self {
        Self::Symmetric(secret.as_bytes().to_vec())
    }

    pub fn from_rsa_private_key(key: &str) -> Self {
        Self::RsaPrivate(key.as_bytes().to_vec())
    }

    pub fn from_rsa_public_key(key: &str) -> Self {
        Self::RsaPublic(key.as_bytes().to_vec())
    }

    pub fn from_ec_private_key(key: &str) -> Self {
        Self::EcPrivate(key.as_bytes().to_vec())
    }

    pub fn from_ec_public_key(key: &str) -> Self {
        Self::EcPublic(key.as_bytes().to_vec())
    }
}

pub struct JwtManager {
    pub(crate) config: JwtConfig,
    pub(crate) encoding_key: EncodingKey,
    pub(crate) decoding_key: DecodingKey,
    pub(crate) blacklist: Arc<dyn BlacklistStore>,
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("config", &self.config)
            .finish()
    }
}
