use super::*;

impl ContainerOperations for PackageManager {
    async fn configure_iptables_nat(&self, container: &str) -> Result<()> {
        let settings = self.get_container_settings(container)?;

        // Set route_localnet if not already set
        let _ = SafeCommand::new("sudo")
            .and_then(|c| c.arg("sysctl"))
            .and_then(|c| c.arg("-w"))
            .and_then(|c| c.arg("net.ipv4.conf.all.route_localnet=1"))
            .and_then(|cmd| cmd.execute());

        for rule in &settings.nat_rules {
            // Pre-allocate strings to satisfy lifetime requirements
            let port_str = rule.port.to_string();
            let dest = format!("{}:{}", settings.ip, rule.port);
            let dest_ref = dest.as_str();
            let port_ref = port_str.as_str();
            let protocol_ref = rule.protocol.as_str();
            let ip_ref = settings.ip.as_str();

            // PREROUTING rule - for external traffic
            SafeCommand::new("sudo")
                .and_then(|c| c.arg("iptables"))
                .and_then(|c| c.arg("-t"))
                .and_then(|c| c.arg("nat"))
                .and_then(|c| c.arg("-A"))
                .and_then(|c| c.arg("PREROUTING"))
                .and_then(|c| c.arg("-p"))
                .and_then(|c| c.arg(protocol_ref))
                .and_then(|c| c.arg("--dport"))
                .and_then(|c| c.arg(port_ref))
                .and_then(|c| c.arg("-j"))
                .and_then(|c| c.arg("DNAT"))
                .and_then(|c| c.arg("--to-destination"))
                .and_then(|c| c.arg(dest_ref))
                .and_then(|cmd| cmd.execute())?;

            // OUTPUT rule - for local traffic
            SafeCommand::new("sudo")
                .and_then(|c| c.arg("iptables"))
                .and_then(|c| c.arg("-t"))
                .and_then(|c| c.arg("nat"))
                .and_then(|c| c.arg("-A"))
                .and_then(|c| c.arg("OUTPUT"))
                .and_then(|c| c.arg("-p"))
                .and_then(|c| c.arg(protocol_ref))
                .and_then(|c| c.arg("--dport"))
                .and_then(|c| c.arg(port_ref))
                .and_then(|c| c.arg("-j"))
                .and_then(|c| c.arg("DNAT"))
                .and_then(|c| c.arg("--to-destination"))
                .and_then(|c| c.arg(dest_ref))
                .and_then(|cmd| cmd.execute())?;

            // FORWARD rules
            SafeCommand::new("sudo")
                .and_then(|c| c.arg("iptables"))
                .and_then(|c| c.arg("-A"))
                .and_then(|c| c.arg("FORWARD"))
                .and_then(|c| c.arg("-p"))
                .and_then(|c| c.arg(protocol_ref))
                .and_then(|c| c.arg("-d"))
                .and_then(|c| c.arg(ip_ref))
                .and_then(|c| c.arg("--dport"))
                .and_then(|c| c.arg(port_ref))
                .and_then(|c| c.arg("-j"))
                .and_then(|c| c.arg("ACCEPT"))
                .and_then(|cmd| cmd.execute())?;
        }

        let settings_ip_ref = settings.ip.as_str();

        // POSTROUTING MASQUERADE for return traffic
        SafeCommand::new("sudo")
            .and_then(|c| c.arg("iptables"))
            .and_then(|c| c.arg("-t"))
            .and_then(|c| c.arg("nat"))
            .and_then(|c| c.arg("-A"))
            .and_then(|c| c.arg("POSTROUTING"))
            .and_then(|c| c.arg("-p"))
            .and_then(|c| c.arg("tcp"))
            .and_then(|c| c.arg("-d"))
            .and_then(|c| c.arg(settings_ip_ref))
            .and_then(|c| c.arg("-j"))
            .and_then(|c| c.arg("MASQUERADE"))
            .and_then(|cmd| cmd.execute())?;

        // Save rules
        let _ = SafeCommand::new("sudo")
            .and_then(|c| c.arg("sh"))
            .and_then(|c| c.arg("-c"))
            .and_then(|c| c.arg("iptables-save > /etc/iptables/rules.v4"))
            .and_then(|cmd| cmd.execute());

        Ok(())
    }

