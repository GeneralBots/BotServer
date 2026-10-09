use super::*;

impl PackageManager {
    pub(super) fn unseal_vault(&self, vault_bin: &std::path::Path, vault_addr: &str) -> Result<()> {
        info!("Unsealing Vault...");
        let unseal_keys_file = self.base_path.join("vault-unseal-keys");
        if unseal_keys_file.exists() {
            let content = std::fs::read_to_string(&unseal_keys_file)?;
            let keys: Vec<String> = content
                .lines()
                .filter_map(|line| {
                    line.strip_prefix("VAULT_UNSEAL_KEY_")
                        .and_then(|rest| rest.split_once('='))
                        .map(|(_, key)| key.to_string())
                })
                .collect();

            for (i, key) in keys.iter().take(3).enumerate() {
                let unseal_cmd = format!(
                    "{} operator unseal -tls-skip-verify -address={} {}",
                    vault_bin.display(),
                    vault_addr,
                    key
                );
                let unseal_output = safe_sh_command(&unseal_cmd);
                if let Some(ref output) = unseal_output {
                    if !output.status.success() {
                        warn!("Unseal step {} may have failed", i + 1);
                    }
                }
            }
        }
        Ok(())
    }

    /// Ensure .env file exists with Vault credentials
    pub(super) fn ensure_env_file_exists(&self) -> Result<()> {
        let init_json = self.base_path.join("conf/vault/init.json");
        let env_file = std::path::PathBuf::from(".env");

        if !init_json.exists() {
            return Ok(()); // No init, no .env needed yet
        }

        let init_content = std::fs::read_to_string(&init_json)?;
        let init_json_val: serde_json::Value = serde_json::from_str(&init_content)?;

        let root_token = init_json_val["root_token"]
            .as_str()
            .context("No root_token in init.json")?;

        let conf_path = self.base_path.join("conf");
        let ca_cert = conf_path.join("system/certificates/ca/ca.crt");
        let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_default();

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
                return Ok(());
            }
            let mut file = std::fs::OpenOptions::new().append(true).open(&env_file)?;
            use std::io::Write;
            file.write_all(env_content.as_bytes())?;
        } else {
            std::fs::write(&env_file, env_content.trim_start())?;
        }

        info!("Created .env with Vault credentials");
        Ok(())
    }
}
