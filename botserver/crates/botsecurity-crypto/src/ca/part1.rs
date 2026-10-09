use super::*;

impl Default for CaConfig {
    fn default() -> Self {
        Self {
            ca_cert_path: PathBuf::from("certs/ca/ca.crt"),
            ca_key_path: PathBuf::from("certs/ca/ca.key"),
            intermediate_cert_path: Some(PathBuf::from("certs/ca/intermediate.crt")),
            intermediate_key_path: Some(PathBuf::from("certs/ca/intermediate.key")),
            validity_days: 365,
            key_size: 4096,
            organization: "BotServer Internal CA".to_string(),
            country: "BR".to_string(),
            state: "SP".to_string(),
            locality: "São Paulo".to_string(),
            external_ca_enabled: false,
            external_ca_url: None,
            external_ca_api_key: None,
            crl_path: Some(PathBuf::from("certs/ca/crl.pem")),
            ocsp_url: None,
        }
    }
}
