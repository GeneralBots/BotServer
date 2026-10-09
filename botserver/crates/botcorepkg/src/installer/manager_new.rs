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

    /// Build the full component registry (single source of truth, shared
    /// with `botcore`'s runtime PackageManager).
    pub fn all_components() -> HashMap<String, ComponentConfig> {
        let mut components = HashMap::new();
        installer_regs::register_drive(&mut components);
        installer_regs::register_tables(&mut components);
        installer_regs::register_cache(&mut components);
        installer_regs::register_llm(&mut components);
        installer_regs::register_email(&mut components);
        installer_regs::register_proxy(&mut components);
        installer_regs::register_directory(&mut components);
        installer_regs::register_alm(&mut components);
        installer_regs::register_alm_ci(&mut components);
        installer_regs::register_dns(&mut components);
        installer_regs::register_meeting(&mut components);
        installer_regs2::register_table_editor(&mut components);
        installer_regs2::register_remote_terminal(&mut components);
        installer_regs2::register_devtools(&mut components);
        installer_regs2::register_vector_db(&mut components);
        installer_regs2::register_timeseries_db(&mut components);
        installer_regs2::register_vault(&mut components);
        installer_regs2::register_observability(&mut components);
        installer_regs2::register_host(&mut components);
        components
    }
}
