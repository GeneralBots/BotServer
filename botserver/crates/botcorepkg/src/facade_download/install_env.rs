use super::*;

pub fn create_directories(base_path: &std::path::Path, component: &str) -> Result<()> {
    for dir in &["bin", "data", "conf", "logs"] {
        let path = base_path.join(dir).join(component);
        std::fs::create_dir_all(&path)
            .context(format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}

pub fn install_system_packages(component: &ComponentConfig, os_type: &OsType) -> Result<()> {    let packages = match os_type {
        OsType::Linux => &component.linux_packages,
        OsType::MacOS => &component.macos_packages,
        OsType::Windows => &component.windows_packages,
    };
    if packages.is_empty() {
        return Ok(());
    }
    trace!(
        "Installing {} system packages for component '{}'",
        packages.len(),
        component.name
    );
    match os_type {
        OsType::Linux => {
            if let Some(output) = safe_apt_get(&["update"]) {
                if !output.status.success() {
                    warn!("apt-get update had issues");
                }
            }
            let mut args = vec!["install", "-y"];
            for pkg in packages {
                args.push(pkg.as_str());
            }
            if let Some(output) = safe_apt_get(&args) {
                if !output.status.success() {
                    warn!("Some packages may have failed to install");
                }
            }
        }
        OsType::MacOS => {
            let mut args = vec!["install"];
            for pkg in packages {
                args.push(pkg.as_str());
            }
            if let Some(output) = safe_brew(&args) {
                if !output.status.success() {
                    warn!("Homebrew installation had warnings");
                }
            }
        }
        OsType::Windows => {
            trace!("Windows packages managed via GB portable binaries, skipping system package manager");
        }
    }
    Ok(())
}
