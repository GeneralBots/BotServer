use super::*;

impl Default for CertPinningConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pins: HashMap::new(),
            require_pins: false,
            allow_backup_pins: true,
            report_only: false,
            config_path: None,
            cache_ttl_secs: 3600,
        }
    }
}
impl CertPinningConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strict() -> Self {
        Self {
            enabled: true,
            pins: HashMap::new(),
            require_pins: true,
            allow_backup_pins: true,
            report_only: false,
            config_path: None,
            cache_ttl_secs: 3600,
        }
    }

    pub fn report_only() -> Self {
        Self {
            enabled: true,
            pins: HashMap::new(),
            require_pins: false,
            allow_backup_pins: true,
            report_only: true,
            config_path: None,
            cache_ttl_secs: 3600,
        }
    }

    pub fn add_pin(&mut self, pin: PinnedCert) {
        let hostname = pin.hostname.clone();
        self.pins.entry(hostname).or_default().push(pin);
    }

    pub fn add_pins(&mut self, hostname: &str, pins: Vec<PinnedCert>) {
        self.pins.insert(hostname.to_string(), pins);
    }

    pub fn remove_pins(&mut self, hostname: &str) {
        self.pins.remove(hostname);
    }

    pub fn get_pins(&self, hostname: &str) -> Option<&Vec<PinnedCert>> {
        self.pins.get(hostname)
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read pin config from {}", path.display()))?;

        let config: Self =
            serde_json::from_str(&content).context("Failed to parse pin configuration")?;

        info!("Loaded certificate pinning config from {}", path.display());
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize pin configuration")?;

        fs::write(path, content)
            .with_context(|| format!("Failed to write pin config to {}", path.display()))?;

        info!("Saved certificate pinning config to {}", path.display());
        Ok(())
    }
}
