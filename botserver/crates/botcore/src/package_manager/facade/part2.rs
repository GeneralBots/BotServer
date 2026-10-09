use super::*;

impl PackageManager {
    pub async fn install(&self, component_name: &str) -> Result<Option<InstallResult>> {
        let component = self
            .components
            .get(component_name)
            .context(format!("Component '{}' not found", component_name))?;
        trace!(
            "Starting installation of component '{}' in {:?} mode",
            component_name,
            self.mode
        );
        for dep in &component.dependencies {
            if !self.is_installed(dep) {
                warn!("Installing missing dependency: {}", dep);
                Box::pin(self.install(dep)).await?;
            }
        }
        let result = match self.mode {
            InstallMode::Local => {
                self.install_local(component).await?;
                None
            }
            InstallMode::Container => Some(self.install_container(component)?),
        };
        trace!(
            "Component '{}' installation completed successfully",
            component_name
        );
        Ok(result)
    }
    pub async fn install_local(&self, component: &ComponentConfig) -> Result<()> {
        trace!(
            "Installing component '{}' locally to {}",
            component.name,
            self.base_path.display()
        );
        self.create_directories(&component.name)?;
        let (pre_cmds, post_cmds) = match self.os_type {
            OsType::Linux => (
                &component.pre_install_cmds_linux,
                &component.post_install_cmds_linux,
            ),
            OsType::MacOS => (
                &component.pre_install_cmds_macos,
                &component.post_install_cmds_macos,
            ),
            OsType::Windows => (
                &component.pre_install_cmds_windows,
                &component.post_install_cmds_windows,
            ),
        };
        self.run_commands(pre_cmds, "local", &component.name)?;
        self.install_system_packages(component)?;
        if let Some(url) = &component.download_url {
            let url = url.clone();
            let name = component.name.clone();
            let binary_name = component.effective_binary_name();
            self.download_and_install(&url, &name, binary_name.as_deref())
                .await?;
        }
        if !component.data_download_list.is_empty() {
            let cache_base = self.base_path.parent().unwrap_or(&self.base_path);
            let cache = DownloadCache::new(cache_base).ok();

            for url in &component.data_download_list {
                let filename = DownloadCache::extract_filename(url);
                let output_path = self
                    .base_path
                    .join("data")
                    .join(&component.name)
                    .join(&filename);

                if output_path.exists() {
                    info!("Data file already exists: {}", output_path.display());
                    continue;
                }

                if let Some(parent) = output_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                if let Some(ref c) = cache {
                    if let Some(cached_path) = c.get_cached_path(&filename) {
                        info!("Using cached data file: {}", cached_path.display());
                        std::fs::copy(&cached_path, &output_path)?;
                        continue;
                    }
                }

                let download_target = if let Some(ref c) = cache {
                    c.get_cache_path(&filename)
                } else {
                    output_path.clone()
                };

                info!("Downloading data file: {}", url);
                println!("Downloading {}", url);
                utils::download_file(url, download_target.to_str().unwrap_or_default()).await?;

                if cache.is_some() && download_target != output_path {
                    std::fs::copy(&download_target, &output_path)?;
                    info!("Copied cached file to: {}", output_path.display());
                }
            }
        }
        self.run_commands(post_cmds, "local", &component.name)?;

        if component.name == "tables" && self.os_type == OsType::Windows {
            use diesel::pg::PgConnection;
            use diesel::connection::SimpleConnection;
            use diesel::Connection;
            use std::time::Duration;

            let pg_url = "postgres://gbuser@127.0.0.1:5432/postgres";
            for attempt in 0..60 {
                if attempt > 0 {
                    std::thread::sleep(Duration::from_secs(2));
                }
                match PgConnection::establish(pg_url) {
                    Ok(mut conn) => {
                        info!("Connected to PostgreSQL, creating database botserver...");
                        let result = conn.batch_execute("CREATE DATABASE botserver WITH OWNER gbuser");
                        match result {
                            Ok(_) => info!("Database botserver created successfully"),
                            Err(e) => {
                                let err_str = e.to_string();
                                if err_str.contains("already exists") {
                                    info!("Database botserver already exists");
                                } else {
                                    warn!("Failed to create database botserver: {}", e);
                                }
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        if attempt == 0 {
                            info!("Waiting for PostgreSQL to be ready...");
                        }
                        if attempt >= 59 {
                            warn!("Failed to connect to PostgreSQL after 60 attempts: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }
    pub fn install_container_only(&self, component_name: &str) -> Result<InstallResult> {
        let container_name = format!("{}-{}", self.tenant, component_name);

        let _ = safe_lxd(&["init", "--auto"]);

        let images = [
            "ubuntu:24.04",
            "ubuntu:22.04",
            "images:debian/12",
            "images:debian/11",
        ];

        let mut last_error = String::new();
        let mut success = false;

        for image in &images {
            info!("Attempting to create container with image: {}", image);
            let output = safe_lxc(&[
                "launch",
                image,
                &container_name,
                "-c",
                "security.privileged=true",
            ]);

            let output = match output {
                Some(o) => o,
                None => continue,
            };

            if output.status.success() {
                info!("Successfully created container with image: {}", image);
                success = true;
                break;
            }
            last_error = String::from_utf8_lossy(&output.stderr).to_string();
            warn!("Failed to create container with {}: {}", image, last_error);

            let _ = safe_lxc(&["delete", &container_name, "--force"]);
        }

        if !success {
            return Err(anyhow::anyhow!(
                "LXC container creation failed with all images. Last error: {}",
                last_error
            ));
        }

        std::thread::sleep(std::time::Duration::from_secs(15));

        let container_ip = Self::get_container_ip(&container_name)?;

        info!("Container '{}' created successfully at IP: {}", container_name, container_ip);

        Ok(InstallResult {
            component: component_name.to_string(),
            container_name: container_name.clone(),
            container_ip: container_ip.clone(),
            ports: vec![],
            env_vars: std::collections::HashMap::new(),
            connection_info: format!(
                "Container '{}' created successfully at IP: {}\nRun without --container-only to complete installation.",
                container_name, container_ip
            ),
        })
    }
}
