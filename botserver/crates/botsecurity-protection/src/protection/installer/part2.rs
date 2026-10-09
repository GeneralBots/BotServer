use super::*;

impl ProtectionInstaller {
    pub(super) fn install_lmd(&self) -> Result<bool> {
        let maldet_path = Path::new("/usr/local/sbin/maldet");
        if maldet_path.exists() {
            info!("LMD already installed");
            return Ok(false);
        }

        info!("Installing Linux Malware Detect (LMD)...");

        let temp_dir = "/tmp/maldetect_install";
        fs::create_dir_all(temp_dir).ok();

        let download_result = SafeCommand::new("curl")?
            .arg("-sL")?
            .arg("-o")?
            .arg("/tmp/maldetect-current.tar.gz")?
            .arg("https://www.rfxn.com/downloads/maldetect-current.tar.gz")?
            .execute();

        if download_result.is_err() {
            return Err(anyhow::anyhow!("Failed to download LMD"));
        }

        SafeCommand::new("tar")?
            .arg("-xzf")?
            .arg("/tmp/maldetect-current.tar.gz")?
            .arg("-C")?
            .arg(temp_dir)?
            .execute()
            .context("Failed to extract LMD archive")?;

        let entries = fs::read_dir(temp_dir)?;
        let mut install_dir = None;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("maldetect"))
            {
                install_dir = Some(path);
                break;
            }
        }

        let install_dir =
            install_dir.ok_or_else(|| anyhow::anyhow!("LMD install directory not found"))?;
        let install_script = install_dir.join("install.sh");

        if !install_script.exists() {
            return Err(anyhow::anyhow!("LMD install.sh not found"));
        }

        SafeCommand::new("bash")?
            .arg("-c")?
            .shell_script_arg(&format!("cd {} && ./install.sh", install_dir.display()))?
            .execute()
            .context("Failed to run LMD installer")?;

        fs::remove_dir_all(temp_dir).ok();
        fs::remove_file("/tmp/maldetect-current.tar.gz").ok();

        Ok(true)
    }

    #[cfg(not(windows))]
    pub(super) fn update_databases(&self) -> Result<()> {
        info!("Updating security tool databases...");

        if Path::new("/usr/bin/rkhunter").exists() {
            info!("Updating RKHunter database...");
            let result = SafeCommand::new("rkhunter")?.arg("--update")?.execute();
            if let Err(e) = result {
                warn!("RKHunter update failed: {e}");
            }
        }

        if Path::new("/usr/bin/freshclam").exists() {
            info!("Updating ClamAV signatures...");
            let result = SafeCommand::new("freshclam")?.execute();
            if let Err(e) = result {
                warn!("ClamAV update failed: {e}");
            }
        }

        if Path::new("/usr/bin/suricata-update").exists() {
            info!("Updating Suricata rules...");
            let result = SafeCommand::new("suricata-update")?.execute();
            if let Err(e) = result {
                warn!("Suricata update failed: {e}");
            }
        }

        if Path::new("/usr/local/sbin/maldet").exists() {
            info!("Updating LMD signatures...");
            let result = SafeCommand::new("maldet")?.arg("--update-sigs")?.execute();
            if let Err(e) = result {
                warn!("LMD update failed: {e}");
            }
        }

        Ok(())
    }

    #[cfg(windows)]
    pub(super) fn update_windows_signatures(&self) -> Result<()> {
        info!("Updating Windows Defender signatures...");

        let result = SafeCommand::new("powershell")
            .and_then(|cmd| cmd.args(&[
                "-Command",
                "Update-MpSignature; Write-Host 'Windows Defender signatures updated'",
            ]))
            .and_then(|cmd| cmd.execute());

        match result {
            Ok(output) => {
                if output.status.success() {
                    info!("Windows Defender signatures updated successfully");
                } else {
                    warn!("Windows Defender signature update had issues");
                }
            }
            Err(e) => {
                warn!("Failed to update Windows Defender signatures: {e}");
            }
        }

        Ok(())
    }

    pub fn uninstall(&self) -> Result<UninstallResult> {
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
                    "This command requires root privileges. Run with: sudo botserver remove protection"
                ));
            }
        }

        info!("Removing security protection components...");

        let mut result = UninstallResult::default();

        #[cfg(not(windows))]
        {
            if Path::new(SUDOERS_FILE).exists() {
                match fs::remove_file(SUDOERS_FILE) {
                    Ok(()) => {
                        result.sudoers_removed = true;
                        info!("Removed sudoers file");
                    }
                    Err(e) => {
                        result.errors.push(format!("Failed to remove sudoers: {e}"));
                    }
                }
            }

            result.message = "Protection sudoers removed. Packages were NOT uninstalled - remove manually if needed.".to_string();
        }

        #[cfg(windows)]
        {
            result.message = "Windows protection uninstalled. Windows Defender and Firewall settings were not modified - remove manually if needed.".to_string();
        }

        result.success = result.errors.is_empty();
        Ok(result)
    }

    pub fn verify(&self) -> VerifyResult {
        let mut result = VerifyResult::default();

        #[cfg(not(windows))]
        {
            for package in PACKAGES {
                let binary = match *package {
                    "clamav" | "clamav-daemon" => "clamscan",
                    other => other,
                };

                let check = SafeCommand::new("which")
                    .and_then(|cmd| cmd.arg(binary))
                    .and_then(|cmd| cmd.execute());

                let installed = check.map(|o| o.status.success()).unwrap_or(false);
                result.tools.push(ToolVerification {
                    name: (*package).to_string(),
                    installed,
                    sudo_configured: false,
                });
            }

            let maldet_installed = Path::new("/usr/local/sbin/maldet").exists();
            result.tools.push(ToolVerification {
                name: "maldetect".to_string(),
                installed: maldet_installed,
                sudo_configured: false,
            });

            result.sudoers_exists = Path::new(SUDOERS_FILE).exists();

            if result.sudoers_exists {
                if let Ok(content) = fs::read_to_string(SUDOERS_FILE) {
                    for tool in &mut result.tools {
                        tool.sudo_configured = content.contains(&tool.name)
                            || (tool.name == "clamav" && content.contains("clamav-daemon"))
                            || (tool.name == "clamav-daemon" && content.contains("clamav-daemon"));
                    }
                }
            }

            result.all_installed = result
                .tools
                .iter()
                .filter(|t| t.name != "clamav-daemon")
                .all(|t| t.installed);
            result.all_configured = result.sudoers_exists
                && result
                    .tools
                    .iter()
                    .all(|t| t.sudo_configured || !t.installed);
        }

        #[cfg(windows)]
        {
            for (tool_name, tool_cmd) in WINDOWS_TOOLS {
                let check = SafeCommand::new(tool_cmd)
                    .and_then(|cmd| cmd.arg("--version"))
                    .and_then(|cmd| cmd.execute());

                let installed = check.map(|o| o.status.success()).unwrap_or(false);
                result.tools.push(ToolVerification {
                    name: tool_name.to_string(),
                    installed,
                    sudo_configured: true, // Windows tools are typically pre-configured
                });
            }

            result.sudoers_exists = false; // No sudoers on Windows
            result.all_installed = result.tools.iter().all(|t| t.installed);
            result.all_configured = true; // Windows tools are pre-configured
        }

        result
    }
}
