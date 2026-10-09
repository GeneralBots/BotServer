use super::*;

impl PackageManager {
    pub(super) fn fetch_vault_credentials() -> HashMap<String, String> {
        let mut credentials = HashMap::new();

        dotenvy::dotenv().ok();

        let base_path = std::env::var("BOTSERVER_STACK_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join("botserver-stack")
            });

        let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_default();
        let vault_token = std::env::var("VAULT_TOKEN").unwrap_or_default();

        if vault_token.is_empty() || vault_addr.is_empty() {
            info!("VAULT_ADDR or VAULT_TOKEN not set yet, using bootstrap defaults");
            return credentials;
        }

        let client_cert = base_path.join("conf/system/certificates/botserver/client.crt");
        let client_key = base_path.join("conf/system/certificates/botserver/client.key");
        let vault_check = SafeCommand::new("curl")
            .and_then(|c| {
                c.args(&[
                    "-sfk",
                    "--cert",
                    &client_cert.to_string_lossy(),
                    "--key",
                    &client_key.to_string_lossy(),
                    &format!("{}/v1/sys/health", vault_addr),
                ])
            })
            .and_then(|c| c.execute())
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !vault_check {
            trace!(
                "Vault not reachable at {}, skipping credential fetch",
                vault_addr
            );
            return credentials;
        }

        #[cfg(target_os = "windows")]
        let vault_bin = base_path.join("bin/vault/vault.exe");
        #[cfg(not(target_os = "windows"))]
        let vault_bin = base_path.join("bin/vault/vault");
        let ca_cert_path = std::env::var("VAULT_CACERT").unwrap_or_else(|_| {
            base_path
                .join("conf/system/certificates/ca/ca.crt")
                .to_string_lossy()
                .to_string()
        });

        let services = [
            ("drive", "secret/gbo/drive"),
            ("cache", "secret/gbo/cache"),
            ("tables", "secret/gbo/tables"),
            ("vectordb", "secret/gbo/vectordb"),
            ("directory", "secret/gbo/directory"),
            ("llm", "secret/gbo/llm"),
            ("meet", "secret/gbo/meet"),
            ("alm", "secret/gbo/alm"),
            ("encryption", "secret/gbo/encryption"),
        ];

        for (service_name, vault_path) in &services {
            let result = SafeCommand::new(vault_bin.to_str().unwrap_or("vault"))
                .and_then(|c| {
                    let mut command = c
                        .env("VAULT_ADDR", &vault_addr)?
                        .env("VAULT_TOKEN", &vault_token)?;
                    if std::path::Path::new(&ca_cert_path).is_file() {
                        command = command.env("VAULT_CACERT", &ca_cert_path)?;
                    }
                    Ok(command)
                })
                .and_then(|c| {
                    c.args(&["kv", "get", "-format=json", "-tls-skip-verify", vault_path])
                })
                .and_then(|c| c.execute());

            if let Ok(output) = result {
                if output.status.success() {
                    let json_str = String::from_utf8_lossy(&output.stdout);
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        if let Some(data) = json.get("data").and_then(|d| d.get("data")) {
                            if let Some(obj) = data.as_object() {
                                let prefix = service_name.to_uppercase();
                                for (key, value) in obj {
                                    if let Some(v) = value.as_str() {
                                        let env_key = format!("{}_{}", prefix, key.to_uppercase());
                                        credentials.insert(env_key, v.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        trace!("Fetched {} credentials from Vault", credentials.len());
        credentials
    }
}
