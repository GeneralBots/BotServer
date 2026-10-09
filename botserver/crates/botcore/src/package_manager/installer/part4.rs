use super::*;

impl PackageManager {
    pub(super) fn initialize_vault_local(&self) -> Result<()> {
        use std::io::Write;

        info!("Initializing Vault locally (non-LXC mode)...");

        let bin_path = self.base_path.join("bin/vault");
        let conf_path = self.base_path.join("conf");
        #[cfg(target_os = "windows")]
        let vault_bin = bin_path.join("vault.exe");
        #[cfg(not(target_os = "windows"))]
        let vault_bin = bin_path.join("vault");
        let vault_data = self.base_path.join("data/vault");

        // Check if Vault data directory exists (real indicator of initialized state)
        let vault_data_exists = vault_data.exists();

        if !vault_data_exists {
            info!("Vault data directory not found, will initialize fresh");
        } else {
            info!("Vault data directory found, checking health...");
        }

        // Wait for Vault to be ready
        info!("Waiting for Vault to start...");
        std::thread::sleep(std::time::Duration::from_secs(3));

        let vault_addr = std::env::var("VAULT_ADDR")
            .ok()
            .filter(|a| !a.is_empty())
            .ok_or_else(|| anyhow::anyhow!("VAULT_ADDR must be set in .env or environment"))?;
        let ca_cert = conf_path.join("system/certificates/ca/ca.crt");

        // Only attempt recovery if data directory exists
        if vault_data_exists {
            // Check if Vault is already initialized via health endpoint
            let health_cmd = format!(
                "curl -f -s --connect-timeout 2 -k {}/v1/sys/health",
                vault_addr
            );
            let health_output = safe_sh_command(&health_cmd);

            let already_initialized = if let Some(ref output) = health_output {
                if output.status.success() {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(
                        &String::from_utf8_lossy(&output.stdout),
                    ) {
                        json.get("initialized")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                    } else {
                        false
                    }
                } else {
                    // Health endpoint returns 503 when sealed but initialized
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.contains("\"initialized\":true")
                        || stderr.contains("\"initialized\":true")
                }
            } else {
                false
            };

            if already_initialized {
                info!("Vault already initialized (detected via health/data), skipping init");
                return self.recover_existing_vault();
            }
        }

        // Initialize Vault: wait for it to be ready, then run init
        let init_cmd = format!(
            "{} operator init -tls-skip-verify -key-shares=5 -key-threshold=3 -format=json -address={}",
            vault_bin.display(),
            vault_addr
        );

        // Wait for Vault to be ready (up to 120s, polling every 2s). Windows
        // first-launch of the ~112MB binary can exceed 30s under AV scanning.
        info!("Waiting for Vault to be ready on {}...", vault_addr);
        let vault_ready = 'wait: {
            for i in 1..=60 {
                // Use curl without -f (health returns 503 when sealed, which is fine)
                let check_cmd = format!(
                    "curl -sk --connect-timeout 2 --max-time 4 -o /dev/null -w '%{{http_code}}' {}/v1/sys/health",
                    vault_addr
                );
                if let Some(output) = safe_sh_command(&check_cmd) {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    // 501 = uninitialized (still ready for `operator init`); 503 = sealed
                    if output.status.success()
                        || stdout.contains("501")
                        || stdout.contains("503")
                        || stdout.contains("200")
                    {
                        break 'wait true;
                    }
                }
                info!("Waiting for Vault... (attempt {}/15)", i);
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            false
        };

        if !vault_ready {
            return Err(anyhow::anyhow!("Vault did not become ready after 30s"));
        }

        // Check if already initialized
        let check_json = format!("curl -sk --connect-timeout 2 --max-time 4 {}/v1/sys/health", vault_addr);
        if let Some(output) = safe_sh_command(&check_json) {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("\"initialized\":true") {
                info!("Vault already initialized, recovering existing data");
                return self.recover_existing_vault();
            }
        }

        info!("Running vault operator init...");
        let output = safe_sh_command(&init_cmd)
            .ok_or_else(|| anyhow::anyhow!("Failed to execute vault init command"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already initialized") {
                info!("Vault already initialized, recovering existing data");
                return self.recover_existing_vault();
            }
            return Err(anyhow::anyhow!("Failed to initialize Vault: {}", stderr));
        }

        let init_output = String::from_utf8_lossy(&output.stdout);
        let init_json_val: serde_json::Value =
            serde_json::from_str(&init_output).context("Failed to parse Vault init output")?;

        let root_token = init_json_val["root_token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No root_token in vault init output"))?
            .to_string();
        let unseal_keys: Vec<String> = init_json_val["unseal_keys_b64"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("No unseal_keys_b64 in vault init output"))?
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();

        // Set VAULT_TOKEN early so it's available for all subsequent operations
        std::env::set_var("VAULT_TOKEN", &root_token);

