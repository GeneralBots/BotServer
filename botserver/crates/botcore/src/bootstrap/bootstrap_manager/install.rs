use super::*;

impl BootstrapManager {
    pub fn system_status(&self) -> BootstrapProgress {
        BootstrapProgress::StartingComponent("System".to_string())
    }

    /// Run the bootstrap process
    pub async fn bootstrap(&mut self) -> anyhow::Result<()> {
        info!("Starting bootstrap process...");
        // Kill any existing processes
        self.kill_stack_processes().await?;

        // Install all required components
        self.install_all().await?;

        Ok(())
    }

    /// Install all required components
    pub async fn install_all(&mut self) -> anyhow::Result<()> {
        // If VAULT_ADDR is set and points to a remote server, skip local installation
        // All services are assumed to be running in separate containers
        let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_default();
        let is_remote_vault = !vault_addr.is_empty()
            && !vault_addr.contains("localhost")
            && !vault_addr.contains("127.0.0.1");

        if is_remote_vault {
            info!(
                "Remote Vault detected ({}), skipping local service installation",
                vault_addr
            );
            info!("All services are assumed to be running in separate containers");
            return Ok(());
        }

        let pm = PackageManager::new(self.install_mode.clone(), self.tenant.clone())?;

        // Install vault first (required for secrets management)
        if !pm.is_installed("vault") {
            info!("Installing Vault...");
            match pm.install("vault").await {
                Ok(Some(_)) => info!("Vault installed successfully"),
                Ok(None) => info!("Vault installation returned no result"),
                Err(e) => warn!("Failed to install Vault: {}", e),
            }
        } else {
            info!("Vault already installed");
        }

        // Install other core components (names must match 3rdparty.toml)
        let core_components = [
            "tables",
            "cache",
            "drive",
            "directory",
            "llm",
            "vector_db",
            "alm",
            "alm-ci",
        ];
        for component in core_components {
            if !pm.is_installed(component) {
                info!("Installing {}...", component);
                match pm.install(component).await {
                    Ok(Some(_)) => info!("{} installed successfully", component),
                    Ok(None) => info!("{} installation returned no result", component),
                    Err(e) => warn!("Failed to install {}: {}", component, e),
                }
            }
        }

        Ok(())
    }
}
