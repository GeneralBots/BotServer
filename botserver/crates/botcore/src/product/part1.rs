use super::*;

impl Default for ProductConfig {
    fn default() -> Self {
        let mut apps = HashSet::new();
        // All apps enabled by default (docs/slides excluded — OOXML SDK gate)
        for app in &[
            "chat",
            "mail",
            "calendar",
            "drive",
            "tasks",
            "paper",
            "sheet",
            "meet",
            "research",
            "sources",
            "analytics",
            "admin",
            "monitoring",
            "settings",
            "attendant",
            "tools",
            "video",
            "player",
            "canvas",
            "social",
            "people",
            "crm",
            "tickets",
            "billing",
            "products",
            "designer",
            "workspace",
            "project",
            "goals",
            "editor",
            "learn",
            "vibe",
            "campaigns",
            "lists",
            "templates",
            "terminal",
            "browser",
            "database",
            "plan",
            "compliance",
            "tax",
            "vision",
            "fraud",
            "erp",
            "integrations",
            "itsm",
            "hr",
            "banking",
            "sales",
            "pos",
            "retail",
            "handoff",
            "kyc",
            "biometry",
            "timeclock",
            "m365",
            "office365",
            "minutes",
        ] {
            apps.insert(app.to_string());
        }

        Self {
            name: "General Bots".to_string(),
            apps,
            preview_apps: HashSet::new(),
            theme: "sentient".to_string(),
            logo: None,
            favicon: None,
            primary_color: None,
            support_email: None,
            docs_url: None,
            copyright: None,
            search_enabled: Some(false),
            menu_launcher_enabled: Some(false),
            sidebar: None,
        }
    }
}
impl ProductConfig {
    /// Load configuration from .product file
    pub fn load() -> Result<Self, ProductConfigError> {
        let paths = [".product", "./botserver/.product", "../.product"];

        let mut content = None;
        for path in &paths {
            if Path::new(path).exists() {
                content = Some(fs::read_to_string(path).map_err(ProductConfigError::IoError)?);
                info!("Loaded product configuration from: {}", path);
                break;
            }
        }

        let content = match content {
            Some(c) => c,
            None => {
                warn!("No .product file found, using default configuration");
                return Ok(Self::default());
            }
        };

        Self::parse(&content)
    }

    /// Parse configuration from string content
    pub fn parse(content: &str) -> Result<Self, ProductConfigError> {
        let mut config = Self::default();
        let mut apps_specified = false;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key=value pairs
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_lowercase();
                let value = value.trim();

                match key.as_str() {
                    "name" => {
                        if !value.is_empty() {
                            config.name = value.to_string();
                        }
                    }
                    "apps" => {
                        apps_specified = true;
                        config.apps.clear();
                        for app in value.split(',') {
                            let app = app.trim().to_lowercase();
                            if !app.is_empty() {
                                config.apps.insert(app);
                            }
                        }
                    }
                    "preview_apps" => {
                        config.preview_apps.clear();
                        for app in value.split(',') {
                            let app = app.trim().to_lowercase();
                            if !app.is_empty() {
                                config.preview_apps.insert(app);
                            }
                        }
                    }
                    "theme" => {
                        if !value.is_empty() {
                            config.theme = value.to_string();
                        }
                    }
                    "logo" => {
                        if !value.is_empty() {
                            config.logo = Some(value.to_string());
                        }
                    }
                    "favicon" => {
                        if !value.is_empty() {
                            config.favicon = Some(value.to_string());
                        }
                    }
                    "primary_color" => {
                        if !value.is_empty() {
                            config.primary_color = Some(value.to_string());
                        }
                    }
                    "support_email" => {
                        if !value.is_empty() {
                            config.support_email = Some(value.to_string());
                        }
                    }
                    "docs_url" => {
                        if !value.is_empty() {
                            config.docs_url = Some(value.to_string());
                        }
                    }
                    "copyright" => {
                        if !value.is_empty() {
                            config.copyright = Some(value.to_string());
                        }
                    }
                    "search_enabled" => {
                        let enabled = value.eq_ignore_ascii_case("true")
                            || value == "1"
                            || value.eq_ignore_ascii_case("yes");
                        config.search_enabled = Some(enabled);
                    }
                    "menu_launcher_enabled" => {
                        let enabled = value.eq_ignore_ascii_case("true")
                            || value == "1"
                            || value.eq_ignore_ascii_case("yes");
                        config.menu_launcher_enabled = Some(enabled);
                    }
                    "sidebar" => {
                        let enabled = value.eq_ignore_ascii_case("true")
                            || value == "1"
                            || value.eq_ignore_ascii_case("yes");
                        config.sidebar = Some(enabled);
                    }
                    _ => {
                        warn!("Unknown product configuration key: {}", key);
                    }
                }
            }
        }

        if !apps_specified {
            info!("No apps specified in .product, all apps enabled by default");
        }

        info!(
            "Product config loaded: name='{}', apps={:?}, theme='{}'",
            config.name, config.apps, config.theme
        );

        Ok(config)
    }

    /// Check if an app is enabled
    pub fn is_app_enabled(&self, app: &str) -> bool {
        self.apps.contains(&app.to_lowercase())
    }

    /// Check whether an app is a preview application, i.e. one that is only
    /// surfaced while Preview mode is on (#1348). Preview applications are not
    /// part of `apps`, so `is_app_enabled` still answers `false` for them: the
    /// switch decides what is visible, never whether the app is installed.
    pub fn is_app_preview(&self, app: &str) -> bool {
        self.preview_apps.contains(&app.to_lowercase())
    }

    /// Preview applications, sorted so the catalog and the manifest are stable.
    pub fn get_preview_apps(&self) -> Vec<String> {
        let mut preview: Vec<String> = self.preview_apps.iter().cloned().collect();
        preview.sort();
        preview
    }

    /// Get the product name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get the default theme
    pub fn get_theme(&self) -> &str {
        &self.theme
    }

    /// Replace "General Bots" with the product name in a string
    pub fn replace_branding(&self, text: &str) -> String {
        text.replace("General Bots", &self.name)
            .replace("general bots", &self.name.to_lowercase())
            .replace("GENERAL BOTS", &self.name.to_uppercase())
    }

    /// Get copyright text with year substitution
    pub fn get_copyright(&self) -> String {
        let year = chrono::Utc::now().format("%Y").to_string();
        let template = self
            .copyright
            .as_deref()
            .unwrap_or("© {year} {name}. All rights reserved.");

        template
            .replace("{year}", &year)
            .replace("{name}", &self.name)
    }

    /// Get all enabled apps as a vector
    pub fn get_enabled_apps(&self) -> Vec<String> {
        self.apps.iter().cloned().collect()
    }

    /// Reload configuration from file
    pub fn reload() -> Result<(), ProductConfigError> {
        let new_config = Self::load()?;
        let mut config = PRODUCT_CONFIG
            .write()
            .map_err(|_| ProductConfigError::LockError)?;
        *config = new_config;
        info!("Product configuration reloaded");
        Ok(())
    }
}
