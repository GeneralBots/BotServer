use super::*;

impl PackageManager {
    pub fn setup_port_forwarding(&self, container: &str, ports: &[u16]) -> Result<()> {
        for port in ports {
            let device_name = format!("port-{}", port);
            let _ = safe_lxc(&["config", "device", "remove", container, &device_name]);
            let listen_arg = format!("listen=tcp:0.0.0.0:{port}");
            let connect_arg = format!("connect=tcp:127.0.0.1:{port}");
            let output = safe_lxc(&[
                "config",
                "device",
                "add",
                container,
                &device_name,
                "proxy",
                &listen_arg,
                &connect_arg,
            ])
            .ok_or_else(|| anyhow::anyhow!("Failed to execute lxc port forward command"))?;
            if !output.status.success() {
                warn!("Failed to setup port forwarding for port {}", port);
            }
            trace!(
                "Port forwarding configured: {} -> container {}",
                port,
                container
            );
        }
        Ok(())
    }
}
