use super::*;

impl ContainerOperations for PackageManager {
    async fn provision_certs_to_container_ssh(&self, container: &str, ssh_host: &str) -> Result<()> {
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

        // Copy certs to temp, then SCP to host, then incus push
        for (src, dest) in [
            (&ca_cert, "/opt/gbo/conf/system/certificates/ca/ca.crt".to_string()),
            (&cert_dir.join("server.crt"), format!("/opt/gbo/conf/system/certificates/{}/server.crt", settings.cert_name)),
            (&cert_dir.join("server.key"), format!("/opt/gbo/conf/system/certificates/{}/server.key", settings.cert_name)),
        ] {
            if !src.exists() {
                trace!("Cert file {:?} not found, skipping", src);
                continue;
            }

            let remote_tmp = format!(
                "/tmp/{}-{}",
                container,
                src.file_name().and_then(|f| f.to_str()).unwrap_or("unknown")
            );
            let scp_src = src.to_str().unwrap_or_default();
            let scp_dest = format!("{}:{}", ssh_host, remote_tmp);

            // SCP to host
            SafeCommand::new("scp")
                .and_then(|c| c.arg(scp_src))
                .and_then(|c| c.arg(scp_dest.as_str()))
                .and_then(|c| c.arg("-o"))
                .and_then(|c| c.arg("StrictHostKeyChecking=no"))
                .and_then(|c| c.arg("-o"))
                .and_then(|c| c.arg("ConnectTimeout=10"))
                .and_then(|cmd| cmd.execute())?;

            // incus exec mkdir on host
            SafeCommand::new("ssh")
                .and_then(|c| c.arg(ssh_host))
                .and_then(|c| c.arg("-o"))
                .and_then(|c| c.arg("StrictHostKeyChecking=no"))
                .and_then(|c| c.arg("sudo"))
                .and_then(|c| c.arg("incus"))
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

            // incus file push on host
            let target = format!("{}:{}", container, dest);
            SafeCommand::new("ssh")
                .and_then(|c| c.arg(ssh_host))
                .and_then(|c| c.arg("-o"))
                .and_then(|c| c.arg("StrictHostKeyChecking=no"))
                .and_then(|c| c.arg("sudo"))
                .and_then(|c| c.arg("incus"))
                .and_then(|c| c.arg("file"))
                .and_then(|c| c.arg("push"))
                .and_then(|c| c.arg(remote_tmp.as_str()))
                .and_then(|c| c.arg(target.as_str()))
                .and_then(|cmd| cmd.execute())?;

            // Cleanup temp on host
            let _ = SafeCommand::new("ssh")
                .and_then(|c| c.arg(ssh_host))
                .and_then(|c| c.arg("sudo"))
                .and_then(|c| c.arg("rm"))
                .and_then(|c| c.arg("-f"))
                .and_then(|c| c.arg(remote_tmp.as_str()))
                .and_then(|cmd| cmd.execute());
        }

        info!("Provisioned certificates for {container} via SSH");
        Ok(())
    }
}
