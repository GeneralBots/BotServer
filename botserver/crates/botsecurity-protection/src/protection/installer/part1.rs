use super::*;

impl ProtectionInstaller {
    pub fn new() -> Result<Self> {
        let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

        Ok(Self { user })
    }

    #[cfg(windows)]
    pub fn check_admin() -> bool {
        let result = SafeCommand::new("powershell")
            .and_then(|cmd| cmd.args(&[
                "-Command",
                "([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)"
            ]))
            .and_then(|cmd| cmd.execute());

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.trim() == "True"
            }
            Err(_) => false,
        }
    }

    #[cfg(not(windows))]
    pub fn check_root() -> bool {
        SafeCommand::new("id")
            .and_then(|cmd| cmd.arg("-u"))
            .and_then(|cmd| cmd.execute())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
    }

    pub fn install(&self) -> Result<InstallResult> {
        #[cfg(windows)]
        {
            if !Self::check_admin() {
                return Err(anyhow::anyhow!(
                    "This command requires administrator privileges. Run as Administrator."
                ));
            }
        }

        #[cfg(not(windows))]
        {
            if !Self::check_root() {
                return Err(anyhow::anyhow!(
                    "This command requires root privileges. Run with: sudo botserver install protection"
                ));
            }
        }

        info!(
            "Starting security protection installation for user: {}",
            self.user
        );

        let mut result = InstallResult::default();

        #[cfg(windows)]
        {
            match self.configure_windows_security() {
                Ok(()) => {
                    result
                        .packages_installed
                        .push("Windows Defender".to_string());
                    result
                        .packages_installed
                        .push("Windows Firewall".to_string());
                    info!("Windows security configured successfully");
                }
                Err(e) => {
                    warn!("Windows security configuration had issues: {e}");
                    result
                        .warnings
                        .push(format!("Windows security configuration: {e}"));
                }
            }

            match self.update_windows_signatures() {
                Ok(()) => {
                    result.databases_updated = true;
                    info!("Windows security signatures updated");
                }
                Err(e) => {
                    warn!("Windows signature update failed: {e}");
                    result
                        .warnings
                        .push(format!("Windows signature update: {e}"));
                }
            }
        }

        #[cfg(not(windows))]
        {
            match self.install_packages() {
                Ok(installed) => {
                    result.packages_installed = installed;
                    info!("Packages installed: {:?}", result.packages_installed);
                }
                Err(e) => {
                    log::error!("Failed to install packages: {e}");
                    result
                        .errors
                        .push(format!("Package installation failed: {e}"));
                }
            }

            match self.create_sudoers() {
                Ok(()) => {
                    result.sudoers_created = true;
                    info!("Sudoers file created successfully");
                }
                Err(e) => {
                    log::error!("Failed to create sudoers file: {e}");
                    result.errors.push(format!("Sudoers creation failed: {e}"));
                }
            }

            match self.install_lmd() {
                Ok(installed) => {
                    if installed {
                        result.packages_installed.push("maldetect".to_string());
                        info!("LMD (maldetect) installed successfully");
                    }
                }
                Err(e) => {
                    warn!("LMD installation skipped: {e}");
                    result
                        .warnings
                        .push(format!("LMD installation skipped: {e}"));
                }
            }

            match self.update_databases() {
                Ok(()) => {
                    result.databases_updated = true;
                    info!("Security databases updated");
                }
                Err(e) => {
                    warn!("Database update failed: {e}");
                    result.warnings.push(format!("Database update failed: {e}"));
                }
            }
        }

        result.success = result.errors.is_empty();
        Ok(result)
    }

    #[cfg(not(windows))]
    pub(super) fn install_packages(&self) -> Result<Vec<String>> {
        info!("Updating package lists...");

        SafeCommand::new("apt-get")?
            .arg("update")?
            .execute()
            .context("Failed to update package lists")?;

        let mut installed = Vec::new();

        for package in PACKAGES {
            info!("Installing package: {package}");

            let result = SafeCommand::new("apt-get")?
                .arg("install")?
                .arg("-y")?
                .arg(package)?
                .execute();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        installed.push((*package).to_string());
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        warn!("Package {package} installation had issues: {stderr}");
                    }
                }
                Err(e) => {
                    warn!("Failed to install {package}: {e}");
                }
            }
        }

        Ok(installed)
    }

    #[cfg(windows)]
    pub(super) fn configure_windows_security(&self) -> Result<()> {
        info!("Configuring Windows security settings...");

        let _ = SafeCommand::new("powershell")
            .and_then(|cmd| cmd.args(&[
                "-Command",
                "Set-MpPreference -DisableRealtimeMonitoring $false; Set-MpPreference -DisableIOAVProtection $false; Set-MpPreference -DisableScriptScanning $false"
            ]))
            .and_then(|cmd| cmd.execute());

        let _ = SafeCommand::new("netsh")
            .and_then(|cmd| cmd.args(&["advfirewall", "set", "allprofiles", "state", "on"]))
            .and_then(|cmd| cmd.execute());

        let _ = SafeCommand::new("powershell")
            .and_then(|cmd| cmd.args(&[
                "-Command",
                "Set-MpPreference -DisableRemovableDriveScanning $false -DisableScanningMappedNetworkDrivesForFullScan $false"
            ]))
            .and_then(|cmd| cmd.execute());

        info!("Windows security configuration completed");
        Ok(())
    }

    #[cfg(not(windows))]
    pub(super) fn create_sudoers(&self) -> Result<()> {
        

        let content = SUDOERS_CONTENT.replace("{user}", &self.user);

        info!("Creating sudoers file at {SUDOERS_FILE}");

        fs::write(SUDOERS_FILE, &content).context("Failed to write sudoers file")?;

        let permissions = fs::Permissions::from_mode(0o440);
        fs::set_permissions(SUDOERS_FILE, permissions)
            .context("Failed to set sudoers file permissions")?;

        self.validate_sudoers()?;

        info!("Sudoers file created and validated");
        Ok(())
    }

    #[cfg(not(windows))]
    pub(super) fn validate_sudoers(&self) -> Result<()> {
        let output = SafeCommand::new("visudo")
            .and_then(|cmd| cmd.args(&["-c", "-f", SUDOERS_FILE]))
            .and_then(|cmd| cmd.execute())
            .context("Failed to run visudo validation")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            fs::remove_file(SUDOERS_FILE).ok();
            return Err(anyhow::anyhow!("Invalid sudoers file syntax: {stderr}"));
        }

        Ok(())
    }
}
