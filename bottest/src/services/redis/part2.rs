//! Split from `redis.rs` (#1504).

use super::*;

impl RedisService {
    pub async fn start(port: u16, data_dir: &str) -> Result<Self> {
        let data_path = PathBuf::from(data_dir).join("redis");
        ensure_dir(&data_path)?;

        let mut service = Self {
            port,
            data_dir: data_path,
            process: None,
            password: None,
        };

        service.start_server().await?;
        service.wait_ready().await?;

        Ok(service)
    }

    pub async fn start_with_password(port: u16, data_dir: &str, password: &str) -> Result<Self> {
        let data_path = PathBuf::from(data_dir).join("redis");
        ensure_dir(&data_path)?;

        let mut service = Self {
            port,
            data_dir: data_path,
            process: None,
            password: Some(password.to_string()),
        };

        service.start_server().await?;
        service.wait_ready().await?;

        Ok(service)
    }

    pub(crate) async fn start_server(&mut self) -> Result<()> {
        tokio::task::yield_now().await;
        log::info!("Starting Redis on port {}", self.port);

        let redis = Self::find_binary()?;

        let mut args = vec![
            "--port".to_string(),
            self.port.to_string(),
            "--bind".to_string(),
            "127.0.0.1".to_string(),
            "--dir".to_string(),
            self.data_dir.to_str().unwrap().to_string(),
            "--daemonize".to_string(),
            "no".to_string(),
            "--save".to_string(),
            String::new(),
            "--appendonly".to_string(),
            "no".to_string(),
            "--maxmemory".to_string(),
            "64mb".to_string(),
            "--maxmemory-policy".to_string(),
            "allkeys-lru".to_string(),
        ];

        if let Some(ref password) = self.password {
            args.push("--requirepass".to_string());
            args.push(password.clone());
        }

        let child = Command::new(&redis)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Redis")?;

        self.process = Some(child);
        Ok(())
    }

    pub(crate) async fn wait_ready(&self) -> Result<()> {
        log::info!("Waiting for Redis to be ready...");

        wait_for(HEALTH_CHECK_TIMEOUT, HEALTH_CHECK_INTERVAL, || async {
            check_tcp_port("127.0.0.1", self.port).await
        })
        .await
        .context("Redis failed to start in time")?;

        if let Ok(redis_cli) = Self::find_cli_binary() {
            for _ in 0..30 {
                let mut cmd = Command::new(&redis_cli);
                cmd.args(["-h", "127.0.0.1", "-p", &self.port.to_string()]);

                if let Some(ref password) = self.password {
                    cmd.args(["-a", password]);
                }

                cmd.arg("PING");

                if let Ok(output) = cmd.output() {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        if stdout.trim() == "PONG" {
                            return Ok(());
                        }
                    }
                }
                sleep(Duration::from_millis(100)).await;
            }
        }

