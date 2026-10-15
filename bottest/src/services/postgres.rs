use super::{check_tcp_port, ensure_dir, wait_for, HEALTH_CHECK_INTERVAL, HEALTH_CHECK_TIMEOUT};
use anyhow::{Context, Result};
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

pub struct PostgresService {
    port: u16,
    data_dir: PathBuf,
    bin_dir: PathBuf,
    lib_dir: Option<PathBuf>,
    process: Option<Child>,
    connection_string: String,
    database_name: String,
    username: String,
    password: String,
}

impl PostgresService {
    pub const DEFAULT_DATABASE: &'static str = "bottest";

    pub const DEFAULT_USERNAME: &'static str = "bottest";

    pub const DEFAULT_PASSWORD: &'static str = "bottest";

    fn find_postgres_installation() -> Result<(PathBuf, Option<PathBuf>)> {
        if let Ok(stack_path) = std::env::var("BOTSERVER_STACK_PATH") {
            let bin_dir = PathBuf::from(&stack_path).join("bin/tables/bin");
            let lib_dir = PathBuf::from(&stack_path).join("bin/tables/lib");
            if bin_dir.join("postgres").exists() || bin_dir.join("initdb").exists() {
                log::info!(
                    "Using PostgreSQL from BOTSERVER_STACK_PATH: {}",
                    bin_dir.display()
                );
                return Ok((bin_dir, Some(lib_dir)));
            }
        }

        let cwd = std::env::current_dir().unwrap_or_default();
        let relative_paths = [
            "../botserver/botserver-stack/bin/tables/bin",
            "botserver/botserver-stack/bin/tables/bin",
            "botserver-stack/bin/tables/bin",
        ];

        for rel_path in &relative_paths {
            let bin_dir = cwd.join(rel_path);
            if bin_dir.join("postgres").exists() || bin_dir.join("initdb").exists() {
                let lib_dir = bin_dir.parent().unwrap().join("lib");
                log::info!(
                    "Using PostgreSQL from botserver-stack: {}",
                    bin_dir.display()
                );
                return Ok((
                    bin_dir,
                    if lib_dir.exists() {
                        Some(lib_dir)
                    } else {
                        None
                    },
                ));
            }
        }

        let system_paths = [
            "/usr/lib/postgresql/17/bin",
            "/usr/lib/postgresql/16/bin",
            "/usr/lib/postgresql/15/bin",
            "/usr/lib/postgresql/14/bin",
            "/usr/bin",
            "/usr/local/bin",
            "/opt/homebrew/bin",
            "/opt/homebrew/opt/postgresql@17/bin",
            "/opt/homebrew/opt/postgresql@16/bin",
            "/opt/homebrew/opt/postgresql@15/bin",
        ];

        for path in &system_paths {
            let bin_dir = PathBuf::from(path);
            if bin_dir.join("postgres").exists() || bin_dir.join("initdb").exists() {
                log::info!("Using system PostgreSQL from: {}", bin_dir.display());
                return Ok((bin_dir, None));
            }
        }

        if let Ok(initdb_path) = which::which("initdb") {
            if let Some(bin_dir) = initdb_path.parent() {
                log::info!("Using PostgreSQL from PATH: {}", bin_dir.display());
                return Ok((bin_dir.to_path_buf(), None));
            }
        }

        anyhow::bail!(
            "PostgreSQL not found. Install PostgreSQL or set BOTSERVER_STACK_PATH env var"
        )
    }

    pub async fn start(port: u16, data_dir: &str) -> Result<Self> {
        let (bin_dir, lib_dir) = Self::find_postgres_installation()?;

        let data_path = PathBuf::from(data_dir).join("postgres");
        ensure_dir(&data_path)?;

        let mut service = Self {
            port,
            data_dir: data_path.clone(),
            bin_dir,
            lib_dir,
            process: None,
            connection_string: String::new(),
            database_name: Self::DEFAULT_DATABASE.to_string(),
            username: Self::DEFAULT_USERNAME.to_string(),
            password: Self::DEFAULT_PASSWORD.to_string(),
        };

        service.connection_string = service.build_connection_string();

        if !data_path.join("PG_VERSION").exists() {
            service.init_db()?;
        }

        service.start_server()?;

        service.wait_ready().await?;

        service.setup_test_database()?;

        Ok(service)
    }

    fn get_binary(&self, name: &str) -> PathBuf {
        self.bin_dir.join(name)
    }

    fn build_command(&self, binary_name: &str) -> Command {
        let binary = self.get_binary(binary_name);
        let mut cmd = Command::new(&binary);
        if let Some(ref lib_dir) = self.lib_dir {
            cmd.env("LD_LIBRARY_PATH", lib_dir);
        }
        cmd
    }

    fn init_db(&self) -> Result<()> {
        log::info!(
            "Initializing PostgreSQL data directory at {}",
            self.data_dir.display()
        );

        let output = self
            .build_command("initdb")
            .args([
                "-D",
                self.data_dir.to_str().unwrap(),
                "-U",
                "postgres",
                "-A",
                "trust",
                "-E",
                "UTF8",
                "--no-locale",
            ])
            .output()
            .context("Failed to run initdb")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("initdb failed: {stderr}");
        }

        self.configure_for_testing()?;

