use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use std::io::{self, Write};

use super::wizard_core::{ComponentChoice, InstallMode, LlmProvider, StartupWizard};

impl StartupWizard {
    pub(super) fn step_install_mode(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Installation Mode")?;

        let options = vec![
            (
                "Development",
                "Local development with hot reload",
                InstallMode::Development,
            ),
            (
                "Production",
                "Optimized for production servers",
                InstallMode::Production,
            ),
            (
                "Container",
                "Docker/LXC container deployment",
                InstallMode::Container,
            ),
        ];

        let selected = self.select_option(stdout, &options, 0)?;
        self.config.install_mode = options[selected].2.clone();

        Ok(())
    }

    pub(super) fn step_llm_provider(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "AI/LLM Provider")?;

        execute!(
            stdout,
            cursor::MoveTo(2, 7),
            Print("Select your preferred AI provider:"),
            cursor::MoveTo(2, 8),
            SetForegroundColor(Color::DarkGrey),
            Print("(You can use multiple providers later)"),
            ResetColor
        )?;

        let options = vec![
            (
                "Claude (Anthropic)",
                "Best reasoning, 200K context - Recommended",
                LlmProvider::Claude,
            ),
            (
                "GPT-4 (OpenAI)",
                "Widely compatible, good all-around",
                LlmProvider::OpenAI,
            ),
            (
                "Gemini (Google)",
                "Great for Google Workspace integration",
                LlmProvider::Gemini,
            ),
            (
                "Local Models",
                "Llama, Mistral - Full privacy, no API costs",
                LlmProvider::Local,
            ),
            (
                "Skip for now",
                "Configure AI providers later",
                LlmProvider::None,
            ),
        ];

        let selected = self.select_option(stdout, &options, 0)?;
        self.config.llm_provider = options[selected].2.clone();

        if self.config.llm_provider != LlmProvider::Local
            && self.config.llm_provider != LlmProvider::None
        {
            terminal::disable_raw_mode()?;
            execute!(
                stdout,
                cursor::MoveTo(2, 20),
                Print("Enter API key (or press Enter to skip): ")
            )?;
            stdout.flush()?;

            let mut api_key = String::new();
            io::stdin().read_line(&mut api_key)?;
            let api_key = api_key.trim().to_string();

            if !api_key.is_empty() {
                self.config.llm_api_key = Some(api_key);
            }
            terminal::enable_raw_mode()?;
        }

        if self.config.llm_provider == LlmProvider::Local {
            terminal::disable_raw_mode()?;
            execute!(
                stdout,
                cursor::MoveTo(2, 20),
                Print("Enter model path (default: ./models/llama-3.1-8b): ")
            )?;
            stdout.flush()?;

            let mut model_path = String::new();
            io::stdin().read_line(&mut model_path)?;
            let model_path = model_path.trim().to_string();

            self.config.local_model_path = Some(if model_path.is_empty() {
                "./models/llama-3.1-8b".to_string()
            } else {
                model_path
            });
            terminal::enable_raw_mode()?;
        }