        Ok(())
    }

    pub async fn execute(&self, args: &[&str]) -> Result<String> {
        tokio::task::yield_now().await;
        let redis_cli = Self::find_cli_binary()?;

        let mut cmd = Command::new(&redis_cli);
        cmd.args(["-h", "127.0.0.1", "-p", &self.port.to_string()]);

        if let Some(ref password) = self.password {
            cmd.args(["-a", password]);
        }

        cmd.args(args);

        let output = cmd.output().context("Failed to execute Redis command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Redis command failed: {stderr}");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn set(&self, key: &str, value: &str) -> Result<()> {
        self.execute(&["SET", key, value]).await?;
        Ok(())
    }

    pub async fn setex(&self, key: &str, seconds: u64, value: &str) -> Result<()> {
        self.execute(&["SETEX", key, &seconds.to_string(), value])
            .await?;
        Ok(())
    }

    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let result = self.execute(&["GET", key]).await?;
        if result.is_empty() || result == "(nil)" {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub async fn del(&self, key: &str) -> Result<()> {
        self.execute(&["DEL", key]).await?;
        Ok(())
    }

    pub async fn exists(&self, key: &str) -> Result<bool> {
        let result = self.execute(&["EXISTS", key]).await?;
        Ok(result == "1" || result == "(integer) 1")
    }

    pub async fn keys(&self, pattern: &str) -> Result<Vec<String>> {
        let result = self.execute(&["KEYS", pattern]).await?;
        if result.is_empty() || result == "(empty list or set)" {
            Ok(Vec::new())
        } else {
            Ok(result
                .lines()
                .map(std::string::ToString::to_string)
                .collect())
        }
    }

    pub async fn flushall(&self) -> Result<()> {
        self.execute(&["FLUSHALL"]).await?;
        Ok(())
    }

    pub async fn publish(&self, channel: &str, message: &str) -> Result<i64> {
        let result = self.execute(&["PUBLISH", channel, message]).await?;
        let count = result.replace("(integer) ", "").parse::<i64>().unwrap_or(0);
        Ok(count)
    }

    pub async fn lpush(&self, key: &str, value: &str) -> Result<()> {
        self.execute(&["LPUSH", key, value]).await?;
        Ok(())
    }

    pub async fn rpush(&self, key: &str, value: &str) -> Result<()> {
        self.execute(&["RPUSH", key, value]).await?;
        Ok(())
    }

    pub async fn lpop(&self, key: &str) -> Result<Option<String>> {
        let result = self.execute(&["LPOP", key]).await?;
        if result.is_empty() || result == "(nil)" {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub async fn rpop(&self, key: &str) -> Result<Option<String>> {
        let result = self.execute(&["RPOP", key]).await?;
        if result.is_empty() || result == "(nil)" {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub async fn llen(&self, key: &str) -> Result<i64> {
        let result = self.execute(&["LLEN", key]).await?;
        let len = result.replace("(integer) ", "").parse::<i64>().unwrap_or(0);
        Ok(len)
    }

    pub async fn hset(&self, key: &str, field: &str, value: &str) -> Result<()> {
        self.execute(&["HSET", key, field, value]).await?;
        Ok(())
    }

    pub async fn hget(&self, key: &str, field: &str) -> Result<Option<String>> {
        let result = self.execute(&["HGET", key, field]).await?;
        if result.is_empty() || result == "(nil)" {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    pub async fn hgetall(&self, key: &str) -> Result<Vec<(String, String)>> {
        let result = self.execute(&["HGETALL", key]).await?;
        if result.is_empty() || result == "(empty list or set)" {
            return Ok(Vec::new());
        }

        let lines: Vec<&str> = result.lines().collect();
        let mut pairs = Vec::new();

        for chunk in lines.chunks(2) {
            if chunk.len() == 2 {
                pairs.push((chunk[0].to_string(), chunk[1].to_string()));
            }
        }

        Ok(pairs)
    }

    pub async fn incr(&self, key: &str) -> Result<i64> {
        let result = self.execute(&["INCR", key]).await?;
        let val = result.replace("(integer) ", "").parse::<i64>().unwrap_or(0);
        Ok(val)
    }

    pub async fn decr(&self, key: &str) -> Result<i64> {
        let result = self.execute(&["DECR", key]).await?;
        let val = result.replace("(integer) ", "").parse::<i64>().unwrap_or(0);
        Ok(val)
    }

    #[must_use]
    pub fn connection_string(&self) -> String {
        match &self.password {
            Some(pw) => format!("redis://:{}@127.0.0.1:{}", pw, self.port),
            None => format!("redis://127.0.0.1:{}", self.port),
        }
    }

    #[must_use]
    pub fn url(&self) -> String {
        self.connection_string()
    }

    #[must_use]
    pub const fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub const fn host_port(&self) -> (&str, u16) {
        ("127.0.0.1", self.port)
    }

    pub(crate) fn find_binary() -> Result<PathBuf> {
        let common_paths = [
            "/usr/bin/redis-server",
            "/usr/local/bin/redis-server",
            "/opt/homebrew/bin/redis-server",
            "/opt/redis/redis-server",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        which::which("redis-server")
            .context("redis-server binary not found in PATH or common locations")
    }

    pub(crate) fn find_cli_binary() -> Result<PathBuf> {
        let common_paths = [
            "/usr/bin/redis-cli",
            "/usr/local/bin/redis-cli",
            "/opt/homebrew/bin/redis-cli",
            "/opt/redis/redis-cli",
        ];

        for path in common_paths {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }

        which::which("redis-cli").context("redis-cli binary not found")
    }

    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.process {
            log::info!("Stopping Redis...");

            if let Ok(redis_cli) = Self::find_cli_binary() {
                let mut cmd = Command::new(&redis_cli);
                cmd.args(["-h", "127.0.0.1", "-p", &self.port.to_string()]);

                if let Some(ref password) = self.password {
                    cmd.args(["-a", password]);
                }

                cmd.arg("SHUTDOWN");
                cmd.arg("NOSAVE");

                let _ = cmd.output();
            }

            for _ in 0..30 {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        self.process = None;
                        return Ok(());
                    }
                    Ok(None) => sleep(Duration::from_millis(100)).await,
                    Err(_) => break,
                }
            }

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            for _ in 0..20 {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        self.process = None;
                        return Ok(());
                    }
                    Ok(None) => sleep(Duration::from_millis(100)).await,
                    Err(_) => break,
                }
            }

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
            self.process = None;
        }

        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        if self.data_dir.exists() {
            std::fs::remove_dir_all(&self.data_dir)?;
        }
        Ok(())
    }
}

impl Drop for RedisService {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.process {
            if let Ok(redis_cli) = Self::find_cli_binary() {
                let mut cmd = Command::new(&redis_cli);
                cmd.args(["-h", "127.0.0.1", "-p", &self.port.to_string()]);

                if let Some(ref password) = self.password {
                    cmd.args(["-a", password]);
                }

                cmd.args(["SHUTDOWN", "NOSAVE"]);
                let _ = cmd.output();

                std::thread::sleep(Duration::from_millis(200));
            }

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGTERM);
            }
            #[cfg(not(unix))]
            let _ = child.kill();

            std::thread::sleep(Duration::from_millis(300));

            #[cfg(unix)]
            {
                let pid = Pid::from_raw(child.id() as i32);
                let _ = kill(pid, Signal::SIGKILL);
            }
            #[cfg(not(unix))]
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
