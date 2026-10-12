use crate::env_defaults::get_from_env;
use crate::paths::SecretPaths;
use anyhow::{anyhow, Result};

pub struct ServiceConfigResult;

impl SecretsManager {
    pub fn get_drive_config(&self) -> Result<(String, String, String)> {
        if let Ok(vault_addr) = std::env::var("VAULT_ADDR") {
            if let Ok(vault_token) = std::env::var("VAULT_TOKEN") {
                let ca_cert = std::env::var("VAULT_CACERT").unwrap_or_default();
                log::info!("Attempting to read drive config from Vault: {}", vault_addr);
                let url = format!("{}/v1/secret/data/gbo/drive", vault_addr);
                let auth = format!("X-Vault-Token: {vault_token}");

                let mut args = vec!["-sf".to_string(), "-H".to_string(), auth, url];
                if !ca_cert.is_empty() && std::path::Path::new(&ca_cert).exists() {
                    args.insert(1, "--cacert".to_string());
                    args.insert(2, ca_cert);
                } else {
                    args.insert(1, "-k".to_string());
                }

                let result = std::process::Command::new("curl").args(&args).output();

                if let Ok(output) = result {
                    if output.status.success() {
                        if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                            if let Some(secret_data) = data.get("data").and_then(|d| d.get("data")) {
                                let host = secret_data
                                    .get("host")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("drive host not configured in Vault"))?;
                                let accesskey = secret_data
                                    .get("accesskey")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("drive accesskey not configured in Vault"))?;
                                let secret = secret_data
                                    .get("secret")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("drive secret not configured in Vault"))?;
                                log::info!("get_drive_config: Successfully read from Vault - host={}", host);
                                return Ok((host.to_string(), accesskey.to_string(), secret.to_string()));
                            }
                        }
                    } else {
                        log::error!("curl failed: {}", String::from_utf8_lossy(&output.stderr));
                    }
                }
            }
        }
        Err(anyhow!("Drive configuration not available in Vault"))
    }

    pub fn get_cache_config(&self) -> Result<(String, u16, Option<String>)> {
        if let Ok(vault_addr) = std::env::var("VAULT_ADDR") {
            if let Ok(vault_token) = std::env::var("VAULT_TOKEN") {
                let ca_cert = std::env::var("VAULT_CACERT").unwrap_or_default();
                log::info!("Attempting to read cache config from Vault: {}", vault_addr);
                let url = format!("{}/v1/secret/data/gbo/cache", vault_addr);
                let auth = format!("X-Vault-Token: {vault_token}");

                let mut args = vec!["-sf".to_string(), "-H".to_string(), auth, url];
                if !ca_cert.is_empty() && std::path::Path::new(&ca_cert).exists() {
                    args.insert(1, "--cacert".to_string());
                    args.insert(2, ca_cert);
                } else {
                    args.insert(1, "-k".to_string());
                }

                let result = std::process::Command::new("curl").args(&args).output();

                if let Ok(output) = result {
                    if output.status.success() {
                        if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                            if let Some(secret_data) = data.get("data").and_then(|d| d.get("data")) {
                                let host = secret_data
                                    .get("host")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("cache host not configured in Vault"))?;
                                let port = secret_data
                                    .get("port")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("cache port not configured in Vault"))?
                                    .parse()
                                    .map_err(|e| anyhow!("Invalid cache port: {}", e))?;
                                let password = secret_data.get("password").and_then(|v| v.as_str()).map(|s| s.to_string());
                                log::info!("get_cache_config: Successfully read from Vault - host={}", host);
                                return Ok((host.to_string(), port, password));
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow!("Cache configuration not available in Vault"))
    }

    pub fn get_qdrant_config(&self) -> Result<(String, Option<String>)> {
        if let Ok(vault_addr) = std::env::var("VAULT_ADDR") {
            if let Ok(vault_token) = std::env::var("VAULT_TOKEN") {
                let ca_cert = std::env::var("VAULT_CACERT").unwrap_or_default();
                log::info!("Attempting to read qdrant config from Vault: {}", vault_addr);
                let url = format!("{}/v1/secret/data/gbo/vectordb", vault_addr);
                let auth = format!("X-Vault-Token: {vault_token}");

                let mut args = vec!["-sf".to_string(), "-H".to_string(), auth, url];
                if !ca_cert.is_empty() && std::path::Path::new(&ca_cert).exists() {
                    args.insert(1, "--cacert".to_string());
                    args.insert(2, ca_cert);
                } else {
                    args.insert(1, "-k".to_string());
                }

                let result = std::process::Command::new("curl").args(&args).output();

                if let Ok(output) = result {
                    if output.status.success() {
                        if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                            if let Some(secret_data) = data.get("data").and_then(|d| d.get("data")) {
                                let url = secret_data
                                    .get("url")
                                    .and_then(|v| v.as_str())
                                    .ok_or_else(|| anyhow!("vectordb url not configured in Vault"))?;
                                let api_key = secret_data.get("api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
                                log::info!("get_qdrant_config: Successfully read from Vault - url={}", url);
                                return Ok((url.to_string(), api_key));
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow!("VectorDB configuration not available in Vault"))
    }

    pub fn get_database_config_sync(&self) -> Result<(String, u16, String, String, String)> {
        if let Ok(secrets) = get_from_env(SecretPaths::TABLES) {
            let host = secrets
                .get("host")
                .cloned()
                .ok_or_else(|| anyhow!("database host not configured"))?;
            let port = secrets
                .get("port")
                .and_then(|p| p.parse().ok())
                .ok_or_else(|| anyhow!("database port not configured"))?;
            let database = secrets
                .get("database")
                .cloned()
                .ok_or_else(|| anyhow!("database name not configured"))?;
            let username = secrets
                .get("username")
                .cloned()
                .ok_or_else(|| anyhow!("database username not configured"))?;
            let password = secrets.get("password").cloned().unwrap_or_default();
            return Ok((host, port, database, username, password));
        }
        Err(anyhow!("Database configuration not available"))
    }

    pub async fn get_drive_credentials(&self) -> Result<(String, String)> {
        let s = self.get_secret(SecretPaths::DRIVE).await?;
        Ok((
            s.get("accesskey").cloned().unwrap_or_default(),
            s.get("secret").cloned().unwrap_or_default(),
        ))
    }

    pub async fn get_database_config(&self) -> Result<(String, u16, String, String, String)> {
        let s = self.get_secret(SecretPaths::TABLES).await?;
        let host = s
            .get("host")
            .cloned()
            .ok_or_else(|| anyhow!("database host not configured in Vault"))?;
        let port = s
            .get("port")
            .and_then(|p| p.parse().ok())
            .ok_or_else(|| anyhow!("database port not configured in Vault"))?;
        let database = s
            .get("database")
            .cloned()
            .ok_or_else(|| anyhow!("database name not configured in Vault"))?;
        let username = s
            .get("username")
            .cloned()
            .ok_or_else(|| anyhow!("database username not configured in Vault"))?;
        let password = s.get("password").cloned().unwrap_or_default();
        Ok((host, port, database, username, password))
    }

    pub async fn get_database_url(&self) -> Result<String> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            let url = url.trim().to_string();
            if !url.is_empty() {
                return Ok(url);
            }
        }
        let (host, port, db, user, pass) = self.get_database_config().await?;
        Ok(format!("postgres://{}:{}@{}:{}/{}", user, pass, host, port, db))
    }

    pub async fn get_database_credentials(&self) -> Result<(String, String)> {
        let s = self.get_secret(SecretPaths::TABLES).await?;
        Ok((
            s.get("username").cloned().unwrap_or_else(|| "gbuser".into()),
            s.get("password").cloned().unwrap_or_default(),
        ))
    }

    pub async fn get_cache_password(&self) -> Result<Option<String>> {
        Ok(self.get_secret(SecretPaths::CACHE).await?.get("password").cloned())
    }

    pub async fn get_directory_config(&self) -> Result<(String, String, String, String)> {
        let s = self.get_secret(SecretPaths::DIRECTORY).await?;
        let url = s
            .get("url")
            .cloned()
            .ok_or_else(|| anyhow!("directory url not configured in Vault"))?;
        Ok((
            url,
            s.get("project_id").cloned().unwrap_or_default(),
            s.get("client_id").cloned().unwrap_or_default(),
            s.get("client_secret").cloned().unwrap_or_default(),
        ))
    }

    pub async fn get_directory_credentials(&self) -> Result<(String, String)> {
        let s = self.get_secret(SecretPaths::DIRECTORY).await?;
        Ok((
            s.get("client_id").cloned().unwrap_or_default(),
            s.get("client_secret").cloned().unwrap_or_default(),
        ))
    }

    pub async fn get_vectordb_config(&self) -> Result<(String, Option<String>)> {
        let s = self.get_secret(SecretPaths::VECTORDB).await?;
        let url = s
            .get("url")
            .cloned()
            .ok_or_else(|| anyhow!("vectordb url not configured in Vault"))?;
        Ok((url, s.get("api_key").cloned()))
    }

    pub async fn get_observability_config(&self) -> Result<(String, String, String, String)> {
        let s = self.get_secret(SecretPaths::OBSERVABILITY).await?;
        let url = s
            .get("url")
            .cloned()
            .ok_or_else(|| anyhow!("observability url not configured in Vault"))?;
        Ok((
            url,
            s.get("org").cloned().unwrap_or_else(|| "system".into()),
            s.get("bucket").cloned().unwrap_or_else(|| "metrics".into()),
            s.get("token").cloned().unwrap_or_default(),
        ))
    }

    pub async fn get_llm_api_key(&self, provider: &str) -> Result<Option<String>> {
        let s = self.get_secret(SecretPaths::LLM).await?;
        Ok(s.get(&format!("{}_key", provider.to_lowercase())).cloned())
    }

    pub async fn get_encryption_key(&self) -> Result<String> {
        self.get_value(SecretPaths::ENCRYPTION, "master_key").await
    }
}

use crate::manager::SecretsManager;
