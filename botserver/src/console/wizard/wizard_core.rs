use crossterm::{
    cursor,
    execute,
    terminal::{self, ClearType},
};
use serde::{Deserialize, Serialize};
use std::fmt::Write as FmtWrite;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardConfig {
    pub llm_provider: LlmProvider,

    pub llm_api_key: Option<String>,

    pub local_model_path: Option<String>,

    pub components: Vec<ComponentChoice>,

    pub admin: AdminConfig,

    pub organization: OrgConfig,

    pub template: Option<String>,

    pub install_mode: InstallMode,

    pub data_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LlmProvider {
    Claude,
    OpenAI,
    Gemini,
    Local,
    None,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "Claude (Anthropic) - Best for complex reasoning"),
            Self::OpenAI => write!(f, "GPT-4 (OpenAI) - General purpose"),
            Self::Gemini => write!(f, "Gemini (Google) - Google integration"),
            Self::Local => write!(f, "Local (Llama/Mistral) - Privacy focused"),
            Self::None => write!(f, "None - Configure later"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComponentChoice {
    Drive,
    Email,
    Meet,
    Tables,
    Cache,
    VectorDb,
    Proxy,
    Directory,
    BotModels,
}

impl std::fmt::Display for ComponentChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Drive => write!(f, "Drive (MinIO) - File storage"),
            Self::Email => write!(f, "Email Server - Send/receive emails"),
            Self::Meet => write!(f, "Meet (LiveKit) - Video meetings"),
            Self::Tables => write!(f, "Database (PostgreSQL) - Required"),
            Self::Cache => write!(f, "Cache (Redis) - Sessions & queues"),
            Self::VectorDb => write!(f, "Vector DB - AI embeddings"),
            Self::Proxy => write!(f, "Proxy (Caddy) - HTTPS & routing"),
            Self::Directory => write!(f, "Directory - Users & SSO"),
            Self::BotModels => write!(f, "BotModels - Local AI models"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdminConfig {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OrgConfig {
    pub name: String,
    pub slug: String,
    pub domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallMode {
    Development,
    Production,
    Container,
}

impl Default for WizardConfig {
    fn default() -> Self {
        Self {
            llm_provider: LlmProvider::None,
            llm_api_key: None,
            local_model_path: None,
            components: vec![
                ComponentChoice::Tables,
                ComponentChoice::Cache,
                ComponentChoice::Drive,
            ],
            admin: AdminConfig::default(),
            organization: OrgConfig::default(),
            template: None,
            install_mode: InstallMode::Development,
            data_dir: PathBuf::from(crate::core::shared::utils::get_stack_path()),
        }
    }
}

#[derive(Debug)]
pub struct StartupWizard {
    pub(super) config: WizardConfig,
    pub(super) current_step: usize,
    pub(super) total_steps: usize,
}

impl Default for StartupWizard {
    fn default() -> Self {
        Self {
            config: WizardConfig::default(),
            current_step: 0,
            total_steps: 7,
        }
    }
}

impl StartupWizard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self) -> io::Result<WizardConfig> {
        terminal::enable_raw_mode()?;
        let mut stdout = io::stdout();

        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        self.show_welcome(&mut stdout)?;
        self.wait_for_enter()?;

        self.current_step = 1;
        self.step_install_mode(&mut stdout)?;

        self.current_step = 2;
        self.step_llm_provider(&mut stdout)?;

        self.current_step = 3;
        self.step_components(&mut stdout)?;

        self.current_step = 4;
        self.step_organization(&mut stdout)?;

        self.current_step = 5;
        self.step_admin_user(&mut stdout)?;

        self.current_step = 6;
        self.step_template(&mut stdout)?;

        self.current_step = 7;
        self.step_summary(&mut stdout)?;

        terminal::disable_raw_mode()?;
        Ok(self.config.clone())
    }
}

pub fn save_wizard_config(config: &WizardConfig, path: &str) -> io::Result<()> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn load_wizard_config(path: &str) -> io::Result<WizardConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: WizardConfig =
        toml::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(config)
}

pub fn should_run_wizard() -> bool {
    !std::path::Path::new(&crate::core::shared::utils::get_stack_path()).exists()
        && !std::path::Path::new("/opt/gbo").exists()
}

pub fn apply_wizard_config(config: &WizardConfig) -> io::Result<()> {
    use std::fs;

    fs::create_dir_all(&config.data_dir)?;

    let subdirs = ["bots", "logs", "cache", "uploads", "config"];
    for subdir in &subdirs {
        fs::create_dir_all(config.data_dir.join(subdir))?;
    }

    save_wizard_config(
        config,
        &config.data_dir.join("config/wizard.toml").to_string_lossy(),
    )?;

    let mut env_content = String::new();
    let _ = writeln!(
        env_content,
        "# Generated by General Bots Setup Wizard\n"
    );
    let _ = writeln!(env_content, "INSTALL_MODE={:?}", config.install_mode);
    let _ = writeln!(env_content, "ORG_NAME={}", config.organization.name);
    let _ = writeln!(env_content, "ORG_SLUG={}", config.organization.slug);

    if let Some(domain) = &config.organization.domain {
        let _ = writeln!(env_content, "DOMAIN={domain}");
    }

    match &config.llm_provider {
        LlmProvider::Claude => env_content.push_str("LLM_PROVIDER=anthropic\n"),
        LlmProvider::OpenAI => env_content.push_str("LLM_PROVIDER=openai\n"),
        LlmProvider::Gemini => env_content.push_str("LLM_PROVIDER=google\n"),
        LlmProvider::Local => env_content.push_str("LLM_PROVIDER=local\n"),
        LlmProvider::None => {}
    }

    if let Some(api_key) = &config.llm_api_key {
        let _ = writeln!(env_content, "LLM_API_KEY={api_key}");
    }

    if let Some(model_path) = &config.local_model_path {
        let _ = writeln!(env_content, "LOCAL_MODEL_PATH={model_path}");
    }

    fs::write(config.data_dir.join(".env"), env_content)?;

    println!("\n Configuration applied successfully!");
    println!("   Data directory: {}", config.data_dir.display());
    println!("\n   Next steps:");
    println!("   1. Run: botserver start");
    println!("   2. Open: ");
    println!("   3. Login with: {}", config.admin.username);

    Ok(())
}
