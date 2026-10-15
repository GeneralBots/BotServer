//! Split from `harness.rs` — 326 lines.

use super::*;

impl Insertable for Bot {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Bool, Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO bots (id, name, description, kb_enabled, llm_enabled, llm_model, active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (id) DO UPDATE SET name = $2, description = $3, kb_enabled = $4, llm_enabled = $5, llm_model = $6, active = $7, updated_at = $9",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<Text, _>(&self.name)
        .bind::<Nullable<Text>, _>(&self.description)
        .bind::<Bool, _>(self.kb_enabled)
        .bind::<Bool, _>(self.llm_enabled)
        .bind::<Nullable<Text>, _>(&self.llm_model)
        .bind::<Bool, _>(self.active)
        .bind::<Timestamptz, _>(self.created_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Session {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO sessions (id, bot_id, customer_id, channel, state, started_at, updated_at, ended_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET state = $5, updated_at = $7, ended_at = $8",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.bot_id)
        .bind::<DieselUuid, _>(self.customer_id)
        .bind::<Text, _>(format!("{:?}", self.channel).to_lowercase())
        .bind::<Text, _>(format!("{:?}", self.state).to_lowercase())
        .bind::<Timestamptz, _>(self.started_at)
        .bind::<Timestamptz, _>(self.updated_at)
        .bind::<Nullable<Timestamptz>, _>(self.ended_at)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for Message {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO messages (id, session_id, direction, content, content_type, timestamp)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.session_id)
        .bind::<Text, _>(format!("{:?}", self.direction).to_lowercase())
        .bind::<Text, _>(&self.content)
        .bind::<Text, _>(format!("{:?}", self.content_type).to_lowercase())
        .bind::<Timestamptz, _>(self.timestamp)
        .execute(&mut conn)?;
        Ok(())
    }
}

impl Insertable for QueueEntry {
    fn insert(&self, pool: &DbPool) -> Result<()> {
        use diesel::prelude::*;
        use diesel::sql_query;
        use diesel::sql_types::{Nullable, Text, Timestamptz, Uuid as DieselUuid};

        let mut conn = pool.get()?;
        sql_query(
            "INSERT INTO queue_entries (id, customer_id, session_id, priority, status, entered_at, assigned_at, attendant_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET status = $5, assigned_at = $7, attendant_id = $8",
        )
        .bind::<DieselUuid, _>(self.id)
        .bind::<DieselUuid, _>(self.customer_id)
        .bind::<DieselUuid, _>(self.session_id)
        .bind::<Text, _>(format!("{:?}", self.priority).to_lowercase())
        .bind::<Text, _>(format!("{:?}", self.status).to_lowercase())
        .bind::<Timestamptz, _>(self.entered_at)
        .bind::<Nullable<Timestamptz>, _>(self.assigned_at)
        .bind::<Nullable<DieselUuid>, _>(self.attendant_id)
        .execute(&mut conn)?;
        Ok(())
    }
}

pub struct BotServerInstance {
    pub url: String,
    pub port: u16,
    pub stack_path: PathBuf,
    pub(crate) process: Option<std::process::Child>,
}

impl BotServerInstance {
    #[must_use]
    pub fn existing(url: &str) -> Self {
        let port = url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self {
            url: url.to_string(),
            port,
            stack_path: PathBuf::from("./botserver-stack"),
            process: None,
        }
    }

    pub async fn start_with_main_stack() -> Result<Self> {
        let port = 8080;
        let url = "https://localhost:9000".to_string();

        let botserver_bin = std::env::var("BOTSERVER_BIN")
            .unwrap_or_else(|_| "../botserver/target/debug/botserver".to_string());

        if !PathBuf::from(&botserver_bin).exists() {
            log::warn!("Botserver binary not found at: {botserver_bin}");
            anyhow::bail!(
                "Botserver binary not found at: {botserver_bin}. Run: cd ../botserver && cargo build"
            );
        }

        let botserver_bin_path =
            std::fs::canonicalize(&botserver_bin).unwrap_or_else(|_| PathBuf::from(&botserver_bin));
        let botserver_dir = botserver_bin_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::fs::canonicalize("../botserver")
                    .unwrap_or_else(|_| PathBuf::from("../botserver"))
            });

        let stack_path = botserver_dir.join("botserver-stack");

        if !stack_path.exists() {
            anyhow::bail!(
                "Main botserver-stack not found at {}.\n\
                 Run botserver once to initialize: cd ../botserver && cargo run",
                stack_path.display()
            );
        }

        log::info!(
            "Starting botserver with MAIN stack at {}",
            stack_path.display()
        );
        println!("🚀 Starting BotServer with main stack...");
        println!("   Stack: {}", stack_path.display());

        let process = std::process::Command::new(&botserver_bin_path)
            .current_dir(&botserver_dir)
            .arg("--noconsole")
            .env_remove("RUST_LOG")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .ok();

        if process.is_some() {
            let max_wait = 120;
            log::info!("Waiting for botserver to start (max {max_wait}s)...");

            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default();

            for i in 0..max_wait {
                if let Ok(resp) = client.get(format!("{url}/health")).send().await {
                    if resp.status().is_success() {
                        log::info!("Botserver ready on port {port}");
                        println!("   ✓ BotServer ready at {url}");
                        return Ok(Self {
                            url,
                            port,
                            stack_path,
                            process,
                        });
                    }
                }
                if i % 10 == 0 && i > 0 {
                    log::info!("Still waiting for botserver... ({i}s)");
                    println!("   ... waiting ({i}s)");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            log::warn!("Botserver did not respond in time");
            println!("   ⚠ Botserver may not be ready");
        }

        Ok(Self {
            url,
            port,
            stack_path,
            process,
        })
    }
}

pub struct BotUIInstance {
    pub url: String,
    pub port: u16,
    pub(crate) process: Option<std::process::Child>,
}

impl BotUIInstance {
    #[must_use]
    pub fn existing(url: &str) -> Self {
        let port = url
            .split(':')
            .next_back()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        Self {
            url: url.to_string(),
            port,
            process: None,
        }
    }
}

impl BotUIInstance {
    pub async fn start(botserver_url: &str) -> Result<Self> {
        let port = crate::ports::PortAllocator::allocate();
        let url = format!("http://127.0.0.1:{port}");

        let botui_bin = std::env::var("BOTUI_BIN")
            .unwrap_or_else(|_| "../botui/target/debug/botui".to_string());

        if !PathBuf::from(&botui_bin).exists() {
            log::warn!("BotUI binary not found at: {botui_bin}");
            return Ok(Self {
                url,
                port,
                process: None,
            });
        }

        let botui_bin_path =
            std::fs::canonicalize(&botui_bin).unwrap_or_else(|_| PathBuf::from(&botui_bin));
        let botui_dir = botui_bin_path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| {
                std::fs::canonicalize("../botui").unwrap_or_else(|_| PathBuf::from("../botui"))
            });

        log::info!("Starting botui from: {botui_bin} on port {port}");
        log::info!("  BOTUI_PORT={port}");
        log::info!("  BOTSERVER_URL={botserver_url}");
        log::info!("  Working directory: {}", botui_dir.display());

        let process = std::process::Command::new(&botui_bin_path)
            .current_dir(&botui_dir)
            .env("BOTUI_PORT", port.to_string())
            .env("BOTSERVER_URL", botserver_url)
            .env_remove("RUST_LOG")
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .ok();

        if process.is_some() {
            let max_wait = 30;
            log::info!("Waiting for botui to become ready... (max {max_wait}s)");
            for i in 0..max_wait {
                if let Ok(resp) = reqwest::get(&format!("{url}/health")).await {
                    if resp.status().is_success() {
                        log::info!("BotUI is ready on port {port}");
                        return Ok(Self { url, port, process });
                    }
                }
                if let Ok(resp) = reqwest::get(&url).await {
                    if resp.status().is_success() {
                        log::info!("BotUI is ready on port {port}");
                        return Ok(Self { url, port, process });
                    }
                }
                if i % 5 == 0 {
                    log::info!("Still waiting for botui... ({i}s)");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            log::warn!("BotUI did not respond in time");
        }

        Ok(Self {
            url,
            port,
            process: None,
        })
    }

    #[must_use]
    pub const fn is_running(&self) -> bool {
        self.process.is_some()
    }
}

impl Drop for BotUIInstance {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
