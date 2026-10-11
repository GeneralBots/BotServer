pub(crate) use crate::package_manager::PackageManager;
pub(crate) use botlib::security::command_guard::SafeCommand;
pub(crate) use anyhow::{Context, Result};
pub use botcorepkg::container::{ContainerSettings, NatRule};

/// Extension trait for PackageManager to handle container operations
pub trait ContainerOperations {
    /// Bootstrap a container with all its services and NAT rules
    fn bootstrap_container(
        &self,
        container_name: &str,
        source_lxd: Option<&str>,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Cleanup existing socat and proxy devices
    fn cleanup_existing(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Copy container from LXD source
    fn copy_container(
        &self,
        source_remote: &str,
        name: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Add eth0 network to container
    fn ensure_network(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Sync data from host to container
    fn sync_data_to_container(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Fix file permissions based on container user
    fn fix_permissions(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Install systemd service file and start
    fn install_systemd_service(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Configure iptables NAT rules on host
    fn configure_iptables_nat(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Start CoreDNS (special case)
    fn start_coredns(&self, container: &str) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Reload DNS zones with new IPs
    fn reload_dns_zones(&self) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Get container settings for a component
    fn get_container_settings(&self, container: &str) -> Result<&ContainerSettings>;

    /// Install binary to container from URL
    fn install_binary_to_container(
        &self,
        container: &str,
        component: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Download and push binary to container
    fn download_and_push_binary(
        &self,
        container: &str,
        url: &str,
        binary_name: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Provision TLS certificates to a container via incus file push
    fn provision_certs_to_container(
        &self,
        container: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Provision TLS certificates to a container via SSH to remote host
    fn provision_certs_to_container_ssh(
        &self,
        container: &str,
        ssh_host: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Service file templates for various containers
pub mod service_templates {
    /// CoreDNS service template
    pub fn dns_service() -> &'static str {
        r#"[Unit]
Description=CoreDNS
After=network.target

[Service]
User=root
WorkingDirectory=/opt/gbo
ExecStart=/opt/gbo/bin/coredns -conf /opt/gbo/conf/Corefile
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    }

    /// Stalwart email service template
    pub fn email_service() -> &'static str {
        r#"[Unit]
Description=Stalwart Mail Server
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/gbo
ExecStart=/opt/gbo/bin/stalwart --config /opt/gbo/conf/config.toml
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    }

    /// Caddy proxy service template
    pub fn proxy_service() -> &'static str {
        r#"[Unit]
Description=Caddy Reverse Proxy
After=network.target

[Service]
User=gbuser
Group=gbuser
WorkingDirectory=/opt/gbo
ExecStart=/usr/bin/caddy run --config /opt/gbo/conf/config --adapter caddyfile
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    }

    /// Forgejo ALM service template
    pub fn alm_service() -> &'static str {
        r#"[Unit]
Description=Forgejo Git Server
After=network.target

[Service]
User=gbuser
Group=gbuser
WorkingDirectory=/opt/gbo
ExecStart=/opt/gbo/bin/forgejo web --config /opt/gbo/conf/app.ini
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    }

    /// MinIO drive service template
    pub fn minio_service() -> &'static str {
        r#"[Unit]
Description=MinIO Object Storage
After=network-online.target
Wants=network-online.target

[Service]
User=gbuser
Group=gbuser
WorkingDirectory=/opt/gbo
ExecStart=/opt/gbo/bin/minio server --console-address :4646 /opt/gbo/data
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
"#
    }

    /// PostgreSQL tables service template
    pub fn tables_service() -> &'static str {
        r#"[Unit]
Description=PostgreSQL
After=network.target

[Service]
User=gbuser
Group=gbuser
WorkingDirectory=/opt/gbo
ExecStart=/opt/gbo/bin/postgres -D /opt/gbo/data -c config_file=/opt/gbo/conf/postgresql.conf
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
"#
    }
}
// CI trigger
// CI trigger Fri Apr 17 17:34:16 -03 2026

use log::{info, trace};
use std::path::Path;

/// Service-to-container mapping for certificate provisioning
pub const SERVICE_MAP: &[(&str, &str, &str, u16, u16)] = &[
    ("cache",     "cache",     "10.0.0.5",  6379, 6380),
    ("drive",     "drive",     "10.0.0.15", 9100, 9001),
    ("directory", "directory", "10.0.0.4",  8080, 8443),
    ("vectordb",  "vectordb",  "10.0.0.17", 6333, 6334),
    ("alm",       "alm",       "10.0.0.13", 4747, 4748),
    ("meet",      "meet",      "10.0.0.11", 7880, 7881),
    ("vault",     "vault",     "10.0.0.6",  8200, 8201),
    ("tables",    "tables",    "10.0.0.18", 5432, 5433),
    ("api",       "bot",       "10.0.0.21", 5858, 8443),
    ("llm",       "models",    "10.0.0.16", 8081, 8082),
];

mod part2;
mod part3;
mod part4;

pub use part4::*;