        Ok(())
    }

    pub(super) fn step_components(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Components to Install")?;

        execute!(
            stdout,
            cursor::MoveTo(2, 7),
            Print("Select components to install (Space to toggle, Enter to confirm):"),
            cursor::MoveTo(2, 8),
            SetForegroundColor(Color::DarkGrey),
            Print("PostgreSQL and Redis are required and pre-selected"),
            ResetColor
        )?;

        let components = vec![
            (ComponentChoice::Tables, true, false),
            (ComponentChoice::Cache, true, false),
            (ComponentChoice::Drive, true, true),
            (ComponentChoice::VectorDb, true, true),
            (ComponentChoice::Email, false, true),
            (ComponentChoice::Meet, false, true),
            (ComponentChoice::Proxy, true, true),
            (ComponentChoice::Directory, false, true),
            (ComponentChoice::BotModels, false, true),
        ];

        let selected = self.multi_select(stdout, &components)?;
        self.config.components = selected;

        Ok(())
    }

    pub(super) fn step_organization(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Organization Setup")?;

        terminal::disable_raw_mode()?;

        execute!(stdout, cursor::MoveTo(2, 7), Print("Organization name: "))?;
        stdout.flush()?;

        let mut org_name = String::new();
        io::stdin().read_line(&mut org_name)?;
        self.config.organization.name = org_name.trim().to_string();

        self.config.organization.slug = self
            .config
            .organization
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect();

        execute!(
            stdout,
            cursor::MoveTo(2, 9),
            Print(format!("Slug ({}): ", self.config.organization.slug))
        )?;
        stdout.flush()?;

        let mut slug = String::new();
        io::stdin().read_line(&mut slug)?;
        let slug = slug.trim();
        if !slug.is_empty() {
            self.config.organization.slug = slug.to_string();
        }

        execute!(
            stdout,
            cursor::MoveTo(2, 11),
            Print("Domain (optional, e.g., example.com): ")
        )?;
        stdout.flush()?;

        let mut domain = String::new();
        io::stdin().read_line(&mut domain)?;
        let domain = domain.trim();
        if !domain.is_empty() {
            self.config.organization.domain = Some(domain.to_string());
        }

        terminal::enable_raw_mode()?;
        Ok(())
    }

    pub(super) fn step_admin_user(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Admin User")?;

        terminal::disable_raw_mode()?;

        execute!(stdout, cursor::MoveTo(2, 7), Print("Admin username: "))?;
        stdout.flush()?;

        let mut username = String::new();
        io::stdin().read_line(&mut username)?;
        self.config.admin.username = username.trim().to_string();

        execute!(stdout, cursor::MoveTo(2, 9), Print("Admin email: "))?;
        stdout.flush()?;

        let mut email = String::new();
        io::stdin().read_line(&mut email)?;
        self.config.admin.email = email.trim().to_string();

        execute!(stdout, cursor::MoveTo(2, 11), Print("Admin display name: "))?;
        stdout.flush()?;

        let mut display_name = String::new();
        io::stdin().read_line(&mut display_name)?;
        self.config.admin.display_name = display_name.trim().to_string();

        execute!(stdout, cursor::MoveTo(2, 13), Print("Admin password: "))?;
        stdout.flush()?;

        let mut password = String::new();
        io::stdin().read_line(&mut password)?;
        self.config.admin.password = password.trim().to_string();

        terminal::enable_raw_mode()?;
        Ok(())
    }

    pub(super) fn step_template(&mut self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Bot Template")?;

        execute!(
            stdout,
            cursor::MoveTo(2, 7),
            Print("Select a template for your first bot:"),
        )?;

        let options = [("default", "Basic bot with weather, email, and tools"),
            ("crm", "Customer relationship management"),
            ("edu", "Educational/course management"),
            ("store", "E-commerce bot"),
            ("hr", "Human resources assistant"),
            ("healthcare", "Healthcare appointment scheduling"),
            ("none", "Start from scratch")];

        let templates: Vec<(&str, &str, Option<String>)> = options
            .iter()
            .map(|(name, desc)| {
                (
                    *name,
                    *desc,
                    if *name == "none" {
                        None
                    } else {
                        Some((*name).to_string())
                    },
                )
            })
            .collect();

        let selected = self.select_option(stdout, &templates, 0)?;
        self.config.template.clone_from(&templates[selected].2);

        Ok(())
    }

    pub(super) fn step_summary(&self, stdout: &mut io::Stdout) -> io::Result<()> {
        self.show_step_header(stdout, "Configuration Summary")?;

        let mode = match self.config.install_mode {
            InstallMode::Development => "Development",
            InstallMode::Production => "Production",
            InstallMode::Container => "Container",
        };

        let llm = match &self.config.llm_provider {
            LlmProvider::Claude => "Claude (Anthropic)",
            LlmProvider::OpenAI => "GPT-4 (OpenAI)",
            LlmProvider::Gemini => "Gemini (Google)",
            LlmProvider::Local => "Local Models",
            LlmProvider::None => "Not configured",
        };

        execute!(
            stdout,
            cursor::MoveTo(2, 7),
            SetForegroundColor(Color::Cyan),
            Print("═══════════════════════════════════════════════════"),
            ResetColor,
            cursor::MoveTo(2, 9),
            Print(format!("  Installation Mode:  {}", mode)),
            cursor::MoveTo(2, 10),
            Print(format!("  LLM Provider:       {}", llm)),
            cursor::MoveTo(2, 11),
            Print(format!(
                "  Organization:       {}",
                self.config.organization.name
            )),
            cursor::MoveTo(2, 12),
            Print(format!(
                "  Admin User:         {}",
                self.config.admin.username
            )),
            cursor::MoveTo(2, 13),
            Print(format!(
                "  Template:           {}",
                self.config.template.as_deref().unwrap_or("None")
            )),
            cursor::MoveTo(2, 14),
            Print(format!(
                "  Components:         {}",
                self.config.components.len()
            )),
            cursor::MoveTo(2, 16),
            SetForegroundColor(Color::Cyan),
            Print("═══════════════════════════════════════════════════"),
            ResetColor,
            cursor::MoveTo(2, 18),
            Print("Components to install:"),
        )?;

        for (i, component) in self.config.components.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(4, 19 + i as u16),
                SetForegroundColor(Color::Green),
                Print("* "),
                ResetColor,
                Print(format!("{}", component))
            )?;
        }

        let last_line = 19 + self.config.components.len() as u16 + 2;
        execute!(
            stdout,
            cursor::MoveTo(2, last_line),
            SetForegroundColor(Color::Yellow),
            Print("Press ENTER to apply configuration, or ESC to cancel"),
            ResetColor
        )?;

        stdout.flush()?;

        loop {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Enter => break,
                    KeyCode::Esc => {
                        return Err(io::Error::new(
                            io::ErrorKind::Interrupted,
                            "Wizard cancelled",
                        ));
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }
}