        // Save init.json
        let init_json = self.base_path.join("conf/vault/init.json");
        std::fs::create_dir_all(
            init_json.parent()
                .ok_or_else(|| anyhow::anyhow!("Invalid init.json path: no parent directory"))?
        )?;
        std::fs::write(&init_json, serde_json::to_string_pretty(&init_json_val)?)?;
        info!("Created {}", init_json.display());

        // Create .env file with Vault credentials
        let env_file = std::path::PathBuf::from(".env");
        let env_content = format!(
            r#"
# Vault Configuration (auto-generated)
VAULT_ADDR={}
VAULT_TOKEN={}
VAULT_CACERT={}
"#,
            vault_addr,
            root_token,
            ca_cert.display()
        );

        if env_file.exists() {
            let existing = std::fs::read_to_string(&env_file)?;
            if existing.contains("VAULT_ADDR=") {
                warn!(".env already contains VAULT_ADDR, not overwriting");
            } else {
                let mut file = std::fs::OpenOptions::new().append(true).open(&env_file)?;
                file.write_all(env_content.as_bytes())?;
                info!("Appended Vault config to .env");
            }
        } else {
            std::fs::write(&env_file, env_content.trim_start())?;
            info!("Created .env with Vault config");
        }

        // Create vault-unseal-keys file in botserver directory (next to .env)
        let unseal_keys_file = self.base_path.join("vault-unseal-keys");
        let keys_content: String = unseal_keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                format!("VAULT_UNSEAL_KEY_{}={key}\n", i + 1)
            })
            .collect();

        std::fs::write(&unseal_keys_file, keys_content)?;

        {
            let abs = crate::os_abstraction::get_abstraction();
            let _ = abs.set_readonly_owner(&unseal_keys_file)
                .map_err(|e| warn!("Failed to set permissions on {}: {}", unseal_keys_file.display(), e));
        }
        info!("Created {} (chmod 600)", unseal_keys_file.display());

        // Unseal Vault (need 3 keys)
        self.unseal_vault(&vault_bin, &vault_addr)?;

        info!("Vault initialized and unsealed successfully");
        info!("✓ Created .env with VAULT_ADDR, VAULT_TOKEN");
        info!("✓ Created /opt/gbo/secrets/vault-unseal-keys (chmod 600)");

        // Enable KV2 secrets engine at 'secret/' path
        info!("Enabling KV2 secrets engine at 'secret/'...");
        let enable_result = SafeCommand::new(vault_bin.to_str().unwrap_or("vault"))
            .and_then(|c| c.env("VAULT_ADDR", vault_addr.as_str()))
            .and_then(|c| c.env("VAULT_TOKEN", root_token.as_str()))
            .and_then(|c| c.env("VAULT_CACERT", ca_cert.to_str().unwrap_or("")))
            .and_then(|c| c.args(&["secrets", "enable", "-path=secret", "kv-v2"]))
            .and_then(|c| c.execute());
        match enable_result {
            Ok(output) => {
                if output.status.success() {
                    info!("KV2 secrets engine enabled at 'secret/'");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    if stderr.contains("path is already in use") {
                        info!("KV2 secrets engine already enabled");
                    } else {
                        warn!("Failed to enable KV2 secrets engine: {}", stderr);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to execute KV2 enable command: {e}");
            }
        }

        // Write default credentials to Vault for all components
        self.seed_vault_defaults(&vault_addr, &root_token, &ca_cert, &vault_bin)?;

        Ok(())
    }

    /// Check if Vault already has seeded credentials (to avoid overwriting on recovery)
    pub(super) fn vault_seeds_exist(
        &self,
        vault_addr: &str,
        root_token: &str,
        ca_cert: &std::path::Path,
        vault_bin: &std::path::Path,
    ) -> Result<bool> {
        let args = vec![
            "kv".to_string(),
            "get".to_string(),
            "-tls-skip-verify".to_string(),
            format!("-address={}", vault_addr),
            "-field=accesskey".to_string(),
            "secret/gbo/drive".to_string(),
        ];

        let result = SafeCommand::new(vault_bin.to_str().unwrap_or("vault"))
            .and_then(|c| {
                let mut cmd = c;
                for arg in &args {
                    cmd = cmd.trusted_arg(arg)?;
                }
                Ok(cmd)
            })
            .and_then(|c| {
                let mut command = c
                    .env("VAULT_ADDR", vault_addr)?
                    .env("VAULT_TOKEN", root_token)?;
                if ca_cert.is_file() {
                    command = command.env("VAULT_CACERT", ca_cert.to_str().unwrap_or(""))?;
                }
                Ok(command)
            })
            .and_then(|c| c.execute());

        match result {
            Ok(output) => {
                if output.status.success() {
                    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    Ok(!value.is_empty())
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }
}
