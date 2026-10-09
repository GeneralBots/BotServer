use super::*;

impl PackageManager {
    pub fn new(mode: InstallMode, tenant: Option<String>) -> Result<Self> {
        let os_type = detect_os();
        let base_path = if mode == InstallMode::Container {
            PathBuf::from("/opt/gbo")
        } else if let Ok(custom_path) = std::env::var("BOTSERVER_STACK_PATH") {
            PathBuf::from(custom_path)
        } else {
            std::env::current_dir()?.join("botserver-stack")
        };
        let tenant = tenant.unwrap_or_else(|| "default".to_string());

        let mut pm = Self {
            mode,
            os_type,
            base_path,
            tenant,
            components: HashMap::new(),
        };
        pm.register_components();
        Ok(pm)
    }

    pub fn with_base_path(
        mode: InstallMode,
        tenant: Option<String>,
        base_path: PathBuf,
    ) -> Result<Self> {
        let os_type = detect_os();
        let tenant = tenant.unwrap_or_else(|| "default".to_string());

        let mut pm = Self {
            mode,
            os_type,
            base_path,
            tenant,
            components: HashMap::new(),
        };
        pm.register_components();
        Ok(pm)
    }

    pub(super) fn register_components(&mut self) {
        self.components = botcorepkg::installer::PackageManager::all_components();
    }

    pub fn start(&self, component: &str) -> Result<std::process::Child> {
        if let Some(component) = self.components.get(component) {
            let bin_path = self.base_path.join("bin").join(&component.name);
            let data_path = self.base_path.join("data").join(&component.name);
            let conf_path = self.base_path.join("conf");
            let logs_path = self.base_path.join("logs").join(&component.name);

            let check_cmd = component
                .effective_check_cmd()
                .replace("{{BIN_PATH}}", &bin_path.to_string_lossy())
                .replace("{{DATA_PATH}}", &data_path.to_string_lossy())
                .replace("{{CONF_PATH}}", &conf_path.to_string_lossy())
                .replace("{{LOGS_PATH}}", &logs_path.to_string_lossy());

            let check_output = safe_sh_command(&check_cmd)
                .map(|o| o.status.success())
                .unwrap_or(false);

            if check_output {
                info!(
                    "Component {} is already running, skipping start",
                    component.name
                );
                return SafeCommand::noop_child()
                    .map_err(|e| anyhow::anyhow!("Failed to create noop process: {}", e));
            }

            // Generate qdrant config.yaml if missing
            if component.name == "vector_db" {
                let qdrant_conf = conf_path.join("vector_db/config.yaml");
                if !qdrant_conf.exists() {
                    let storage = data_path.join("storage");
                    let snapshots = data_path.join("snapshots");
                    let _ = std::fs::create_dir_all(&storage);
                    let _ = std::fs::create_dir_all(&snapshots);
                    let yaml = format!(
                        "storage:\n  storage_path: {}\n  snapshots_path: {}\n\nservice:\n  host: 0.0.0.0\n  http_port: 6333\n  grpc_port: 6334\n  enable_tls: false\n\nlog_level: INFO\n",
                        storage.display(),
                        snapshots.display()
                    );
                    if let Err(e) = std::fs::write(&qdrant_conf, yaml) {
                        warn!("Failed to write qdrant config: {}", e);
                    }
 else {
                        info!("Generated qdrant config at {}", qdrant_conf.display());
                    }
                }
            }

            let rendered_cmd = component
                .effective_exec_cmd()
                .replace("{{BIN_PATH}}", &bin_path.to_string_lossy())
                .replace("{{DATA_PATH}}", &data_path.to_string_lossy())
                .replace("{{CONF_PATH}}", &conf_path.to_string_lossy())
                .replace("{{LOGS_PATH}}", &logs_path.to_string_lossy());

        if let Err(e) = std::fs::create_dir_all(&logs_path) {
            warn!("Failed to create log directory {}: {}", logs_path.display(), e);
        }

            trace!(
                "Starting component {} with command: {}",
                component.name,
                rendered_cmd
            );
            trace!(
                "Working directory: {}, logs_path: {}",
                bin_path.display(),
                logs_path.display()
            );

            let vault_credentials = Self::fetch_vault_credentials();

            let mut evaluated_envs = HashMap::new();
            for (k, v) in &component.env_vars {
                if let Some(var_name) = v.strip_prefix('$') {
                    let value = vault_credentials
                        .get(var_name)
                        .cloned()
                        .or_else(|| std::env::var(var_name).ok())
                        .unwrap_or_default();
                    evaluated_envs.insert(k.clone(), value);
                } else {
                    evaluated_envs.insert(k.clone(), v.clone());
                }
            }

            trace!(
                "About to spawn shell command for {}: {}",
                component.name,
                rendered_cmd
            );
            trace!("Working dir: {}", bin_path.display());
            let abs = crate::os_abstraction::get_abstraction();
            let (shell, flag) = abs.shell_command();
            #[cfg(target_os = "windows")]
            let wrapped_cmd = format!("\"{rendered_cmd}\"");
            #[cfg(target_os = "windows")]
            let script_arg: &str = &wrapped_cmd;
            #[cfg(not(target_os = "windows"))]
            let script_arg: &str = &rendered_cmd;
            let child = SafeCommand::new(shell)
                .and_then(|c| c.arg(flag))
                .and_then(|c| c.raw_shell_script_arg(script_arg))
                .and_then(|c| c.working_dir(&bin_path))
                .and_then(|cmd| cmd.spawn_with_envs(&evaluated_envs));

            trace!("Spawn result for {}: {:?}", component.name, child.is_ok());
            std::thread::sleep(std::time::Duration::from_secs(2));

            trace!(
                "Checking if {} process exists after 2s sleep...",
                component.name
            );
            let check_proc = safe_pgrep(&["-f", &component.name]);
            if let Some(output) = check_proc {
                let pids = String::from_utf8_lossy(&output.stdout);
                trace!("pgrep '{}' result: '{}'", component.name, pids.trim());
            }

            match child {
                Ok(c) => {
                    trace!("Component {} spawned successfully", component.name);

                    if component.name == "vault" && self.mode == InstallMode::Local {
                        if let Some(addr) = evaluated_envs.get("VAULT_ADDR") {
                            if !addr.is_empty() {
                                std::env::set_var("VAULT_ADDR", addr);
                            }
                        }
                        if let Err(e) = self.initialize_vault_local() {
                            warn!("Failed to initialize Vault: {}", e);
                        }
                    }

                    Ok(c)
                }
                Err(e) => {
                    log::error!("Spawn failed for {}: {}", component.name, e);
                    let err_msg = e.to_string();
                    if err_msg.contains("already running")
                        || err_msg.contains("be running")
                        || component.name == "tables"
                    {
                        trace!("Component {} may already be running", component.name);
                        if component.name == "vault" && self.mode == InstallMode::Local {
                            let _ = self.ensure_env_file_exists();
                        }
                        
                        Err(anyhow::anyhow!("Already running"))
                    } else {
                        Err(anyhow::anyhow!("Failed to start component {}: {}", component.name, e))
                    }
                }
            }
        } else {
            Err(anyhow::anyhow!("Component not found: {}", component))
        }
    }
}
