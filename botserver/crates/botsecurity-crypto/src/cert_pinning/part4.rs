use super::*;

impl CertPinningManager {
    pub fn new(config: CertPinningConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            validation_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn default_manager() -> Self {
        Self::new(CertPinningConfig::default())
    }

    pub fn is_enabled(&self) -> bool {
        self.config
            .read()
            .map(|c| c.enabled)
            .unwrap_or(false)
    }

    pub fn add_pin(&self, pin: PinnedCert) -> Result<()> {
        let mut config = self
            .config
            .write()
            .map_err(|_| anyhow!("Failed to acquire write lock"))?;
        config.add_pin(pin);
        Ok(())
    }

    pub fn remove_pins(&self, hostname: &str) -> Result<()> {
        let mut config = self
            .config
            .write()
            .map_err(|_| anyhow!("Failed to acquire write lock"))?;
        config.remove_pins(hostname);

        let mut cache = self
            .validation_cache
            .write()
            .map_err(|_| anyhow!("Failed to acquire cache lock"))?;
        cache.remove(hostname);

        Ok(())
    }

    pub fn validate_certificate(
        &self,
        hostname: &str,
        cert_der: &[u8],
    ) -> Result<PinValidationResult> {
        let config = self
            .config
            .read()
            .map_err(|_| anyhow!("Failed to acquire read lock"))?;

        if !config.enabled {
            return Ok(PinValidationResult::success(hostname, "disabled", false));
        }

        if let Ok(cache) = self.validation_cache.read() {
            if let Some((result, timestamp)) = cache.get(hostname) {
                if timestamp.elapsed().as_secs() < config.cache_ttl_secs {
                    return Ok(result.clone());
                }
            }
        }

        let actual_hash = compute_spki_fingerprint(cert_der)?;
        let actual_fingerprint = format!("sha256//{}", BASE64.encode(&actual_hash));

        let Some(pins) = config.get_pins(hostname) else {
            if config.require_pins {
                let result = PinValidationResult::failure(
                    hostname,
                    &actual_fingerprint,
                    "No pins configured for hostname",
                );

                if config.report_only {
                    warn!(
                        "Certificate pinning violation (report-only): {} - {}",
                        hostname, "No pins configured"
                    );
                    return Ok(PinValidationResult::success(hostname, "report-only", false));
                }

                return Ok(result);
            }

            return Ok(PinValidationResult::success(
                hostname,
                "no-pins-required",
                false,
            ));
        };

        for pin in pins {
            match pin.verify(cert_der) {
                Ok(true) => {
                    let result =
                        PinValidationResult::success(hostname, &pin.fingerprint, pin.is_backup);

                    if pin.is_backup {
                        warn!(
                            "Certificate matched backup pin for {}: {}",
                            hostname,
                            pin.description.as_deref().unwrap_or("backup")
                        );
                    }

                    if let Ok(mut cache) = self.validation_cache.write() {
                        cache.insert(
                            hostname.to_string(),
                            (result.clone(), std::time::Instant::now()),
                        );
                    }

                    return Ok(result);
                }
                Ok(false) => {}
                Err(e) => {
                    debug!("Pin verification error for {}: {}", hostname, e);
                }
            }
        }

        let result = PinValidationResult::failure(
            hostname,
            &actual_fingerprint,
            &format!(
                "Certificate fingerprint {} does not match any pinned certificate",
                actual_fingerprint
            ),
        );

        if config.report_only {
            warn!(
                "Certificate pinning violation (report-only): {} - actual fingerprint: {}",
                hostname, actual_fingerprint
            );
            return Ok(PinValidationResult::success(hostname, "report-only", false));
        }

        error!(
            "Certificate pinning failure for {}: fingerprint {} not in pin set",
            hostname, actual_fingerprint
        );

        Ok(result)
    }

    pub fn create_pinned_client(&self, hostname: &str) -> Result<Client> {
        self.create_pinned_client_with_options(hostname, None, Duration::from_secs(30))
    }

    pub fn create_pinned_client_with_options(
        &self,
        hostname: &str,
        ca_cert: Option<&Certificate>,
        timeout: Duration,
    ) -> Result<Client> {
        let config = self
            .config
            .read()
            .map_err(|_| anyhow!("Failed to acquire read lock"))?;

        let mut builder = ClientBuilder::new()
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            .use_rustls_tls()
            .https_only(true)
            .tls_built_in_root_certs(true);

        if let Some(cert) = ca_cert {
            builder = builder.add_root_certificate(cert.clone());
        }

        if config.enabled && config.get_pins(hostname).is_some() {
            debug!(
                "Creating pinned client for {} with {} pins",
                hostname,
                config.get_pins(hostname).map(|p| p.len()).unwrap_or(0)
            );
        }

        builder.build().context("Failed to build HTTP client")
    }

    pub fn validate_pem_file(
        &self,
        hostname: &str,
        pem_path: &Path,
    ) -> Result<PinValidationResult> {
        let pem_data = fs::read(pem_path)
            .with_context(|| format!("Failed to read PEM file: {}", pem_path.display()))?;

        let der = pem_to_der(&pem_data)?;
        self.validate_certificate(hostname, &der)
    }

    pub fn generate_pin_from_file(hostname: &str, cert_path: &Path) -> Result<PinnedCert> {
        let cert_data = fs::read(cert_path)
            .with_context(|| format!("Failed to read certificate: {}", cert_path.display()))?;

        let der = if cert_data.starts_with(b"-----BEGIN") {
            pem_to_der(&cert_data)?
        } else {
            cert_data
        };

        let fingerprint = compute_spki_fingerprint(&der)?;
        let fingerprint_str = format!("sha256//{}", BASE64.encode(&fingerprint));

        Ok(PinnedCert::new(hostname, &fingerprint_str))
    }

    pub fn generate_pins_from_directory(
        hostname: &str,
        cert_dir: &Path,
    ) -> Result<Vec<PinnedCert>> {
        let mut pins = Vec::new();

        for entry in fs::read_dir(cert_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if matches!(ext, "crt" | "pem" | "cer" | "der") {
                    match Self::generate_pin_from_file(hostname, &path) {
                        Ok(pin) => {
                            info!("Generated pin from {:?}: {}", path, pin.fingerprint);
                            pins.push(pin);
                        }
                        Err(e) => {
                            warn!("Failed to generate pin from {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        Ok(pins)
    }

    pub fn export_pins(&self, path: &Path) -> Result<()> {
        let config = self
            .config
            .read()
            .map_err(|_| anyhow!("Failed to acquire read lock"))?;

        config.save_to_file(path)
    }

    pub fn import_pins(&self, path: &Path) -> Result<()> {
        let imported = CertPinningConfig::load_from_file(path)?;

        let mut config = self
            .config
            .write()
            .map_err(|_| anyhow!("Failed to acquire write lock"))?;

        for (hostname, pins) in imported.pins {
            config.pins.insert(hostname, pins);
        }

        if let Ok(mut cache) = self.validation_cache.write() {
            cache.clear();
        }

        Ok(())
    }
}
