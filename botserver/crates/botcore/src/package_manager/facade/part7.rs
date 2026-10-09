use super::*;

impl PackageManager {
    pub fn install_binary(
        &self,
        temp_file: &std::path::Path,
        bin_path: &std::path::Path,
        name: &str,
    ) -> Result<()> {
        #[cfg(target_os = "windows")]
        let name = if !name.ends_with(".exe") {
            format!("{}.exe", name)
        } else {
            name.to_string()
        };
        #[cfg(not(target_os = "windows"))]
        let name = name.to_string();
        let final_path = bin_path.join(name);

        if temp_file.to_string_lossy().contains("botserver-installers") {
            std::fs::copy(temp_file, &final_path)?;
        } else {
            std::fs::rename(temp_file, &final_path)?;
        }
        self.make_executable(&final_path)?;
        Ok(())
    }
    pub fn make_executable(&self, path: &std::path::Path) -> Result<()> {
        #[cfg(unix)]
        {
            let _ = botlib::os::fs::get_permissions_manager().set_executable(path);
        }
        #[cfg(windows)]
        {
            trace!("Skipping chmod on Windows for: {}", path.display());
        }
        Ok(())
    }
    pub fn run_commands(&self, commands: &[String], target: &str, component: &str) -> Result<()> {
        self.run_commands_with_password(commands, target, component, "")
    }

