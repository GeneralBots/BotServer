use super::*;

impl BootstrapManager {
    pub fn new(mode: InstallMode, tenant: Option<String>) -> Self {
        let stack_path = PathBuf::from(get_stack_path());

        Self {
            install_mode: mode,
            tenant,
            stack_path,
        }
    }

    pub fn stack_dir(&self, subpath: &str) -> PathBuf {
        self.stack_path.join(subpath)
    }

    pub fn vault_bin(&self) -> String {
        self.stack_dir("bin/vault/vault")
            .to_str()
            .unwrap_or(&get_stack_path())
            .to_string()
    }

    pub async fn kill_stack_processes(&self) -> anyhow::Result<()> {
        info!("Killing any existing stack processes...");

        let processes = crate::bootstrap::bootstrap_utils::get_processes_to_kill();
        for (name, args) in processes {
            safe_pkill(&[name.as_str()], &args);
        }

        // Give processes time to terminate
        sleep(Duration::from_millis(500)).await;

        info!("Stack processes terminated");
        Ok(())
    }

    pub async fn start_all(&mut self) -> anyhow::Result<()> {
        // If VAULT_ADDR points to a remote server, skip local service startup
        let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_default();
        let is_remote_vault = !vault_addr.is_empty()
            && !vault_addr.contains("localhost")
            && !vault_addr.contains("127.0.0.1");

        if is_remote_vault {
            info!(
                "Remote Vault detected ({}), skipping local service startup",
                vault_addr
            );
            info!("All services are assumed to be running in separate containers");
            return Ok(());
        }

        let pm = PackageManager::new(self.install_mode.clone(), self.tenant.clone())?;
        let skip_local_directory = cfg!(windows)
            && std::env::var("GBO_SKIP_LOCAL_DIRECTORY")
                .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
                .unwrap_or(false);

        info!("Starting bootstrap process (parallel services)...");

        // Phase 1: Start vault first (blocking, ensures credentials available for other services).
        // Vault init includes unseal + seeding credentials - must finish before services that
        // need those credentials (drive, cache) can start.
        if pm.is_installed("vault") {
            if vault_health_check() {
                info!("vault is already running");
            } else {
                info!("Starting vault...");
                let vault_pm = PackageManager::new(self.install_mode.clone(), self.tenant.clone())?;
                tokio::task::spawn_blocking(move || match vault_pm.start("vault") {
                    Ok(_) => info!("vault started and initialized"),
                    Err(e) => warn!("Failed to start vault: {}", e),
                })
                .await
                .ok();
            }
        }

        // Phase 2: Start all other services IN PARALLEL (vault credentials now available).
        // Start them all first (fast nohup launches), then wait for readiness concurrently.
        type ServiceConfig = (&'static str, fn() -> bool, u32, bool);
        let other_services: [ServiceConfig; 6] = [
            ("vector_db", vector_db_health_check, 45, true),
            ("tables", tables_health_check, 0, false),
            ("cache", cache_health_check, 30, true),
            ("drive", drive_health_check, 0, false),
            ("directory", zitadel_health_check, 60, true),
            ("alm", alm_health_check, 0, false),
        ];

        type WaitService = (&'static str, fn() -> bool, u32);
        let mut wait_services: Vec<WaitService> = Vec::new();

        for (name, check_fn, max_wait, need_wait) in &other_services {
            if skip_local_directory && *name == "directory" {
                info!("Skipping local directory service for this Windows development run");
                continue;
            }
            if !pm.is_installed(name) {
                continue;
            }
            let already_running = check_fn();
            if already_running {
                info!("{} is already running", name);
            } else {
                info!("Starting {}...", name);
                match pm.start(name) {
                    Ok(_) => {
                        info!("{} started", name);
                        if *need_wait {
                            wait_services.push((*name, *check_fn, *max_wait));
                        }
                    }
                    Err(e) => {
                        warn!("Failed to start {}: {}", name, e);
                    }
                }
            }
        }

        // Phase 2: Wait for all started services CONCURRENTLY using spawn_blocking
        // Total wait = max of all waits, not sum (since they run in parallel)
        if !wait_services.is_empty() {
            info!(
                "Waiting for {} services to become ready (parallel)...",
                wait_services.len()
            );
            let mut handles = Vec::new();
            for (name, check_fn, max_wait) in &wait_services {
                let name = *name;
                let check_fn = *check_fn;
                let max_wait = *max_wait;
                handles.push(tokio::task::spawn_blocking(move || {
                    let start = std::time::Instant::now();
                    for _ in 0..max_wait {
                        if check_fn() {
                            let elapsed = start.elapsed().as_secs();
                            info!("{} is responding after {}s", name, elapsed);
                            return true;
                        }
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    warn!(
                        "{} did not respond after {} seconds, continuing anyway",
                        name, max_wait
                    );
                    false
                }));
            }
            for h in handles {
                h.await.ok();
            }
        }

        // Phase 3: Post-startup setup tasks (depends on services being ready)

        // Directory OAuth setup (needs Zitadel running)
        if !skip_local_directory && pm.is_installed("directory") {
            let config_path = self.stack_dir("conf/system/directory_config.json");
            if !config_path.exists() {
                info!("Creating OAuth client for Directory service...");
                #[cfg(feature = "directory")]
                match crate::package_manager::setup_directory().await {
                    Ok(_) => info!("OAuth client created successfully"),
                    Err(e) => warn!("Failed to create OAuth client in Directory service (Zitadel): {}. Make sure Zitadel is running on the configured port.", e),
                }
                #[cfg(not(feature = "directory"))]
                info!("Directory feature not enabled, skipping OAuth setup");
            } else {
                info!("Directory config already exists, skipping OAuth setup");
            }
        }

        // ALM setup (needs Forgejo running)
        if pm.is_installed("alm") {
            let already_running = alm_health_check();
            if !already_running {
                info!("Waiting for ALM (Forgejo) to be ready...");
                let mut alm_ready = false;
                for _ in 0..30 {
                    sleep(Duration::from_secs(1)).await;
                    if alm_health_check() {
                        alm_ready = true;
                        break;
                    }
                }
                if alm_ready {
                    match crate::package_manager::setup_alm().await {
                        Ok(_) => info!("ALM setup and runner generation successful"),
                        Err(e) => warn!("ALM setup failed: {}", e),
                    }
                }
            } else {
                match crate::package_manager::setup_alm().await {
                    Ok(_) => info!("ALM setup and runner generation successful"),
                    Err(e) => warn!("ALM setup failed: {}", e),
                }
            }
        }

        // Caddy configuration validation
        let caddy_cmd = SafeCommand::new("caddy")
            .and_then(|c| c.arg("validate"))
            .and_then(|c| c.arg("--config"))
            .and_then(|c| c.arg("/etc/caddy/Caddyfile"));

        match caddy_cmd {
            Ok(cmd) => match cmd.execute() {
                Ok(_) => info!("Caddy configuration is valid"),
                Err(e) => {
                    info!("Caddy configuration not validated: {:?}", e);
                }
            },
            Err(e) => {
                info!("Caddy command unavailable: {:?}", e);
            }
        }

        info!("Bootstrap process completed!");
        Ok(())
    }
}
