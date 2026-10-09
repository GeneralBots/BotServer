
// Fields default + alias legacy key names so a config file written by an
// older stack (e.g. `base_url` instead of `api_url`, `service_token` instead
// of `service_account_key`, or a partial JSON with only 5 keys) still
// deserializes. Without the aliases, resolve_directory_config() rejects the
// on-disk conf/system/directory_config.json and falls back to Vault, which
// may hold a raw container IP — then Zitadel 404s the token request because
// the Host header no longer matches the registered instance domain.

pub(crate) use anyhow::{anyhow, Result};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Instant;
pub(crate) use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZitadelConfig {
    #[serde(default)]
    pub issuer_url: String,
    #[serde(default)]
    pub issuer: String,
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default = "default_project_id")]
    pub project_id: String,
    #[serde(default, alias = "base_url")]
    pub api_url: String,
    #[serde(default, alias = "service_token")]
    pub service_account_key: Option<String>,
    /// Operator opt-in: allow plain http for the configured api_url even when
    /// the host is a public DNS name (e.g. an internal reverse proxy fronting
    /// Zitadel on a trusted network). Never enable on public internet deployments.
    #[serde(default)]
    pub allow_insecure_http: bool,
}

#[derive(Debug, Clone)]
pub struct ZitadelClient {
    config: ZitadelConfig,
    /// API base rebuilt from parsed URL components (scheme://host[:port]) so
    /// bearer tokens can never be redirected to a tampered host (SSRF guard).
    api_base: String,
    http_client: reqwest::Client,
    access_token: Arc<RwLock<Option<String>>>,
    token_expires_at: Arc<RwLock<Option<Instant>>>,
    pat_token: Option<String>,
    /// Username and password for password grant OAuth flow
    password_credentials: Option<(String, String)>,
}

    /// Rebuilds the API base from validated URL components, dropping any
    /// userinfo, path or query that could retarget the request (SSRF guard).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_accepts_https_public_host() {
        let base = ZitadelClient::sanitize_api_base("https://auth.example.com", false).unwrap();
        assert_eq!(base, "https://auth.example.com");
    }

    #[test]
    fn test_sanitize_accepts_http_private_ipv4() {
        let base = ZitadelClient::sanitize_api_base("http://10.157.134.9:9000", false).unwrap();
        assert_eq!(base, "http://10.157.134.9:9000");
    }

    #[test]
    fn test_sanitize_accepts_http_172_16_range() {
        let base = ZitadelClient::sanitize_api_base("http://172.16.0.10:8080", false).unwrap();
        assert_eq!(base, "http://172.16.0.10:8080");
    }

    #[test]
    fn test_sanitize_accepts_http_localhost() {
        let base = ZitadelClient::sanitize_api_base("http://localhost:9000", false).unwrap();
        assert_eq!(base, "http://localhost:9000");
    }

    #[test]
    fn test_sanitize_accepts_http_bracketed_v6_loopback() {
        let base = ZitadelClient::sanitize_api_base("http://[::1]:9000", false).unwrap();
        assert_eq!(base, "http://[::1]:9000");
    }

    #[test]
    fn test_sanitize_rejects_http_public_host() {
        assert!(ZitadelClient::sanitize_api_base("http://auth.example.com", false).is_err());
    }

    #[test]
    fn test_sanitize_rejects_http_public_ip() {
        assert!(ZitadelClient::sanitize_api_base("http://203.0.113.5", false).is_err());
    }

    #[test]
    fn test_sanitize_rejects_http_dns_name() {
        assert!(ZitadelClient::sanitize_api_base("http://directory.internal:9000", false).is_err());
    }

    #[test]
    fn test_sanitize_opt_in_allows_http_public_host() {
        let base = ZitadelClient::sanitize_api_base("http://login.example.com:8080", true).unwrap();
        assert_eq!(base, "http://login.example.com:8080");
    }

    #[test]
    fn test_sanitize_drops_path_userinfo_and_query() {
        let base = ZitadelClient::sanitize_api_base("https://user:pass@auth.example.com/path?q=1", false).unwrap();
        assert_eq!(base, "https://auth.example.com");
    }

    #[test]
    fn test_config_deserializes_with_and_without_flag() {
        let without: ZitadelConfig = serde_json::from_str(r#"{"api_url": "https://a.b"}"#).unwrap();
        assert!(!without.allow_insecure_http);
        let with: ZitadelConfig =
            serde_json::from_str(r#"{"base_url": "http://a.b", "allow_insecure_http": true}"#).unwrap();
        assert!(with.allow_insecure_http);
        assert_eq!(with.api_url, "http://a.b");
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
use part6::*;