    pub fn run_commands_with_password(&self, commands: &[String], target: &str, component: &str, db_password_override: &str) -> Result<()> {
        let bin_path = if target == "local" {
            self.base_path.join("bin").join(component)
        } else {
            PathBuf::from("/opt/gbo/bin")
        };
        let data_path = if target == "local" {
            self.base_path.join("data").join(component)
        } else {
            PathBuf::from("/opt/gbo/data")
        };

        let conf_path = if target == "local" {
            self.base_path.join("conf")
        } else {
            PathBuf::from("/opt/gbo/conf")
        };
        let logs_path = if target == "local" {
            self.base_path.join("logs").join(component)
        } else {
            PathBuf::from("/opt/gbo/logs")
        };

        let db_password = if let Ok(env_pwd) = std::env::var("BOOTSTRAP_DB_PASSWORD") {
            env_pwd
        } else if !db_password_override.is_empty() {
            db_password_override.to_string()
        } else {
            match get_database_url_sync() {
                Ok(url) => {
                    let (_, password, _, _, _) = parse_database_url(&url);
                    password
                }
                Err(_) => {
                    trace!("Vault not available for DB_PASSWORD, using empty string");
                    String::new()
                }
            }
        };

        for cmd in commands {
            let rendered_cmd = cmd
                .replace("{{BIN_PATH}}", &bin_path.to_string_lossy())
                .replace("{{DATA_PATH}}", &data_path.to_string_lossy())
                .replace("{{CONF_PATH}}", &conf_path.to_string_lossy())
                .replace("{{LOGS_PATH}}", &logs_path.to_string_lossy())
                .replace("{{STACK_PATH}}", &self.base_path.to_string_lossy())
                .replace("{{DATA_PATH_FWD}}", &data_path.to_string_lossy().replace('\\', "/"))
                .replace("{{DB_PASSWORD}}", &db_password);
            if target == "local" {
                trace!("Executing command: {}", rendered_cmd);
                #[cfg(target_os = "windows")]
                let cmd_result = {
                    let wrapped = format!("\"{rendered_cmd}\"");
                    let cmd = SafeCommand::new("cmd")
                        .and_then(|c| c.arg("/C"))
                        .and_then(|c| c.raw_shell_script_arg(&wrapped))
                        .and_then(|c| c.working_dir(&bin_path))
                        .map_err(|e| anyhow::anyhow!("Failed to build cmd command: {}", e))?;
                    let output = cmd.execute().with_context(|| {
                        format!("Failed to execute command for component '{}'", component)
                    })?;
                    if !output.status.success() {
                        error!(
                            "Command '{}' failed. Stderr: {}",
                            rendered_cmd,
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    output
                };
                #[cfg(not(target_os = "windows"))]
                let cmd_result = {
                    let cmd = SafeCommand::new("bash")
                        .and_then(|c| c.arg("-c"))
                        .and_then(|c| c.trusted_shell_script_arg(&rendered_cmd))
                        .and_then(|c| c.working_dir(&bin_path))
                        .map_err(|e| anyhow::anyhow!("Failed to build bash command: {}", e))?;
                    cmd.execute().with_context(|| {
                        format!("Failed to execute command for component '{}'", component)
                    })?
                };
                if !cmd_result.status.success() {
                    error!(
                        "Command had non-zero exit: {}",
                        String::from_utf8_lossy(&cmd_result.stderr)
                    );
                }
            } else {
                self.exec_in_container(target, &rendered_cmd)?;
            }
        }
        Ok(())
    }
    pub fn exec_in_container(&self, container: &str, command: &str) -> Result<()> {
        info!("Executing in container {}: {}", container, command);
        let output = safe_lxc_exec_in_container(container, command)
            .ok_or_else(|| anyhow::anyhow!("Failed to execute lxc command"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            error!(
                "Container command failed.\nCommand: {}\nStderr: {}\nStdout: {}",
                command, stderr, stdout
            );
            return Err(anyhow::anyhow!(
                "Container command failed: {}",
                if stderr.is_empty() {
                    stdout.to_string()
                } else {
                    stderr.to_string()
                }
            ));
        }
        Ok(())
    }
    pub fn download_in_container(
        &self,
        container: &str,
        url: &str,
        _component: &str,
        binary_name: Option<&str>,
    ) -> Result<()> {
        let download_cmd = format!("wget -O /tmp/download.tmp {}", url);
        self.exec_in_container(container, &download_cmd)?;
        let path = std::path::Path::new(url);
        let is_tar_gz = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tgz"))
            || (path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"))
                && path
                    .file_stem()
                    .and_then(|s| std::path::Path::new(s).extension())
                    .is_some_and(|e| e.eq_ignore_ascii_case("tar")));
        let is_zip = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"));
        if is_tar_gz {
            // Try tar, fallback to trying to find it in common paths
            self.exec_in_container(container, "/usr/bin/tar -xzf /tmp/download.tmp -C /opt/gbo/bin || tar -xzf /tmp/download.tmp -C /opt/gbo/bin")?;
        } else if is_zip {
            self.exec_in_container(container, "unzip -o /tmp/download.tmp -d /opt/gbo/bin")?;
        } else if let Some(name) = binary_name {
            let mv_cmd = format!(
                "mv /tmp/download.tmp /opt/gbo/bin/{} && chmod +x /opt/gbo/bin/{}",
                name, name
            );
            self.exec_in_container(container, &mv_cmd)?;
        }
        self.exec_in_container(container, "rm -f /tmp/download.tmp")?;
        Ok(())
    }
    pub fn mount_container_directories(&self, container: &str, component: &str) -> Result<()> {
        let host_base = format!("/opt/gbo/tenants/{}/{}", self.tenant, component);
        for dir in &["data", "conf", "logs"] {
            let host_path = format!("{}/{}", host_base, dir);
            std::fs::create_dir_all(&host_path)?;
            let device_name = format!("{}-{}", component, dir);
            let container_path = format!("/opt/gbo/{}", dir);
            let _ = safe_lxc(&["config", "device", "remove", container, &device_name]);
            let source_arg = format!("source={}", host_path);
            let path_arg = format!("path={}", container_path);
            let output = safe_lxc(&[
                "config",
                "device",
                "add",
                container,
                &device_name,
                "disk",
                &source_arg,
                &path_arg,
            ]);
            let output = match output {
                Some(o) => o,
                None => {
                    warn!("Failed to execute lxc mount command for {}", dir);
                    continue;
                }
            };
            if !output.status.success() {
                warn!("Failed to mount {} in container {}", dir, container);
            }
            trace!(
                "Mounted {} to {} in container {}",
                host_path,
                container_path,
                container
            );
        }
        Ok(())
    }
    pub fn create_container_service(
        &self,
        container: &str,
        component: &str,
        exec_cmd: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<()> {
        let db_password = match get_database_url_sync() {
            Ok(url) => {
                let (_, password, _, _, _) = parse_database_url(&url);
                password
            }
            Err(_) => {
                trace!(
                    "Vault not available for DB_PASSWORD in container service, using empty string"
                );
                String::new()
            }
        };

        let rendered_cmd = exec_cmd
            .replace("{{DB_PASSWORD}}", &db_password)
            .replace("{{BIN_PATH}}", "/opt/gbo/bin")
            .replace("{{DATA_PATH}}", "/opt/gbo/data")
            .replace("{{CONF_PATH}}", "/opt/gbo/conf")
            .replace("{{LOGS_PATH}}", "/opt/gbo/logs");
        let mut env_section = String::new();
        for (key, value) in env_vars {
            let rendered_value = value
                .replace("{{DATA_PATH}}", "/opt/gbo/data")
                .replace("{{BIN_PATH}}", "/opt/gbo/bin")
                .replace("{{CONF_PATH}}", "/opt/gbo/conf")
                .replace("{{LOGS_PATH}}", "/opt/gbo/logs");
            let _ = writeln!(env_section, "Environment={key}={rendered_value}");
        }
        let service_content = format!(
            "[Unit]\nDescription={} Service\nAfter=network.target\n\n[Service]\nType=simple\n{}ExecStart={}\nWorkingDirectory=/opt/gbo/data\nRestart=always\nRestartSec=10\nUser=root\n\n[Install]\nWantedBy=multi-user.target\n",
            component, env_section, rendered_cmd
        );
        let service_file = format!("/tmp/{}.service", component);
        std::fs::write(&service_file, &service_content)?;
        let dest = format!("{container}/etc/systemd/system/{component}.service");
        let output = safe_lxc(&["file", "push", &service_file, &dest])
            .ok_or_else(|| anyhow::anyhow!("Failed to execute lxc file push command"))?;
        if !output.status.success() {
            warn!("Failed to push service file to container");
        }
        self.exec_in_container(container, "systemctl daemon-reload")?;
        self.exec_in_container(container, &format!("systemctl enable {}", component))?;
        self.exec_in_container(container, &format!("systemctl start {}", component))?;
        std::fs::remove_file(&service_file)?;
        trace!(
            "Created and started service in container {}: {}",
            container,
            component
        );
        Ok(())
    }
}
