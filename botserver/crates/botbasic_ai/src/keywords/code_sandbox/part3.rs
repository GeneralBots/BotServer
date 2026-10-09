use super::*;

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            runtime: SandboxRuntime::Process,
            timeout_seconds: 30,
            memory_limit_mb: 256,
            cpu_limit_percent: 50,
            network_enabled: false,
            work_dir: "/tmp/gb-sandbox".to_string(),
            env_vars: HashMap::new(),
            allowed_paths: vec!["/data".to_string(), "/tmp".to_string()],
            python_packages: vec![],
        }
    }
}
impl SandboxConfig {
    pub fn from_bot_config(_state: &dyn BasicRuntime, bot_id: Uuid) -> Self {
        let mut config = Self::default();

        if let Some(vault_configs) = read_bot_config_from_vault(&bot_id) {
            for (config_key, config_value) in vault_configs {
                match config_key.as_str() {
                    "sandbox-enabled" => {
                        config.enabled = config_value.to_lowercase() == "true";
                    }
                    "sandbox-runtime" => {
                        config.runtime = SandboxRuntime::from(config_value.as_str());
                    }
                    "sandbox-timeout" => {
                        config.timeout_seconds = config_value.parse().unwrap_or(30);
                    }
                    "sandbox-memory-mb" | "sandbox-memory-limit" => {
                        config.memory_limit_mb = config_value.parse().unwrap_or(256);
                    }
                    "sandbox-cpu-percent" | "sandbox-cpu-limit" => {
                        config.cpu_limit_percent = config_value.parse().unwrap_or(50);
                    }
                    "sandbox-network" | "sandbox-network-enabled" => {
                        config.network_enabled = config_value.to_lowercase() == "true";
                    }
                    "sandbox-python-packages" => {
                        config.python_packages = config_value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "sandbox-allowed-paths" => {
                        config.allowed_paths = config_value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    _ => {}
                }
            }
        }

        config
    }
}
