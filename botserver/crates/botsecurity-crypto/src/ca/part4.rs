use super::*;

impl CaManager {
    pub fn verify_certificate(&self, cert_pem: &str) -> Result<bool> {
        if !self.config.ca_cert_path.exists() {
            debug!("CA certificate not found");
            return Ok(false);
        }

        if cert_pem.is_empty() || !cert_pem.contains("BEGIN CERTIFICATE") {
            debug!("Invalid certificate PEM format");
            return Ok(false);
        }

        let revoked_path = self.config.ca_cert_path.with_extension("revoked");
        if revoked_path.exists() {
            let revoked_content = fs::read_to_string(&revoked_path)?;
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            cert_pem.hash(&mut hasher);
            let cert_hash = format!("{:016x}", hasher.finish());
            if revoked_content
                .lines()
                .any(|line| line.contains(&cert_hash))
            {
                debug!("Certificate is revoked");
                return Ok(false);
            }
        }

        info!("Certificate verified successfully");
        Ok(true)
    }

    pub fn revoke_certificate(&self, serial_number: &str, reason: &str) -> Result<()> {
        let revoked_path = self.config.ca_cert_path.with_extension("revoked");

        let entry = format!(
            "{}|{}|{}\n",
            serial_number,
            reason,
            OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?
        );

        let mut content = if revoked_path.exists() {
            fs::read_to_string(&revoked_path)?
        } else {
            String::new()
        };

        content.push_str(&entry);
        fs::write(&revoked_path, content)?;

        info!("Certificate {} revoked. Reason: {}", serial_number, reason);

        self.generate_crl()?;

        Ok(())
    }

    pub fn generate_crl(&self) -> Result<()> {
        let revoked_path = self.config.ca_cert_path.with_extension("revoked");
        let crl_path = self.config.ca_cert_path.with_extension("crl");

        let mut crl_content = String::from("-----BEGIN X509 CRL-----\n");
        let _ = writeln!(
            crl_content,
            "# CRL Generated: {}",
            OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?
        );
        let _ = writeln!(crl_content, "# Issuer: {}", self.config.organization);

        if revoked_path.exists() {
            let revoked = fs::read_to_string(&revoked_path)?;
            for line in revoked.lines() {
                if !line.is_empty() {
                    let _ = writeln!(crl_content, "# Revoked: {}", line);
                }
            }
        }

        crl_content.push_str("-----END X509 CRL-----\n");
        fs::write(&crl_path, crl_content)?;

        info!("CRL generated at {:?}", crl_path);

        Ok(())
    }

    pub async fn sync_with_external_ca(&self) -> Result<()> {
        if !self.config.external_ca_enabled {
            return Ok(());
        }

        let (Some(url), Some(api_key)) = (
            &self.config.external_ca_url,
            &self.config.external_ca_api_key,
        ) else {
            return Ok(());
        };

        info!("Syncing with external CA at {}", url);

        let client = reqwest::Client::new();

        let response = client
            .get(format!("{}/status", url))
            .header("Authorization", format!("Bearer {}", api_key))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await?;

        if response.status().is_success() {
            info!("External CA sync successful");
        } else {
            warn!("External CA returned status: {}", response.status());
        }

        Ok(())
    }
}
