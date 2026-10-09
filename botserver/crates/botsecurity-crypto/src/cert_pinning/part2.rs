use super::*;

impl PinnedCert {
    pub fn new(hostname: &str, fingerprint: &str) -> Self {
        Self {
            hostname: hostname.to_string(),
            fingerprint: fingerprint.to_string(),
            description: None,
            is_backup: false,
            expires_at: None,
            pin_type: PinType::Leaf,
        }
    }

    pub fn backup(hostname: &str, fingerprint: &str) -> Self {
        Self {
            hostname: hostname.to_string(),
            fingerprint: fingerprint.to_string(),
            description: Some("Backup pin for certificate rotation".to_string()),
            is_backup: true,
            expires_at: None,
            pin_type: PinType::Leaf,
        }
    }

    pub fn with_type(mut self, pin_type: PinType) -> Self {
        self.pin_type = pin_type;
        self
    }

    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn with_expiration(mut self, expires: chrono::DateTime<chrono::Utc>) -> Self {
        self.expires_at = Some(expires);
        self
    }

    pub fn get_hash_bytes(&self) -> Result<Vec<u8>> {
        let hash_str = self
            .fingerprint
            .strip_prefix("sha256//")
            .ok_or_else(|| anyhow!("Invalid fingerprint format, expected 'sha256//BASE64'"))?;

        BASE64
            .decode(hash_str)
            .context("Failed to decode base64 fingerprint")
    }

    pub fn verify(&self, cert_der: &[u8]) -> Result<bool> {
        let expected_hash = self.get_hash_bytes()?;
        let actual_hash = compute_spki_fingerprint(cert_der)?;

        Ok(expected_hash == actual_hash)
    }
}
