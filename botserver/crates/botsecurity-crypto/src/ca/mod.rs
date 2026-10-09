pub(crate) use anyhow::Result;
pub(crate) use rcgen::{ BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, Issuer, KeyPair, SanType, };
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::fmt::Write;
pub(crate) use std::fs;
pub(crate) use std::path::PathBuf;
pub(crate) use time::{Duration, OffsetDateTime};
pub(crate) use tracing::{debug, info, warn};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CaConfig {
    pub ca_cert_path: PathBuf,

    pub ca_key_path: PathBuf,

    pub intermediate_cert_path: Option<PathBuf>,

    pub intermediate_key_path: Option<PathBuf>,

    pub validity_days: i64,

    pub key_size: usize,

    pub organization: String,

    pub country: String,

    pub state: String,

    pub locality: String,

    pub external_ca_enabled: bool,

    pub external_ca_url: Option<String>,

    pub external_ca_api_key: Option<String>,

    pub crl_path: Option<PathBuf>,

    pub ocsp_url: Option<String>,
}

pub struct CaManager {
    config: CaConfig,
    ca_params: Option<CertificateParams>,
    ca_key: Option<KeyPair>,
    intermediate_params: Option<CertificateParams>,
    intermediate_key: Option<KeyPair>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateRequest {
    pub common_name: String,
    pub san_names: Vec<String>,
    pub is_client: bool,
    pub validity_days: Option<i64>,
    pub key_size: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificateResponse {
    pub certificate: String,
    pub private_key: String,
    pub ca_certificate: String,
    pub expires_at: String,
    pub serial_number: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ca_config_default() {
        let config = CaConfig::default();
        assert_eq!(config.validity_days, 365);
        assert_eq!(config.key_size, 4096);
        assert!(!config.external_ca_enabled);
        assert_eq!(config.country, "BR");
        assert_eq!(config.organization, "BotServer Internal CA");
    }

    #[test]
    fn test_ca_config_paths() {
        let config = CaConfig::default();
        assert_eq!(config.ca_cert_path, PathBuf::from("certs/ca/ca.crt"));
        assert_eq!(config.ca_key_path, PathBuf::from("certs/ca/ca.key"));
        assert!(config.intermediate_cert_path.is_some());
        assert!(config.intermediate_key_path.is_some());
    }

    #[test]
    fn test_ca_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = CaConfig {
            ca_cert_path: temp_dir.path().join("ca.crt"),
            ca_key_path: temp_dir.path().join("ca.key"),
            intermediate_cert_path: Some(temp_dir.path().join("intermediate.crt")),
            intermediate_key_path: Some(temp_dir.path().join("intermediate.key")),
            crl_path: Some(temp_dir.path().join("crl.pem")),
            ..CaConfig::default()
        };

        let manager = CaManager::new(config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_certificate_request_structure() {
        let request = CertificateRequest {
            common_name: "test.example.com".to_string(),
            san_names: vec!["alt.example.com".to_string()],
            is_client: false,
            validity_days: Some(365),
            key_size: Some(2048),
        };

        assert_eq!(request.common_name, "test.example.com");
        assert_eq!(request.san_names.len(), 1);
        assert!(!request.is_client);
    }

    #[test]
    fn test_certificate_response_structure() {
        let response = CertificateResponse {
            certificate: "-----BEGIN CERTIFICATE-----".to_string(),
            private_key: "-----BEGIN PRIVATE KEY-----".to_string(),
            ca_certificate: "-----BEGIN CERTIFICATE-----".to_string(),
            expires_at: "2026-01-01T00:00:00Z".to_string(),
            serial_number: "1234567890".to_string(),
        };

        assert!(response.certificate.starts_with("-----BEGIN"));
        assert!(response.private_key.starts_with("-----BEGIN"));
        assert!(!response.serial_number.is_empty());
    }

    #[test]
    fn test_ca_config_external_ca() {
        let config = CaConfig {
            external_ca_enabled: true,
            external_ca_url: Some("https://ca.example.com".to_string()),
            external_ca_api_key: Some("secret-key".to_string()),
            ..CaConfig::default()
        };

        assert!(config.external_ca_enabled);
        assert_eq!(
            config.external_ca_url,
            Some("https://ca.example.com".to_string())
        );
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