    async fn start_coredns(&self, container: &str) -> Result<()> {
        SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg(container))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("bash"))
            .and_then(|c| c.arg("-c"))
            .and_then(|c| {
                c.arg("mkdir -p /opt/gbo/logs && nohup /opt/gbo/bin/coredns -conf /opt/gbo/conf/Corefile > /opt/gbo/logs/coredns.log 2>&1 &")
            })
            .and_then(|cmd| cmd.execute())?;

        Ok(())
    }

    async fn reload_dns_zones(&self) -> Result<()> {
        // Update zone files to point to new IP
        let _ = SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg("dns"))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("sh"))
            .and_then(|c| c.arg("-c"))
            .and_then(|c| c.arg("sed -i 's/OLD_IP/NEW_IP/g' /opt/gbo/data/*.zone"))
            .and_then(|cmd| cmd.execute());

        // Restart coredns
        self.start_coredns("dns").await?;

        Ok(())
    }

    fn get_container_settings(&self, container: &str) -> Result<&ContainerSettings> {
        self.components
            .get(container)
            .and_then(|c| c.container.as_ref())
            .context("Container settings not found")
    }

    async fn install_binary_to_container(
        &self,
        container: &str,
        component: &str,
    ) -> Result<()> {
        let config = self
            .components
            .get(component)
            .context("Component not found")?;

        let binary_name = config
            .binary_name
            .as_ref()
            .context("No binary name")?;

        let settings = config
            .container
            .as_ref()
            .context("No container settings")?;

        // Check if already exists
        let check = SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg(container))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("test"))
            .and_then(|c| c.arg("-f"))
            .and_then(|c| c.arg(&settings.binary_path))
            .and_then(|cmd| cmd.execute());

        if check.is_ok() {
            info!("Binary {binary_name} already exists in {container}");
            return Ok(());
        }

        // Download if URL available
        if let Some(url) = &config.download_url {
            self.download_and_push_binary(container, url, binary_name)
                .await?;
        }

        // Make executable
        SafeCommand::new("incus")
            .and_then(|c| c.arg("exec"))
            .and_then(|c| c.arg(container))
            .and_then(|c| c.arg("--"))
            .and_then(|c| c.arg("chmod"))
            .and_then(|c| c.arg("+x"))
            .and_then(|c| c.arg(&settings.binary_path))
            .and_then(|cmd| cmd.execute())?;

        Ok(())
    }

    async fn download_and_push_binary(
        &self,
        container: &str,
        url: &str,
        binary_name: &str,
    ) -> Result<()> {
        let temp_path = format!("/tmp/{binary_name}");

        // Download to temp
        let output = SafeCommand::new("curl")
            .and_then(|c| c.arg("-fsSL"))
            .and_then(|c| c.arg(url))
            .and_then(|cmd| cmd.execute())?;

        std::fs::write(&temp_path, output.stdout)?;

        // Push to container
        let target_path = format!("{container}:/opt/gbo/bin/{binary_name}");
        SafeCommand::new("incus")
            .and_then(|c| c.arg("file"))
            .and_then(|c| c.arg("push"))
            .and_then(|c| c.arg(temp_path.as_str()))
            .and_then(|c| c.arg(target_path.as_str()))
            .and_then(|cmd| cmd.execute())?;

        std::fs::remove_file(&temp_path).ok();
        Ok(())
    }

    async fn provision_certs_to_container(&self, container: &str) -> Result<()> {
        let cert_base = self.base_path.join("conf/system/certificates");
        let ca_cert = cert_base.join("ca/ca.crt");
        if !ca_cert.exists() {
            info!("No CA found at {:?}, skipping cert provisioning for {container}", ca_cert);
            return Ok(());
        }

        let settings = match self.components.get(container).and_then(|c| c.container.as_ref()) {
            Some(s) => s,
            None => {
                trace!("No container settings for {container}, skipping certs");
                return Ok(());
            }
        };

        let cert_dir = cert_base.join(&settings.cert_name);

        for (src, dest) in [
            (&ca_cert, "/opt/gbo/conf/system/certificates/ca/ca.crt".to_string()),
            (&cert_dir.join("server.crt"), format!("/opt/gbo/conf/system/certificates/{}/server.crt", settings.cert_name)),
            (&cert_dir.join("server.key"), format!("/opt/gbo/conf/system/certificates/{}/server.key", settings.cert_name)),
        ] {
            if !src.exists() {
                trace!("Cert file {:?} not found, skipping", src);
                continue;
            }
            SafeCommand::new("incus")
                .and_then(|c| c.arg("exec"))
                .and_then(|c| c.arg(container))
                .and_then(|c| c.arg("--"))
                .and_then(|c| c.arg("mkdir"))
                .and_then(|c| c.arg("-p"))
                .and_then(|c| c.arg(
                    std::path::Path::new(&dest)
                        .parent()
                        .and_then(|p| p.to_str())
                        .unwrap_or(".")
                ))
                .and_then(|cmd| cmd.execute())?;

            SafeCommand::new("incus")
                .and_then(|c| c.arg("file"))
                .and_then(|c| c.arg("push"))
                .and_then(|c| c.arg(src.to_str().unwrap_or_default()))
                .and_then(|c| c.arg(format!("{container}:{dest}").as_str()))
                .and_then(|cmd| cmd.execute())?;
        }

        info!("Provisioned certificates for {container}");
        Ok(())
    }
}
