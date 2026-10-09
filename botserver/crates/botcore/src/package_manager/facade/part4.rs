use super::*;

impl PackageManager {
    pub(super) fn initialize_vault(container_name: &str, ip: &str) -> Result<()> {
        info!("Initializing Vault...");

        std::thread::sleep(std::time::Duration::from_secs(5));

        let output = safe_lxc(&[
            "exec",
            container_name,
            "--",
            "bash",
            "-c",
            "VAULT_ADDR=http://127.0.0.1:8200 /opt/gbo/bin/vault operator init -key-shares=5 -key-threshold=3 -format=json",
        ]);

        let output = match output {
            Some(o) => o,
            None => return Err(anyhow::anyhow!("Failed to execute vault init command")),
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            if stderr.contains("already initialized") {
                info!("Vault already initialized, skipping file generation");
                return Ok(());
            }
            return Err(anyhow::anyhow!("Failed to initialize Vault: {}", stderr));
        }

        let init_output = String::from_utf8_lossy(&output.stdout);

        let init_json: serde_json::Value =
            serde_json::from_str(&init_output).context("Failed to parse Vault init output")?;

        let unseal_keys = init_json["unseal_keys_b64"]
            .as_array()
            .context("No unseal keys in output")?;
        let root_token = init_json["root_token"]
            .as_str()
            .context("No root token in output")?;

        let unseal_keys_file = PathBuf::from("/tmp/vault-unseal-keys");
        let mut unseal_content = String::new();
        for (i, key) in unseal_keys.iter().enumerate() {
            if i < 3 {
                let _ = writeln!(
                    unseal_content,
                    "VAULT_UNSEAL_KEY_{}={}",
                    i + 1,
                    key.as_str().unwrap_or("")
                );
            }
        }
        std::fs::write(&unseal_keys_file, &unseal_content)?;

        #[cfg(unix)]
        {
            let _ = botlib::os::fs::get_permissions_manager().set_readonly_owner(&unseal_keys_file);
        }

        info!("Created {}", unseal_keys_file.display());

        let env_file = PathBuf::from(".env");
        let env_content = format!(
            "\n# Vault Configuration (auto-generated)\nVAULT_ADDR=http://{}:8200\nVAULT_TOKEN={}\n",
            ip, root_token
        );

        if env_file.exists() {
            let existing = std::fs::read_to_string(&env_file)?;

            if existing.contains("VAULT_ADDR=") {
                warn!(".env already contains VAULT_ADDR, not overwriting");
            } else {
                let mut file = std::fs::OpenOptions::new().append(true).open(&env_file)?;
                use std::io::Write;
                file.write_all(env_content.as_bytes())?;
                info!("Appended Vault config to .env");
            }
        } else {
            std::fs::write(&env_file, env_content.trim_start())?;
            info!("Created .env with Vault config");
        }

        for i in 0..3 {
            if let Some(key) = unseal_keys.get(i) {
                let key_str = key.as_str().unwrap_or("");
                let unseal_cmd = format!(
                    "VAULT_ADDR=http://127.0.0.1:8200 /opt/gbo/bin/vault operator unseal {}",
                    key_str
                );
                let unseal_output =
                    safe_lxc(&["exec", container_name, "--", "bash", "-c", &unseal_cmd]);

                if let Some(output) = unseal_output {
                    if !output.status.success() {
                        warn!("Unseal step {} may have failed", i + 1);
                    }
                } else {
                    warn!("Unseal step {} command failed to execute", i + 1);
                }
            }
        }

        info!("Vault initialized and unsealed successfully");
        Ok(())
    }

    pub fn generate_env_from_vault(container_name: &str) -> Result<String> {
        let container_ip = Self::get_container_ip(container_name)?;

        info!("Generating .env from vault at {}", container_ip);

        let output = safe_lxc(&[
            "exec",
            container_name,
            "--",
            "bash",
            "-c",
            "cat /opt/gbo/data/core/raft/raft_state 2>/dev/null || echo 'not_initialized'",
        ]);

        let initialized = match output {
            Some(o) => o.status.success(),
            None => false,
        };

        if !initialized {
            return Err(anyhow::anyhow!(
                "Vault in container {} is not initialized. Please initialize it first with: \
                lxc exec {} -- /opt/gbo/bin/vault operator init -key-shares=1 -key-threshold=1",
                container_name, container_name
            ));
        }

        let unseal_output = safe_lxc(&[
            "exec",
            container_name,
            "--",
            "bash",
            "-c",
            "VAULT_ADDR=http://127.0.0.1:8200 vault status -format=json",
        ]);

        let sealed = match unseal_output {
            Some(o) if o.status.success() => {
                let status: serde_json::Value = serde_json::from_str(
                    &String::from_utf8_lossy(&o.stdout)
                ).unwrap_or_default();
                status["sealed"].as_bool().unwrap_or(true)
            }
            _ => true,
        };

        if sealed {
            return Err(anyhow::anyhow!(
                "Vault in container {} is sealed. Please unseal it first with: \
                lxc exec {} -- /opt/gbo/bin/vault operator unseal <key>",
                container_name, container_name
            ));
        }

        let token_output = safe_lxc(&[
            "exec",
            container_name,
            "--",
            "bash",
            "-c",
            "cat /root/.vault-token 2>/dev/null || echo ''",
        ]);

        let root_token = match token_output {
            Some(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            None => String::new(),
        };

        if root_token.is_empty() {
            return Err(anyhow::anyhow!(
                "Could not find root token in vault container. Please ensure vault is properly initialized."
            ));
        }

        let env_content = format!(
            "# Vault Configuration (auto-generated)\nVAULT_ADDR=http://{}:8200\nVAULT_TOKEN={}\n",
            container_ip, root_token
        );

        let env_file = PathBuf::from(".env");
        if env_file.exists() {
            let existing = std::fs::read_to_string(&env_file)?;

            if existing.contains("VAULT_ADDR=") {
                info!(".env already contains VAULT_ADDR, updating...");
                let updated: String = existing
                    .lines()
                    .filter(|line| !line.starts_with("VAULT_ADDR=") && !line.starts_with("VAULT_TOKEN="))
                    .collect::<Vec<_>>()
                    .join("\n");
                std::fs::write(&env_file, format!("{}\n{}", updated.trim(), env_content))?;
            } else {
                let mut file = std::fs::OpenOptions::new().append(true).open(&env_file)?;

                file.write_all(env_content.as_bytes())?;
            }
        } else {
            std::fs::write(&env_file, env_content.trim_start())?;
        }

        info!("Generated .env with Vault config from container {}", container_name);

        Ok(format!(
            "VAULT_ADDR=http://{}:8200\nVAULT_TOKEN={}",
            container_ip, root_token
        ))
    }
}