        Ok(())
    }

    fn configure_for_testing(&self) -> Result<()> {
        let config_path = self.data_dir.join("postgresql.conf");

        let config = format!(
            r"
# Test configuration - optimized for speed, not durability
listen_addresses = '127.0.0.1'
port = {}
max_connections = 50
shared_buffers = 128MB
work_mem = 16MB
maintenance_work_mem = 64MB
wal_level = minimal
fsync = off
synchronous_commit = off
full_page_writes = off
checkpoint_timeout = 30min
max_wal_senders = 0
logging_collector = off
log_statement = 'none'
log_duration = off
unix_socket_directories = ''
",
            self.port
        );

        std::fs::write(&config_path, config)?;
        Ok(())
    }

    fn start_server(&mut self) -> Result<()> {
        log::info!("Starting PostgreSQL on port {}", self.port);

        let log_path = self.data_dir.join("postgres.log");
        let log_file = std::fs::File::create(&log_path)
            .context(format!("Failed to create log file {}", log_path.display()))?;
        let stderr_file = log_file.try_clone()?;

        log::debug!("PostgreSQL log file: {}", log_path.display());

        let mut cmd = self.build_command("postgres");
        let child = cmd
            .args(["-D", self.data_dir.to_str().unwrap()])
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .context("Failed to start PostgreSQL")?;

        self.process = Some(child);
        Ok(())
    }

    async fn wait_ready(&self) -> Result<()> {
        log::info!("Waiting for PostgreSQL to be ready...");

        let result = wait_for(HEALTH_CHECK_TIMEOUT, HEALTH_CHECK_INTERVAL, || async {
            check_tcp_port("127.0.0.1", self.port).await
        })
        .await;

        if let Err(e) = result {
            let log_path = self.data_dir.join("postgres.log");
            if log_path.exists() {
                if let Ok(log_content) = std::fs::read_to_string(&log_path) {
                    log::error!("PostgreSQL log:\n{log_content}");
                }
            }
            return Err(e).context("PostgreSQL failed to start in time");
        }

        for _ in 0..30 {
            let status = self
                .build_command("pg_isready")
                .args(["-h", "127.0.0.1", "-p", &self.port.to_string()])
                .status();

            if status.map(|s| s.success()).unwrap_or(false) {
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    fn setup_test_database(&self) -> Result<()> {
        log::info!("Setting up test database '{}'", self.database_name);

        let _ = self
            .build_command("psql")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-U",
                "postgres",
                "-c",
                &format!(
                    "CREATE USER {} WITH PASSWORD '{}' SUPERUSER",
                    self.username, self.password
                ),
            ])
            .output();

        let _ = self
            .build_command("psql")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-U",
                "postgres",
                "-c",
                &format!(
                    "CREATE DATABASE {} OWNER {}",
                    self.database_name, self.username
                ),
            ])
            .output();

        Ok(())
    }

    pub fn run_migrations(&self) -> Result<()> {
        log::info!("Running database migrations...");

        if let Ok(diesel) = which::which("diesel") {
            let crate_dir = std::env::current_dir().unwrap_or_default();
            let candidate = crate_dir.join("../botserver/migrations");
            let migration_dir = if candidate.is_dir() {
                candidate
            } else {
                crate_dir.parent().map(|p| p.join("botserver/migrations"))
                    .filter(|p| p.is_dir())
                    .unwrap_or_else(|| crate_dir.join("migrations"))
            };
            let status = Command::new(diesel)
                .args([
                    "migration",
                    "run",
                    "--migration-dir",
                    migration_dir.to_str().unwrap_or("migrations"),
                    "--database-url",
                    &self.connection_string,
                ])
                .status();

            if status.map(|s| s.success()).unwrap_or(false) {
                return Ok(());
            }
        }

        log::warn!("diesel CLI not available, skipping migrations");
        Ok(())
    }

    pub fn create_database(&self, name: &str) -> Result<()> {
        let output = self
            .build_command("psql")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-U",
                &self.username,
                "-c",
                &format!("CREATE DATABASE {name}"),
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.contains("already exists") {
                anyhow::bail!("Failed to create database: {stderr}");
            }
        }

        Ok(())
    }

    pub fn execute(&self, sql: &str) -> Result<()> {
        let output = self
            .build_command("psql")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-U",
                &self.username,
                "-d",
                &self.database_name,
                "-c",
                sql,
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SQL execution failed: {stderr}");
        }

        Ok(())
    }

    pub fn query(&self, sql: &str) -> Result<String> {
        let output = self
            .build_command("psql")
            .args([
                "-h",
                "127.0.0.1",
                "-p",
                &self.port.to_string(),
                "-U",
                &self.username,
                "-d",
                &self.database_name,
                "-t",
                "-A",
                "-c",
                sql,
            ])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("SQL query failed: {stderr}");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

mod part2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_string_format() {
        let service = PostgresService {
            port: 5432,
            data_dir: PathBuf::from("/tmp/test"),
            bin_dir: PathBuf::from("/usr/bin"),
            lib_dir: None,
            process: None,
            connection_string: String::new(),
            database_name: "testdb".to_string(),
            username: "testuser".to_string(),
            password: "testpass".to_string(),
        };

        let conn_str = service.build_connection_string();
        assert_eq!(
            conn_str,
            "postgres://testuser:testpass@127.0.0.1:5432/testdb"
        );
    }
}
