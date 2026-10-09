use super::*;

impl ContainerOperations for PackageManager {
    async fn bootstrap_container(
        &self,
        container_name: &str,
        source_lxd: Option<&str>,
    ) -> Result<()> {
        info!("Bootstrapping container: {container_name}");

        // 0. CLEANUP - Remove any existing socat or proxy devices
        self.cleanup_existing(container_name).await?;

        // 1. Copy from source LXD if migrating
        if let Some(source_remote) = source_lxd {
            self.copy_container(source_remote, container_name).await?;
        }

        // 2. Ensure network is configured
        self.ensure_network(container_name).await?;

        // 3. Sync data from host to container
        self.sync_data_to_container(container_name).await?;

        // 4. Fix permissions
        self.fix_permissions(container_name).await?;

        // 5. Provision TLS certificates
        self.provision_certs_to_container(container_name).await?;

        // 6. Install and start service
        self.install_systemd_service(container_name).await?;

        // 7. Configure NAT rules on host (ONLY iptables, never socat)
        self.configure_iptables_nat(container_name).await?;

        // 8. Reload DNS if dns container
        if container_name == "dns" {
            self.reload_dns_zones().await?;
        }

        info!("Container {container_name} bootstrapped successfully");
        Ok(())
    }

    async fn cleanup_existing(&self, container: &str) -> Result<()> {
        // Remove socat processes
        let _ = SafeCommand::new("pkill")
            .and_then(|c| c.arg("-9"))
            .and_then(|c| c.arg("-f"))
            .and_then(|c| c.arg("socat"))
            .and_then(|cmd| cmd.execute());

        // Remove proxy devices from container
        let output = SafeCommand::new("incus")
            .and_then(|c| c.arg("config"))
            .and_then(|c| c.arg("device"))
            .and_then(|c| c.arg("list"))
            .and_then(|c| c.arg(container))
            .and_then(|cmd| cmd.execute())?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("proxy") || line.contains("port") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(name) = parts.first() {
                    let _ = SafeCommand::new("incus")
                        .and_then(|c| c.arg("config"))
                        .and_then(|c| c.arg("device"))
                        .and_then(|c| c.arg("remove"))
                        .and_then(|c| c.arg(container))
                        .and_then(|c| c.arg(name))
                        .and_then(|cmd| cmd.execute());
                }
            }
        }

        Ok(())
    }

    async fn copy_container(&self, source_remote: &str, name: &str) -> Result<()> {
        info!("Copying container {name} from {source_remote}");

        let remote_path = format!("{source_remote}:{name}");
        SafeCommand::new("incus")
            .and_then(|c| c.arg("copy"))
            .and_then(|c| c.arg("--instance-only"))
            .and_then(|c| c.arg(remote_path.as_str()))
            .and_then(|c| c.arg(name))
            .and_then(|cmd| cmd.execute())
            .context("Failed to copy container")?;

        SafeCommand::new("incus")
            .and_then(|c| c.arg("start"))
            .and_then(|c| c.arg(name))
            .and_then(|cmd| cmd.execute())
            .context("Failed to start container")?;

        Ok(())
    }

    async fn ensure_network(&self, container: &str) -> Result<()> {
        let output = SafeCommand::new("incus")
            .and_then(|c| c.arg("config"))
            .and_then(|c| c.arg("device"))
            .and_then(|c| c.arg("list"))
            .and_then(|c| c.arg(container))
            .and_then(|cmd| cmd.execute())?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        if !output_str.contains("eth0") {
            SafeCommand::new("incus")
                .and_then(|c| c.arg("config"))
                .and_then(|c| c.arg("device"))
                .and_then(|c| c.arg("add"))
                .and_then(|c| c.arg(container))
                .and_then(|c| c.arg("eth0"))
                .and_then(|c| c.arg("nic"))
                .and_then(|c| c.arg("name=eth0"))
                .and_then(|c| c.arg("network=PROD-GBO"))
                .and_then(|cmd| cmd.execute())?;
        }
        Ok(())
    }

    async fn sync_data_to_container(&self, container: &str) -> Result<()> {
        let source_path = format!("/opt/gbo/tenants/{}/{}/", self.tenant, container);

        if Path::new(&source_path).exists() {
            info!("Syncing data for {container}");

            SafeCommand::new("incus")
                .and_then(|c| c.arg("exec"))
                .and_then(|c| c.arg(container))
                .and_then(|c| c.arg("--"))
                .and_then(|c| c.arg("mkdir"))
                .and_then(|c| c.arg("-p"))
                .and_then(|c| c.arg("/opt/gbo"))
                .and_then(|cmd| cmd.execute())?;

            let source_path_dot = format!("{source_path}.");
            let target_path = format!("{container}:/opt/gbo/");
            SafeCommand::new("incus")
                .and_then(|c| c.arg("file"))
                .and_then(|c| c.arg("push"))
                .and_then(|c| c.arg("--recursive"))
                .and_then(|c| c.arg(source_path_dot.as_str()))
                .and_then(|c| c.arg(target_path.as_str()))
                .and_then(|cmd| cmd.execute())?;
        }
        Ok(())
    }

    async fn fix_permissions(&self, container: &str) -> Result<()> {
        let settings = self.get_container_settings(container)?;

        let chown_cmd = if let Some(group) = &settings.group {
            format!("chown -R {}:{} /opt/gbo/", settings.user, group)
        } else {
            format!("chown -R {}:{} /opt/gbo/", settings.user, settings.user)
        };

        SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg(container))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("sh"))
            .and_then(|c| c.arg("-c"))
            .and_then(|c| c.arg(&chown_cmd))
            .and_then(|cmd| cmd.execute())?;

        // Make binaries executable
        let bin_path = format!("{}/bin/*", self.base_path.display());
        SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg(container))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("chmod"))
            .and_then(|c| c.arg("+x"))
            .and_then(|c| c.arg(bin_path.as_str()))
            .and_then(|cmd| cmd.execute())?;

        Ok(())
    }

    async fn install_systemd_service(&self, container: &str) -> Result<()> {
        let settings = self.get_container_settings(container)?;

        let service_name = format!("{container}.service");
        let temp_path = format!("/tmp/{service_name}");

        std::fs::write(&temp_path, &settings.service_template)
            .context("Failed to write service template")?;

    let target_service_path = format!("{container}:/etc/systemd/system/{service_name}");
        SafeCommand::new("incus")
            .and_then(|c| c.arg("file"))
            .and_then(|c| c.arg("push"))
            .and_then(|c| c.arg(temp_path.as_str()))
            .and_then(|c| c.arg(target_service_path.as_str()))
            .and_then(|cmd| cmd.execute())?;

        let commands: Vec<Vec<&str>> = vec![
            vec!["daemon-reload"],
            vec!["enable", service_name.as_str()],
            vec!["start", service_name.as_str()],
        ];
        for cmd_args in commands {
            let mut cmd_builder: Result<SafeCommand, botlib::security::command_guard::CommandGuardError> = SafeCommand::new("incus")
                .and_then(|c| c.arg("exec"))
                .and_then(|c| c.arg(container))
                .and_then(|c| c.arg("--"))
                .and_then(|c| c.arg("systemctl"));

            for arg in cmd_args {
                cmd_builder = cmd_builder.and_then(|c| c.arg(arg));
            }
            cmd_builder?.execute()?;
        }

        std::fs::remove_file(&temp_path).ok();
        Ok(())
    }
}
