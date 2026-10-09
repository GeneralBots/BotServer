use super::*;

/// Bootstrap an entire tenant
pub async fn bootstrap_tenant(
    pm: &PackageManager,
    tenant: &str,
    containers: &[&str],
    source_remote: Option<&str>,
) -> Result<()> {
    info!("Bootstrapping tenant: {tenant}");

    for container in containers {
        pm.bootstrap_container(container, source_remote).await?;
    }

    info!("Tenant {tenant} bootstrapped successfully");
    Ok(())
}
/// Bootstrap all pragmatismo containers
pub async fn bootstrap_pragmatismo(pm: &PackageManager) -> Result<()> {
    let containers = [
        "dns", "email", "alm", "drive", "tables", "system", "proxy", "alm-ci",
        "table-editor",
    ];

    bootstrap_tenant(pm, "pragmatismo", &containers, Some("lxd-source")).await
}
