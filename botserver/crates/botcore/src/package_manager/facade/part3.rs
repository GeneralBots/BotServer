use super::*;

impl PackageManager {
    pub fn install_container(&self, component: &ComponentConfig) -> Result<InstallResult> {
        let container_name = format!("{}-{}", self.tenant, component.name);

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
        self.exec_in_container(
            &container_name,
            "mkdir -p /opt/gbo/bin /opt/gbo/data /opt/gbo/conf /opt/gbo/logs",
        )?;

        self.exec_in_container(
            &container_name,
            "echo 'nameserver 8.8.8.8' > /etc/resolv.conf",
        )?;
        self.exec_in_container(
            &container_name,
            "echo 'nameserver 8.8.4.4' >> /etc/resolv.conf",
        )?;

        self.exec_in_container(&container_name, "apt-get update -qq")?;
        self.exec_in_container(
            &container_name,
            "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq wget unzip curl ca-certificates",
        )?;
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
        self.run_commands(pre_cmds, &container_name, &component.name)?;
        let packages = match self.os_type {
            OsType::Linux => &component.linux_packages,
            OsType::MacOS => &component.macos_packages,
            OsType::Windows => &component.windows_packages,
        };
        if !packages.is_empty() {
            let pkg_list = packages.join(" ");
            self.exec_in_container(&container_name, "apt-get update -qq")?;
            self.exec_in_container(
                &container_name,
                &format!(
                    "DEBIAN_FRONTEND=noninteractive apt-get install -y -qq {}",
                    pkg_list
                ),
            )?;
        }
        if let Some(url) = &component.download_url {
            self.download_in_container(
                &container_name,
                url,
                &component.name,
                component.binary_name.as_deref(),
            )?;
        }
        self.run_commands(post_cmds, &container_name, &component.name)?;
        self.exec_in_container(
            &container_name,
            "useradd --system --no-create-home --shell /bin/false gbuser",
        )?;
        self.mount_container_directories(&container_name, &component.name)?;
        if !component.exec_cmd.is_empty() {
            self.create_container_service(
                &container_name,
                &component.name,
                &component.exec_cmd,
                &component.env_vars,
            )?;
        }
        self.setup_port_forwarding(&container_name, &component.ports)?;

        let container_ip = Self::get_container_ip(&container_name)?;

        if component.name == "vault" {
            Self::initialize_vault(&container_name, &container_ip)?;
        }

        let (connection_info, env_vars) =
            self.generate_connection_info(&component.name, &container_ip, &component.ports);

        trace!(
            "Container installation of '{}' completed in {}",
            component.name,
            container_name
        );

        Ok(InstallResult {
            component: component.name.clone(),
            container_name,
            container_ip,
            ports: component.ports.clone(),
            env_vars,
            connection_info,
        })
    }

    pub(super) fn get_container_ip(container_name: &str) -> Result<String> {
        // Poll up to 30s for DHCP to assign an IPv4
        for _ in 0..15 {
            std::thread::sleep(std::time::Duration::from_secs(2));

            if let Some(o) = safe_lxc(&["list", container_name, "-c", "4", "--format", "csv"]) {
                if o.status.success() {
                    let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    let ip = out.split([' ', '(']).next().unwrap_or("").trim();
                    if !ip.is_empty() && ip.contains('.') {
                        return Ok(ip.to_string());
                    }
                }
            }

            if let Some(o) = safe_lxc(&["exec", container_name, "--", "hostname", "-I"]) {
                if o.status.success() {
                    let out = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    if let Some(ip) = out.split_whitespace().find(|s| s.contains('.')) {
                        return Ok(ip.to_string());
                    }
                }
            }
        }

        // DHCP failed (common in privileged containers where systemd-networkd can't run).
        // Assign a static IP on the lxdbr0 bridge (10.43.228.0/24).
        warn!("DHCP timeout for '{}', assigning static IP via lxdbr0", container_name);
        let static_ip = Self::assign_static_ip(container_name)?;
        Ok(static_ip)
    }

    pub(super) fn assign_static_ip(container_name: &str) -> Result<String> {
        // Discover the host bridge gateway and subnet dynamically from the container's default route.
        // The container already has IPv6 + a link on eth0, so we can read the bridge from the host.
        let bridge_info = SafeCommand::new("ip")
            .and_then(|c| c.args(&["-4", "addr", "show", "lxdbr0"]))
            .ok()
            .and_then(|cmd| cmd.execute().ok())
            .and_then(|o| {
                let out = String::from_utf8_lossy(&o.stdout).to_string();
                // Extract "10.x.x.x/prefix" from "inet 10.x.x.x/24 ..."
                out.lines()
                    .find(|l| l.contains("inet "))
                    .and_then(|l| l.split_whitespace().nth(1))
                    .map(|s| s.to_string())
            });

        let (gateway, prefix) = match bridge_info.as_deref().and_then(|cidr| {
            let (ip, pfx_str) = cidr.split_once('/')?;
            let parts: Vec<&str> = ip.split('.').collect();
            let base = format!("{}.{}.{}.", parts[0], parts[1], parts[2]);
            let pfx = pfx_str.parse::<u8>().ok()?;
            Some((ip.to_string(), format!("{base}{{octet}}/{pfx}")))
        }) {
            Some(pair) => pair,
            None => return Err(anyhow::anyhow!("Cannot determine lxdbr0 subnet")),
        };

        // Derive last octet from MAC to avoid collisions (range 100-249).
        let mac_out = safe_lxc(&["exec", container_name, "--", "cat", "/sys/class/net/eth0/address"]);
        let octet = mac_out
            .and_then(|o| o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string()))
            .and_then(|mac| {
                let parts: Vec<&str> = mac.split(':').collect();
                let a = u8::from_str_radix(parts.get(4)?, 16).ok()?;
                let b = u8::from_str_radix(parts.get(5)?, 16).ok()?;
                Some(100u16 + (u16::from(a) * 256 + u16::from(b)) % 150)
            })
            .unwrap_or(100);

        let cidr = prefix.replace("{octet}", &octet.to_string());
        let ip = cidr.split('/').next().unwrap_or("").to_string();

        safe_lxc(&["exec", container_name, "--", "ip", "addr", "add", &cidr, "dev", "eth0"]);
        safe_lxc(&["exec", container_name, "--", "ip", "route", "add", "default", "via", &gateway]);
        safe_lxc(&["exec", container_name, "--", "bash", "-c",
            &format!("printf 'nameserver 8.8.8.8\\nnameserver 8.8.4.4\\n' > /etc/resolv.conf && \
                      printf '#!/bin/sh\\nip addr add {cidr} dev eth0 2>/dev/null||true\\nip route add default via {gateway} 2>/dev/null||true\\nexit 0\\n' > /etc/rc.local && \
                      chmod +x /etc/rc.local && systemctl enable rc-local 2>/dev/null||true")]);

        info!("Assigned static IP {} to container '{}'", ip, container_name);
        Ok(ip)
    }
}
